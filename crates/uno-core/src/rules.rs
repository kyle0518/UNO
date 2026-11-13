use crate::card::{CardFace, Color, Face, Seat};
use crate::config::{StackDraw, UnoRule, Wild4Rule};
use crate::events::{Action, Phase};
use crate::state::GameState;

impl GameState {
    pub(crate) fn team_of(&self, seat: Seat) -> Option<u8> {
        self.config.teams.map(|t| seat % t)
    }

    pub(crate) fn same_team(&self, a: Seat, b: Seat) -> bool {
        match (self.team_of(a), self.team_of(b)) {
            (Some(ta), Some(tb)) => ta == tb,
            _ => false,
        }
    }

    pub(crate) fn has_active_color_in_hand(&self, seat: Seat) -> bool {
        let Some(ac) = self.active_color else {
            return false;
        };
        self.hands[usize::from(seat)]
            .iter()
            .any(|&cid| self.card(cid).face(self.side).color == Some(ac))
    }

    /// 判斷指定玩家目前是否可打出這張牌面（RULES §2.3，並預留疊加分支）。
    pub(crate) fn can_play(&self, seat: Seat, face: &CardFace) -> bool {
        if self.pending_draw > 0 {
            return self.can_stack(face);
        }

        match face.face {
            Face::Wild => true,
            Face::WildDrawFour | Face::WildDrawTwo | Face::WildDrawColor => {
                match self.config.wild4_rule {
                    Wild4Rule::Challenge | Wild4Rule::Free => true,
                    Wild4Rule::Strict => !self.has_active_color_in_hand(seat),
                }
            }
            Face::WildReverseDrawFour
            | Face::WildDrawSix
            | Face::WildDrawTen
            | Face::WildColorRoulette => true,
            _ => {
                let top_face = self.top_card().face(self.side);
                if let (Some(c), Some(ac)) = (face.color, self.active_color) {
                    if c == ac {
                        return true;
                    }
                }
                face.face == top_face.face
            }
        }
    }

    /// 疊加狀態下是否能疊出（將於 T2.1 完整啟用，此處先支援基本介面）。
    pub(crate) fn can_stack(&self, face: &CardFace) -> bool {
        let top_face = self.top_card().face(self.side);
        let Some(card_val) = face.draw_value() else {
            return false;
        };
        let Some(top_val) = top_face.draw_value() else {
            return false;
        };
        match self.config.stack_draw {
            StackDraw::None => false,
            StackDraw::Same => face.face == top_face.face,
            StackDraw::Progressive => card_val >= top_val,
            StackDraw::NoMercy => card_val >= top_val,
        }
    }

    /// 回傳 `seat` 目前所有合法動作（不含系統動作 Timeout/EndByTime）。
    pub fn legal_actions(&self, seat: Seat) -> Vec<Action> {
        if usize::from(seat) >= usize::from(self.config.num_players)
            || self.eliminated[usize::from(seat)]
            || matches!(self.phase, Phase::RoundOver | Phase::GameOver)
        {
            return Vec::new();
        }

        let mut actions = Vec::new();
        let is_decision = self.decision_player() == Some(seat);

        if is_decision {
            match self.phase {
                Phase::Play => {
                    for &cid in &self.hands[usize::from(seat)] {
                        let face = self.card(cid).face(self.side);
                        if self.can_play(seat, &face) {
                            actions.push(Action::Play { card: cid });
                        }
                    }
                    let has_play = !actions.is_empty();
                    if self.pending_draw > 0 || self.available_draw_count() > 0 {
                        actions.push(Action::Draw);
                    } else if !has_play {
                        actions.push(Action::Pass);
                    }

                    if self.config.uno_rule == UnoRule::Manual
                        && !self.uno_called[usize::from(seat)]
                        && self.hands[usize::from(seat)].len() == 2
                        && has_play
                    {
                        actions.push(Action::CallUno);
                    }
                }
                Phase::ChooseColor => {
                    for c in Color::colors_for_side(self.side) {
                        actions.push(Action::ChooseColor { color: c });
                    }
                }
                Phase::ChooseTarget => {
                    for t in 0..self.config.num_players {
                        if t != seat && !self.eliminated[usize::from(t)] {
                            actions.push(Action::ChooseTarget { target: t });
                        }
                    }
                }
                Phase::Challenge => {
                    actions.push(Action::Challenge);
                    actions.push(Action::Accept);
                }
                Phase::DrawnCardDecision => {
                    if let Some(drawn) = self.drawn_card {
                        actions.push(Action::Play { card: drawn });
                    }
                    if !self.config.force_play {
                        actions.push(Action::Pass);
                    }
                    if self.config.uno_rule == UnoRule::Manual
                        && !self.uno_called[usize::from(seat)]
                        && self.hands[usize::from(seat)].len() == 2
                    {
                        actions.push(Action::CallUno);
                    }
                }
                Phase::RoundOver | Phase::GameOver => {}
            }
        } else if self.config.jump_in && self.phase == Phase::Play && self.pending_draw == 0 {
            let top_face = self.top_card().face(self.side);
            if top_face.color.is_some() {
                for &cid in &self.hands[usize::from(seat)] {
                    let face = self.card(cid).face(self.side);
                    if face.color.is_some() && face == top_face {
                        actions.push(Action::JumpIn { card: cid });
                    }
                }
            }
        }

        // UNO 視窗（任何未結束階段皆有效）
        if self.config.uno_rule == UnoRule::Manual {
            if let Some(target) = self.uno_window {
                if target == seat {
                    if !actions.contains(&Action::CallUno) {
                        actions.push(Action::CallUno);
                    }
                } else if !self.same_team(seat, target) {
                    actions.push(Action::CatchUno { target });
                }
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardId};
    use crate::config::RuleConfig;
    use crate::deck::build_deck;
    use crate::rng::Rng;

    fn make_deck_for_test(
        cfg: &RuleConfig,
        seat1_hand: &[CardId],
        start_card: CardId,
    ) -> Vec<CardId> {
        let cards = build_deck(cfg, &mut Rng::new(0));
        let mut used = vec![false; cards.len()];
        for &id in seat1_hand {
            used[usize::from(id)] = true;
        }
        used[usize::from(start_card)] = true;

        let mut remaining: Vec<CardId> = cards
            .iter()
            .map(|c| c.id)
            .filter(|&id| !used[usize::from(id)])
            .collect();

        // 發牌順序從 seat 1 開始（dealer = 0），每輪發 1 張給 1,2,3,0
        let n = usize::from(cfg.num_players);
        let h = usize::from(cfg.hand_size);
        let mut order = Vec::with_capacity(cards.len());
        for &s1_card in seat1_hand.iter().take(h) {
            for offset in 0..n {
                let seat = (1 + offset) % n;
                if seat == 1 {
                    order.push(s1_card);
                } else {
                    order.push(remaining.pop().unwrap());
                }
            }
        }
        order.push(start_card);
        order.extend(remaining);
        order
    }

    fn find_cards(cards: &[Card], color: Option<Color>, face: Face) -> Vec<CardId> {
        cards
            .iter()
            .filter(|c| c.light.color == color && c.light.face == face)
            .map(|c| c.id)
            .collect()
    }

    #[test]
    fn r2_3_play_phase_legal_actions_and_strict_wild4() {
        let mut cfg = RuleConfig::preset("classic", 4).unwrap();
        cfg.hand_size = 3;
        cfg.wild4_rule = Wild4Rule::Strict;
        let cards = build_deck(&cfg, &mut Rng::new(0));
        let r5 = find_cards(&cards, Some(Color::Red), Face::Number(5))[0];
        let b5 = find_cards(&cards, Some(Color::Blue), Face::Number(5))[0];
        let b7 = find_cards(&cards, Some(Color::Blue), Face::Number(7))[0];
        let w4 = find_cards(&cards, None, Face::WildDrawFour)[0];
        let start_r2 = find_cards(&cards, Some(Color::Red), Face::Number(2))[0];

        // 手上有 Red 5 時，Strict 下不可出 Wild4
        let order = make_deck_for_test(&cfg, &[r5, b5, w4], start_r2);
        let state = GameState::new_with_deck(cfg.clone(), order).unwrap();
        let legal = state.legal_actions(1);
        assert!(legal.contains(&Action::Play { card: r5 }));
        assert!(!legal.contains(&Action::Play { card: b5 }));
        assert!(!legal.contains(&Action::Play { card: w4 }));
        assert!(legal.contains(&Action::Draw));

        // 手上沒有 Red 時，Strict 下可出 Wild4
        let order2 = make_deck_for_test(&cfg, &[b7, b5, w4], start_r2);
        let state2 = GameState::new_with_deck(cfg, order2).unwrap();
        let legal2 = state2.legal_actions(1);
        assert!(legal2.contains(&Action::Play { card: w4 }));
        assert!(legal2.contains(&Action::Draw));
        assert!(state2.legal_actions(2).is_empty());
    }

    #[test]
    fn r2_5_6_pass_only_when_no_playable_and_no_draw() {
        let mut cfg = RuleConfig::preset("classic", 2).unwrap();
        cfg.hand_size = 2;
        let cards = build_deck(&cfg, &mut Rng::new(0));
        let b7 = find_cards(&cards, Some(Color::Blue), Face::Number(7))[0];
        let g8 = find_cards(&cards, Some(Color::Green), Face::Number(8))[0];
        let start_r2 = find_cards(&cards, Some(Color::Red), Face::Number(2))[0];
        let order = make_deck_for_test(&cfg, &[b7, g8], start_r2);
        let mut state = GameState::new_with_deck(cfg, order).unwrap();
        state.draw_pile.clear();
        let legal = state.legal_actions(1);
        assert_eq!(legal, vec![Action::Pass]);
    }

    #[test]
    fn phase_specific_legal_actions_choose_color_target_challenge_drawn() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let mut state = GameState::new(cfg, 1).unwrap();

        state.phase = Phase::ChooseColor;
        state.current = 2;
        assert_eq!(
            state.legal_actions(2),
            vec![
                Action::ChooseColor { color: Color::Red },
                Action::ChooseColor {
                    color: Color::Yellow
                },
                Action::ChooseColor {
                    color: Color::Green
                },
                Action::ChooseColor { color: Color::Blue },
            ]
        );

        state.phase = Phase::ChooseTarget;
        assert_eq!(
            state.legal_actions(2),
            vec![
                Action::ChooseTarget { target: 0 },
                Action::ChooseTarget { target: 1 },
                Action::ChooseTarget { target: 3 },
            ]
        );

        state.phase = Phase::Challenge;
        assert_eq!(
            state.legal_actions(2),
            vec![Action::Challenge, Action::Accept]
        );

        state.phase = Phase::DrawnCardDecision;
        let cid = state.hands[2][0];
        state.drawn_card = Some(cid);
        assert_eq!(
            state.legal_actions(2),
            vec![Action::Play { card: cid }, Action::Pass]
        );

        state.config.force_play = true;
        assert_eq!(state.legal_actions(2), vec![Action::Play { card: cid }]);
    }

    #[test]
    fn r2_7_1_uno_window_and_precall_actions() {
        let mut cfg = RuleConfig::preset("classic", 3).unwrap();
        cfg.hand_size = 2;
        let cards = build_deck(&cfg, &mut Rng::new(0));
        let r5 = find_cards(&cards, Some(Color::Red), Face::Number(5))[0];
        let b7 = find_cards(&cards, Some(Color::Blue), Face::Number(7))[0];
        let start_r2 = find_cards(&cards, Some(Color::Red), Face::Number(2))[0];
        let order = make_deck_for_test(&cfg, &[r5, b7], start_r2);
        let mut state = GameState::new_with_deck(cfg, order).unwrap();

        assert!(state.legal_actions(1).contains(&Action::CallUno));
        state.uno_window = Some(2);
        assert!(state.legal_actions(2).contains(&Action::CallUno));
        assert!(state
            .legal_actions(1)
            .contains(&Action::CatchUno { target: 2 }));
        assert!(state
            .legal_actions(0)
            .contains(&Action::CatchUno { target: 2 }));
    }
}
