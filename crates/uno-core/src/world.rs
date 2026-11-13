use std::collections::BTreeMap;

use crate::card::{Card, CardFace, CardId, Color, Face, Seat, Side};
use crate::config::ModeKind;
use crate::deck::build_deck;
use crate::encode::face_index_on_side;
use crate::events::{EngineError, Phase};
use crate::rng::Rng;
use crate::state::GameState;
use crate::view::PlayerView;

fn next_active_dir(eliminated: &[bool], from: Seat, steps: u8, dir: i8) -> Seat {
    let n = eliminated.len() as i16;
    let mut cur = i16::from(from);
    let step_dir = i16::from(dir);
    for _ in 0..steps {
        loop {
            cur = (cur + step_dir).rem_euclid(n);
            if !eliminated[cur as usize] || cur == i16::from(from) {
                break;
            }
        }
    }
    cur as Seat
}

fn fill_flip_hidden_buckets(
    unknown_ids: &[CardId],
    cards: &[Card],
    back_side: Side,
    rng: &mut Rng,
) -> BTreeMap<usize, Vec<CardId>> {
    let mut buckets: BTreeMap<usize, Vec<CardId>> = BTreeMap::new();
    for &cid in unknown_ids {
        let cf = cards[usize::from(cid)].face(back_side);
        let key = face_index_on_side(ModeKind::Flip, back_side, &cf);
        buckets.entry(key).or_default().push(cid);
    }
    for ids in buckets.values_mut() {
        rng.shuffle(ids);
    }
    buckets
}

fn pop_flip_back(
    buckets: &mut BTreeMap<usize, Vec<CardId>>,
    back_side: Side,
    back_face: &CardFace,
) -> Option<CardId> {
    let key = face_index_on_side(ModeKind::Flip, back_side, back_face);
    buckets.get_mut(&key).and_then(Vec::pop)
}

impl GameState {
    /// 依目前不完整資訊進行決定性的世界取樣（保留自己手牌與所有公開資訊，
    /// 未知牌重新洗牌分配；Flip 模式僅於背面相同的牌之間互換）。
    pub fn sample_world(&self, seat: Seat, rng: &mut Rng) -> GameState {
        let mut sampled = self.clone();
        sampled.rng = Rng::new(rng.next_u64());
        sampled.steps.clear();

        let n = usize::from(self.config.num_players);
        let viewer = usize::from(seat);

        let mut unknown: Vec<CardId> = self.draw_pile.clone();
        for (i, h) in self.hands.iter().enumerate() {
            if i != viewer {
                unknown.extend_from_slice(h);
            }
        }

        if self.config.mode != ModeKind::Flip {
            rng.shuffle(&mut unknown);
            let mut cursor = 0usize;
            for i in 0..n {
                if i == viewer {
                    continue;
                }
                let len = self.hands[i].len();
                sampled.hands[i] = unknown[cursor..cursor + len].to_vec();
                cursor += len;
            }
            sampled.draw_pile = unknown[cursor..].to_vec();
            return sampled;
        }

        // Flip 模式：在背面相同的牌之間重新分配
        let back_side = self.side.flip();
        let mut buckets = fill_flip_hidden_buckets(&unknown, &self.cards, back_side, rng);

        for i in 0..n {
            if i == viewer {
                continue;
            }
            let mut new_hand = Vec::with_capacity(self.hands[i].len());
            for &orig_cid in &self.hands[i] {
                let back_face = self.card(orig_cid).face(back_side);
                let picked = pop_flip_back(&mut buckets, back_side, &back_face)
                    .expect("bucket must contain matching back face");
                new_hand.push(picked);
            }
            sampled.hands[i] = new_hand;
        }

        let top_draw = self.draw_pile.last().map(|&orig_top| {
            let back_face = self.card(orig_top).face(back_side);
            pop_flip_back(&mut buckets, back_side, &back_face)
                .expect("bucket must contain matching top back face")
        });

        let mut rest_draw: Vec<CardId> = buckets.into_values().flatten().collect();
        rng.shuffle(&mut rest_draw);
        if let Some(top) = top_draw {
            rest_draw.push(top);
        }
        sampled.draw_pile = rest_draw;
        sampled
    }

    /// 僅依據 `PlayerView` 重建一個與公開視野完全一致的取樣 `GameState`（供 `HardBot` 在獨立客戶端/Worker 使用）。
    pub fn from_view_sampled(view: &PlayerView, rng: &mut Rng) -> Result<GameState, EngineError> {
        let mut config = view.config.clone();
        config.validate()?;

        let n = usize::from(config.num_players);
        if view.players.len() != n {
            return Err(EngineError::InvalidConfig(
                "PlayerView players len mismatch".to_string(),
            ));
        }

        let deck_rng_seed = rng.next_u64();
        let mut dummy_rng = Rng::new(deck_rng_seed);
        let mut cards = build_deck(&config, &mut dummy_rng);
        let total = cards.len();

        let mut known_used = vec![false; total];
        for c in view.my_hand.iter().chain(&view.discard) {
            let idx = usize::from(c.id);
            if idx >= total || known_used[idx] {
                return Err(EngineError::InvalidConfig(format!(
                    "duplicate or invalid CardId {} in PlayerView",
                    c.id
                )));
            }
            known_used[idx] = true;
            if config.mode == ModeKind::Flip {
                cards[idx] = *c;
            }
        }

        let unknown_ids: Vec<CardId> = (0..total as CardId)
            .filter(|&id| !known_used[usize::from(id)])
            .collect();

        if config.mode == ModeKind::Flip {
            // 對尚未見過的 Flip 卡牌ID：分別收集尚未出現的 Light 與 Dark 牌面，洗牌後重新配對
            let full_canonical = build_deck(&config, &mut Rng::new(0));
            let mut rem_lights = subtract_faces(
                full_canonical.iter().map(|c| c.light),
                view.my_hand.iter().chain(&view.discard).map(|c| c.light),
            );
            let mut rem_darks = subtract_faces(
                full_canonical.iter().filter_map(|c| c.dark),
                view.my_hand
                    .iter()
                    .chain(&view.discard)
                    .filter_map(|c| c.dark),
            );
            if rem_lights.len() != unknown_ids.len() || rem_darks.len() != unknown_ids.len() {
                return Err(EngineError::InvalidConfig(
                    "Flip PlayerView card counts inconsistent with deck".to_string(),
                ));
            }
            rng.shuffle(&mut rem_lights);
            rng.shuffle(&mut rem_darks);
            for (i, &cid) in unknown_ids.iter().enumerate() {
                cards[usize::from(cid)] = Card {
                    id: cid,
                    light: rem_lights[i],
                    dark: Some(rem_darks[i]),
                };
            }
        }

        let viewer = view.me.map(usize::from);
        let mut hands = vec![Vec::new(); n];
        if let Some(v_idx) = viewer {
            if v_idx >= n {
                return Err(EngineError::InvalidConfig(
                    "viewer seat out of range".to_string(),
                ));
            }
            hands[v_idx] = view.my_hand.iter().map(|c| c.id).collect();
        }

        let draw_pile = if config.mode != ModeKind::Flip {
            let mut pool = unknown_ids;
            rng.shuffle(&mut pool);
            let mut cursor = 0usize;
            for (i, p) in view.players.iter().enumerate() {
                if viewer == Some(i) {
                    continue;
                }
                let h_len = usize::from(p.hand_size);
                if cursor + h_len > pool.len() {
                    return Err(EngineError::InvalidConfig(
                        "PlayerView hand sizes exceed remaining deck".to_string(),
                    ));
                }
                hands[i] = pool[cursor..cursor + h_len].to_vec();
                cursor += h_len;
            }
            let rem = pool[cursor..].to_vec();
            if rem.len() != usize::from(view.draw_pile_size) {
                return Err(EngineError::InvalidConfig(
                    "PlayerView draw_pile_size inconsistent".to_string(),
                ));
            }
            rem
        } else {
            let back_side = view.side.flip();
            let mut buckets = fill_flip_hidden_buckets(&unknown_ids, &cards, back_side, rng);
            for (i, p) in view.players.iter().enumerate() {
                if viewer == Some(i) {
                    continue;
                }
                let backs = p.hand_backs.as_ref().ok_or_else(|| {
                    EngineError::InvalidConfig("Flip PlayerView missing hand_backs".to_string())
                })?;
                if backs.len() != usize::from(p.hand_size) {
                    return Err(EngineError::InvalidConfig(
                        "Flip PlayerView hand_backs len mismatch".to_string(),
                    ));
                }
                let mut h = Vec::with_capacity(backs.len());
                for cf in backs {
                    let cid = pop_flip_back(&mut buckets, back_side, cf).ok_or_else(|| {
                        EngineError::InvalidConfig(
                            "Flip PlayerView hand_backs inconsistent with remaining cards"
                                .to_string(),
                        )
                    })?;
                    h.push(cid);
                }
                hands[i] = h;
            }

            let top_draw = match view.draw_pile_top_back {
                Some(ref cf) => {
                    Some(pop_flip_back(&mut buckets, back_side, cf).ok_or_else(|| {
                        EngineError::InvalidConfig(
                            "Flip draw_pile_top_back inconsistent with remaining cards".to_string(),
                        )
                    })?)
                }
                None => None,
            };

            let mut pile: Vec<CardId> = buckets.into_values().flatten().collect();
            rng.shuffle(&mut pile);
            if let Some(top) = top_draw {
                pile.push(top);
            }
            if pile.len() != usize::from(view.draw_pile_size) {
                return Err(EngineError::InvalidConfig(
                    "Flip PlayerView draw_pile_size inconsistent".to_string(),
                ));
            }
            pile
        };

        let eliminated: Vec<bool> = view.players.iter().map(|p| p.eliminated).collect();
        let elimination_order: Vec<Seat> = view
            .players
            .iter()
            .filter(|p| p.eliminated)
            .map(|p| p.seat)
            .collect();
        let uno_called: Vec<bool> = view.players.iter().map(|p| p.uno_called).collect();
        let scores: Vec<i32> = view.players.iter().map(|p| p.score).collect();

        let challenge_offender = if view.phase == Phase::Challenge {
            Some(next_active_dir(
                &eliminated,
                view.current,
                1,
                -view.direction,
            ))
        } else {
            None
        };

        let wild4_was_legal = if let Some(offender) = challenge_offender {
            // 若能從棄牌堆前一張推斷出原先色，檢查取樣到的出牌者手牌是否無該色
            let prev_color = view
                .discard
                .len()
                .checked_sub(2)
                .and_then(|idx| view.discard[idx].face(view.side).color);
            match prev_color {
                Some(pc) => !hands[usize::from(offender)]
                    .iter()
                    .any(|&cid| cards[usize::from(cid)].face(view.side).color == Some(pc)),
                None => rng.below(2) == 0,
            }
        } else {
            false
        };

        // 若正處於起始牌選色階段則難以直接從 PlayerView 區分；不過選色完成僅由 view.current 選色
        let start_wild_pending = view.phase == Phase::ChooseColor
            && view.discard.len() == 1
            && view.discard[0].face(view.side).face == Face::Wild
            && view.turn == 0;
        let flip_just_flipped = false;

        Ok(GameState {
            config,
            seed: deck_rng_seed,
            rng: Rng::new(rng.next_u64()),
            cards,
            side: view.side,
            hands,
            draw_pile,
            discard: view.discard.iter().map(|c| c.id).collect(),
            current: view.current,
            direction: view.direction,
            active_color: view.active_color,
            pending_draw: view.pending_draw,
            phase: view.phase,
            uno_called,
            uno_window: view.uno_window,
            drawn_card: view.drawn_card,
            wild4_was_legal,
            challenge_offender,
            eliminated,
            elimination_order,
            scores,
            round: view.round,
            dealer: view.dealer,
            turn: view.turn,
            rankings: view.rankings.clone(),
            start_wild_pending,
            flip_just_flipped,
            initial_events: Vec::new(),
            steps: Vec::new(),
        })
    }
}

fn subtract_faces(
    full: impl Iterator<Item = CardFace>,
    known: impl Iterator<Item = CardFace>,
) -> Vec<CardFace> {
    let mut counts: BTreeMap<(Option<ColorKey>, FaceKey), usize> = BTreeMap::new();
    let mut templates: BTreeMap<(Option<ColorKey>, FaceKey), CardFace> = BTreeMap::new();
    for cf in full {
        let key = face_sort_key(&cf);
        *counts.entry(key).or_insert(0) += 1;
        templates.entry(key).or_insert(cf);
    }
    for cf in known {
        let key = face_sort_key(&cf);
        if let Some(c) = counts.get_mut(&key) {
            *c = c.saturating_sub(1);
        }
    }
    let mut out = Vec::new();
    for (key, cnt) in counts {
        let cf = templates[&key];
        for _ in 0..cnt {
            out.push(cf);
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ColorKey(u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FaceKey(u8, u8);

fn face_sort_key(cf: &CardFace) -> (Option<ColorKey>, FaceKey) {
    let c_key = cf.color.map(|c| match c {
        Color::Red => ColorKey(0),
        Color::Yellow => ColorKey(1),
        Color::Green => ColorKey(2),
        Color::Blue => ColorKey(3),
        Color::Pink => ColorKey(4),
        Color::Teal => ColorKey(5),
        Color::Orange => ColorKey(6),
        Color::Purple => ColorKey(7),
    });
    let f_key = match cf.face {
        Face::Number(n) => FaceKey(0, n),
        Face::Skip => FaceKey(1, 0),
        Face::Reverse => FaceKey(2, 0),
        Face::DrawTwo => FaceKey(3, 0),
        Face::Wild => FaceKey(4, 0),
        Face::WildDrawFour => FaceKey(5, 0),
        Face::DrawOne => FaceKey(6, 0),
        Face::Flip => FaceKey(7, 0),
        Face::WildDrawTwo => FaceKey(8, 0),
        Face::DrawFive => FaceKey(9, 0),
        Face::SkipEveryone => FaceKey(10, 0),
        Face::WildDrawColor => FaceKey(11, 0),
        Face::DrawFour => FaceKey(12, 0),
        Face::DiscardAll => FaceKey(13, 0),
        Face::WildReverseDrawFour => FaceKey(14, 0),
        Face::WildDrawSix => FaceKey(15, 0),
        Face::WildDrawTen => FaceKey(16, 0),
        Face::WildColorRoulette => FaceKey(17, 0),
    };
    (c_key, f_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuleConfig;
    use crate::events::Action;

    fn play_some_steps(mode_id: &str, seed: u64, steps: usize) -> GameState {
        let cfg = RuleConfig::preset(mode_id, 4).unwrap();
        let mut s = GameState::new(cfg, seed).unwrap();
        let mut rng = Rng::new(seed ^ 0xCAFE_BABE);
        for _ in 0..steps {
            if s.is_over() || s.phase() == Phase::RoundOver {
                break;
            }
            let dp = s.decision_player().unwrap();
            let acts = s.legal_actions(dp);
            let act = acts[(rng.next_u64() as usize) % acts.len()];
            s.apply(dp, act).unwrap();
        }
        s
    }

    #[test]
    fn world_sample_and_from_view_sampled_preserve_public_fields_and_card_conservation() {
        for mode_id in ["classic", "flip", "no_mercy"] {
            let state = play_some_steps(mode_id, 777, 15);
            let viewer = state.decision_player().unwrap_or(0);
            let orig_view = state.view(viewer);
            let expected_total = match state.config.mode {
                ModeKind::Classic => 108,
                ModeKind::Flip => 112,
                ModeKind::NoMercy => 168,
            };

            let mut rng1 = Rng::new(42);
            let w1 = state.sample_world(viewer, &mut rng1);
            let w2 = GameState::from_view_sampled(&orig_view, &mut Rng::new(42)).unwrap();

            for world in [&w1, &w2] {
                let g = world.god_view();
                let mut seen = vec![false; expected_total];
                let mut count = 0usize;
                for c in g
                    .hands
                    .iter()
                    .flatten()
                    .chain(&g.draw_pile)
                    .chain(&g.discard)
                {
                    let idx = usize::from(c.id);
                    assert!(!seen[idx]);
                    seen[idx] = true;
                    count += 1;
                }
                assert_eq!(count, expected_total);

                let sv = world.view(viewer);
                assert_eq!(sv.me, orig_view.me);
                assert_eq!(sv.phase, orig_view.phase);
                assert_eq!(sv.current, orig_view.current);
                assert_eq!(sv.direction, orig_view.direction);
                assert_eq!(sv.side, orig_view.side);
                assert_eq!(sv.top_card, orig_view.top_card);
                assert_eq!(sv.active_color, orig_view.active_color);
                assert_eq!(sv.pending_draw, orig_view.pending_draw);
                assert_eq!(sv.draw_pile_size, orig_view.draw_pile_size);
                assert_eq!(sv.draw_pile_top_back, orig_view.draw_pile_top_back);
                assert_eq!(sv.discard, orig_view.discard);
                assert_eq!(sv.my_hand, orig_view.my_hand);
                assert_eq!(sv.drawn_card, orig_view.drawn_card);
                assert_eq!(sv.players, orig_view.players);
                assert_eq!(sv.uno_window, orig_view.uno_window);
                assert_eq!(sv.legal, orig_view.legal);
            }

            // 且可在 sampled 世界繼續推進遊戲到結束不 panic
            for mut world in [w1, w2] {
                let mut roll_rng = Rng::new(555);
                for _ in 0..300 {
                    if world.is_over() || world.phase() == Phase::RoundOver {
                        break;
                    }
                    let dp = world.decision_player().unwrap();
                    let acts = world.legal_actions(dp);
                    let act = acts[(roll_rng.next_u64() as usize) % acts.len()];
                    world.apply(dp, act).unwrap();
                }
                if !world.is_over() && world.phase() != Phase::RoundOver {
                    let _ = world.apply(0, Action::EndByTime);
                }
            }
        }
    }
}
