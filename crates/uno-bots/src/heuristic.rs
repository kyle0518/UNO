use uno_core::{
    Action, CardFace, Color, Face, ModeKind, PlayerView, PublicPlayer, RuleConfig, Seat, Side,
};

use crate::Bot;

/// 依據經驗法則評估盤面的 Bot（支援 Classic、Flip、No Mercy、Teams、Elimination）。
#[derive(Debug, Clone, Default)]
pub struct HeuristicBot;

impl HeuristicBot {
    /// 建立 `HeuristicBot`。
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Bot for HeuristicBot {
    fn name(&self) -> &'static str {
        "heuristic"
    }

    fn act(&mut self, state_view: &PlayerView) -> Action {
        choose_heuristic_action(state_view)
    }
}

/// 無狀態啟發式決策（同時供 `HeuristicBot` 與 ISMCTS rollout 使用）。
#[must_use]
pub fn choose_heuristic_action(view: &PlayerView) -> Action {
    // 1. CallUno 合法 → 優先喊出
    if view.legal.contains(&Action::CallUno) {
        return Action::CallUno;
    }

    // 2. CatchUno 合法 → 優先抓手牌最少者
    if let Some(catch) = best_catch_uno(view) {
        return catch;
    }

    // 3. Challenge 階段（+4 / WildDrawTwo / WildDrawColor 質疑判斷）
    if view.legal.contains(&Action::Challenge) && view.legal.contains(&Action::Accept) {
        return choose_challenge_action(view);
    }

    // 4. ChooseColor：手上張數最多的顏色槽
    if view
        .legal
        .iter()
        .any(|a| matches!(a, Action::ChooseColor { .. }))
    {
        if let Some(best_color) = best_choose_color(view) {
            return best_color;
        }
    }

    // 5. ChooseTarget（如 7 換牌 / NoMercy Roulette）：選對手之中手牌最少者
    if view
        .legal
        .iter()
        .any(|a| matches!(a, Action::ChooseTarget { .. }))
    {
        if let Some(best_target) = best_choose_target(view) {
            return best_target;
        }
    }

    // 6. DrawnCardDecision：剛抽到的牌能出就出
    if matches!(view.phase, uno_core::Phase::DrawnCardDecision) {
        if let Some(&play) = view.legal.iter().find(|a| matches!(a, Action::Play { .. })) {
            return play;
        }
        if view.legal.contains(&Action::Pass) {
            return Action::Pass;
        }
    }

    // 7. Play 階段（或可搶出時的 JumpIn）依分數挑選最高分出牌；否則抽牌
    let dominant = dominant_color(view);
    let next_threat = is_next_opponent_low_hand(view);

    let mut best_action: Option<Action> = None;
    let mut best_score = i32::MIN;

    for &action in &view.legal {
        let card_id = match action {
            Action::Play { card } | Action::JumpIn { card } => card,
            _ => continue,
        };
        let Some(card) = view.my_hand.iter().find(|c| c.id == card_id) else {
            continue;
        };
        let face = card.face(view.side);
        let score = score_card(&face, dominant, next_threat, view.my_hand.len());
        if score > best_score {
            best_score = score;
            best_action = Some(action);
        }
    }

    if let Some(action) = best_action {
        return action;
    }

    if view.legal.contains(&Action::Draw) {
        return Action::Draw;
    }

    view.legal.first().copied().unwrap_or(Action::Draw)
}

fn best_catch_uno(view: &PlayerView) -> Option<Action> {
    view.legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::CatchUno { target } => {
                let size = player_by_seat(view, target)
                    .map(|p| p.hand_size)
                    .unwrap_or(u8::MAX);
                Some((size, target, a))
            }
            _ => None,
        })
        .min_by_key(|&(size, target, _)| (size, target))
        .map(|(_, _, a)| a)
}

fn choose_challenge_action(view: &PlayerView) -> Action {
    let offender_hand = challenge_offender_seat(view)
        .and_then(|seat| player_by_seat(view, seat))
        .map(|p| p.hand_size)
        .unwrap_or(0);

    if offender_hand < 4 {
        return Action::Accept;
    }

    let Some(prev_color) = color_before_top_card(view) else {
        return Action::Accept;
    };

    let history_slice = if view.discard.len() > 1 {
        &view.discard[..view.discard.len() - 1]
    } else {
        &[]
    };
    let seen_in_discard = history_slice
        .iter()
        .filter(|c| c.face(view.side).color == Some(prev_color))
        .count();
    let total_color_cards = total_cards_of_color(&view.config, view.side);

    if seen_in_discard * 10 < total_color_cards * 3 {
        Action::Challenge
    } else {
        Action::Accept
    }
}

fn challenge_offender_seat(view: &PlayerView) -> Option<Seat> {
    let n = view.players.len();
    if n <= 1 {
        return None;
    }
    let me = view.decision_player.or(view.me)?;
    let step = if view.direction >= 0 { -1_i32 } else { 1_i32 };
    let mut idx = i32::from(me);
    for _ in 0..n {
        idx = (idx + step).rem_euclid(n as i32);
        let seat = idx as Seat;
        if let Some(p) = player_by_seat(view, seat) {
            if !p.eliminated && seat != me {
                return Some(seat);
            }
        }
    }
    None
}

fn color_before_top_card(view: &PlayerView) -> Option<Color> {
    if view.discard.len() < 2 {
        return None;
    }
    view.discard[..view.discard.len() - 1]
        .iter()
        .rev()
        .find_map(|c| c.face(view.side).color)
}

fn total_cards_of_color(config: &RuleConfig, _side: Side) -> usize {
    let per_deck = match config.mode {
        ModeKind::Flip => 26_usize,
        ModeKind::NoMercy => 36_usize,
        ModeKind::Classic => 25_usize,
    };
    per_deck * usize::from(config.deck_count.max(1))
}

fn best_choose_color(view: &PlayerView) -> Option<Action> {
    let counts = color_counts_in_hand(view);
    view.legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::ChooseColor { color } => {
                let count = counts[usize::from(color.slot())];
                Some((count, a))
            }
            _ => None,
        })
        .max_by_key(|&(count, _)| count)
        .map(|(_, a)| a)
}

fn best_choose_target(view: &PlayerView) -> Option<Action> {
    let my_team = view.me.and_then(|me| team_of(view, me));
    view.legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::ChooseTarget { target } => {
                let player = player_by_seat(view, target);
                let is_teammate = my_team.is_some() && player.and_then(|p| p.team) == my_team;
                let hand_size = player.map(|p| p.hand_size).unwrap_or(u8::MAX);
                Some((is_teammate, hand_size, target, a))
            }
            _ => None,
        })
        .min_by_key(|&(is_teammate, hand_size, target, _)| (is_teammate, hand_size, target))
        .map(|(_, _, _, a)| a)
}

fn dominant_color(view: &PlayerView) -> Option<Color> {
    let counts = color_counts_in_hand(view);
    let colors = Color::colors_for_side(view.side);
    let mut best_color = None;
    let mut best_count = 0_usize;
    for color in colors {
        let count = counts[usize::from(color.slot())];
        if count > best_count {
            best_count = count;
            best_color = Some(color);
        }
    }
    best_color
}

fn color_counts_in_hand(view: &PlayerView) -> [usize; 4] {
    let mut counts = [0_usize; 4];
    for card in &view.my_hand {
        if let Some(color) = card.face(view.side).color {
            counts[usize::from(color.slot())] += 1;
        }
    }
    counts
}

fn is_next_opponent_low_hand(view: &PlayerView) -> bool {
    let Some(me) = view.decision_player.or(view.me) else {
        return false;
    };
    let my_team = team_of(view, me);
    let Some(next) = next_active_player(view, me) else {
        return false;
    };
    let is_opponent = my_team.is_none() || next.team != my_team;
    is_opponent && next.hand_size <= 2
}

fn next_active_player(view: &PlayerView, from: Seat) -> Option<&PublicPlayer> {
    let n = view.players.len();
    if n <= 1 {
        return None;
    }
    let step = if view.direction >= 0 { 1_i32 } else { -1_i32 };
    let mut idx = i32::from(from);
    for _ in 0..n {
        idx = (idx + step).rem_euclid(n as i32);
        let seat = idx as Seat;
        if seat != from {
            if let Some(p) = player_by_seat(view, seat) {
                if !p.eliminated {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn player_by_seat(view: &PlayerView, seat: Seat) -> Option<&PublicPlayer> {
    view.players
        .get(usize::from(seat))
        .filter(|p| p.seat == seat)
        .or_else(|| view.players.iter().find(|p| p.seat == seat))
}

fn team_of(view: &PlayerView, seat: Seat) -> Option<u8> {
    player_by_seat(view, seat).and_then(|p| p.team)
}

fn score_card(face: &CardFace, dominant: Option<Color>, next_threat: bool, hand_len: usize) -> i32 {
    let mut score = 0_i32;

    if next_threat && is_disruptive_action(face.face) {
        score += 50;
    }

    if let Face::Number(value) = face.face {
        score += i32::from(value);
    }

    if let (Some(card_color), Some(dom_color)) = (face.color, dominant) {
        if card_color == dom_color {
            score += 10;
        }
    }

    if face.is_wild() {
        if hand_len <= 2 {
            score += 40;
        } else {
            score -= 30;
        }
    }

    score
}

fn is_disruptive_action(face: Face) -> bool {
    matches!(
        face,
        Face::Skip
            | Face::SkipEveryone
            | Face::Reverse
            | Face::DrawOne
            | Face::DrawTwo
            | Face::DrawFour
            | Face::DrawFive
            | Face::WildDrawTwo
            | Face::WildDrawFour
            | Face::WildReverseDrawFour
            | Face::WildDrawSix
            | Face::WildDrawTen
            | Face::WildDrawColor
            | Face::WildColorRoulette
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uno_core::{Card, CardFace, Color, Face, GameState, Phase, RuleConfig};

    fn make_card(id: u16, color: Option<Color>, face: Face) -> Card {
        let cf = CardFace::new(color, face);
        Card {
            id,
            light: cf,
            dark: None,
        }
    }

    #[test]
    fn heuristic_prefers_dominant_color_highest_number_and_saves_wild_early() {
        let cfg = RuleConfig::preset("classic", 4).expect("preset");
        let state = GameState::new(cfg, 7).expect("state");
        let mut view = state.view(0);
        view.my_hand = vec![
            make_card(1, Some(Color::Red), Face::Number(2)),
            make_card(2, Some(Color::Red), Face::Number(8)),
            make_card(3, Some(Color::Blue), Face::Number(9)),
            make_card(4, None, Face::Wild),
        ];
        view.legal = vec![
            Action::Play { card: 4 },
            Action::Play { card: 3 },
            Action::Play { card: 2 },
            Action::Play { card: 1 },
            Action::Draw,
        ];

        // Red 是最多色 (2張)，Red 8 得 8+10=18；Blue 9 得 9；Wild 得 -30
        assert_eq!(choose_heuristic_action(&view), Action::Play { card: 2 });

        // 手牌僅剩 <= 2 張時，Wild (+40) 優先打出
        view.my_hand = vec![
            make_card(1, Some(Color::Red), Face::Number(2)),
            make_card(4, None, Face::Wild),
        ];
        view.legal = vec![Action::Play { card: 1 }, Action::Play { card: 4 }];
        assert_eq!(choose_heuristic_action(&view), Action::Play { card: 4 });
    }

    #[test]
    fn heuristic_blocks_low_hand_next_player_and_picks_dominant_color_on_choose() {
        let cfg = RuleConfig::preset("classic", 4).expect("preset");
        let state = GameState::new(cfg, 9).expect("state");
        let mut view = state.view(0);
        view.decision_player = Some(0);
        view.direction = 1;
        view.players[1].hand_size = 2;
        view.my_hand = vec![
            make_card(1, Some(Color::Green), Face::Number(9)),
            make_card(2, Some(Color::Green), Face::Skip),
            make_card(3, Some(Color::Yellow), Face::Number(1)),
        ];
        view.legal = vec![
            Action::Play { card: 1 },
            Action::Play { card: 2 },
            Action::Draw,
        ];
        // Skip (+50+10=60) > Number(9) (+9+10=19)
        assert_eq!(choose_heuristic_action(&view), Action::Play { card: 2 });

        // ChooseColor 選 Green
        view.phase = Phase::ChooseColor;
        view.legal = vec![
            Action::ChooseColor { color: Color::Red },
            Action::ChooseColor {
                color: Color::Green,
            },
            Action::ChooseColor {
                color: Color::Yellow,
            },
        ];
        assert_eq!(
            choose_heuristic_action(&view),
            Action::ChooseColor {
                color: Color::Green
            }
        );
    }

    #[test]
    fn heuristic_choose_target_and_challenge_and_drawn_decision() {
        let mut cfg = RuleConfig::preset("classic", 4).expect("preset");
        cfg.seven_zero = true;
        let state = GameState::new(cfg, 12).expect("state");
        let mut view = state.view(0);

        // ChooseTarget 選最少手牌的對手
        view.phase = Phase::ChooseTarget;
        view.players[1].hand_size = 5;
        view.players[2].hand_size = 1;
        view.players[3].hand_size = 4;
        view.legal = vec![
            Action::ChooseTarget { target: 1 },
            Action::ChooseTarget { target: 2 },
            Action::ChooseTarget { target: 3 },
        ];
        assert_eq!(
            choose_heuristic_action(&view),
            Action::ChooseTarget { target: 2 }
        );

        // DrawnCardDecision 能出就出
        view.phase = Phase::DrawnCardDecision;
        view.legal = vec![Action::Pass, Action::Play { card: 55 }];
        assert_eq!(choose_heuristic_action(&view), Action::Play { card: 55 });

        // Challenge: 出牌者手牌 >= 4 且上色牌出現極少 → Challenge
        view.phase = Phase::Challenge;
        view.decision_player = Some(1);
        view.direction = 1;
        view.players[0].hand_size = 5;
        view.discard = vec![
            make_card(10, Some(Color::Red), Face::Number(5)),
            make_card(11, None, Face::WildDrawFour),
        ];
        view.legal = vec![Action::Challenge, Action::Accept];
        assert_eq!(choose_heuristic_action(&view), Action::Challenge);

        // 出牌者手牌 < 4 → Accept
        view.players[0].hand_size = 3;
        assert_eq!(choose_heuristic_action(&view), Action::Accept);
    }
}
