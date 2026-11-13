use crate::card::{CardFace, Color, Face, Seat, Side};
use crate::config::{ModeKind, Scoring};
use crate::events::{Action, EngineError, Phase};
use crate::state::GameState;
use crate::view::PlayerView;

pub const OBS_VERSION: u32 = 1;

const LIGHT_COLORS: [Color; 4] = [Color::Red, Color::Yellow, Color::Green, Color::Blue];
const DARK_COLORS: [Color; 4] = [Color::Pink, Color::Teal, Color::Orange, Color::Purple];

pub const fn face_count(mode: ModeKind) -> usize {
    match mode {
        ModeKind::Classic => 54,
        ModeKind::NoMercy => 68,
        ModeKind::Flip => 108,
    }
}

pub const fn action_size(mode: ModeKind) -> usize {
    face_count(mode) + 27
}

pub const fn obs_size(mode: ModeKind) -> usize {
    let k = face_count(mode);
    // 手牌(K) + 頂牌(K) + 目前顏色(4) + 階段(5) + 抽到牌(K) + 方向(1)
    // + 累積抽牌(1) + 對手(54) + 自己(3) + 棄牌(K) + 牌堆(1) + UNO視窗(10)
    let base = 4 * k + 79;
    match mode {
        ModeKind::Flip => base + 1 + 9 * k,
        ModeKind::Classic | ModeKind::NoMercy => base,
    }
}

pub(crate) fn face_index_on_side(mode: ModeKind, side: Side, cf: &CardFace) -> usize {
    match mode {
        ModeKind::Classic => match (cf.color, cf.face) {
            (Some(c), Face::Number(n)) if n <= 9 => usize::from(c.slot()) * 13 + usize::from(n),
            (Some(c), Face::Skip) => usize::from(c.slot()) * 13 + 10,
            (Some(c), Face::Reverse) => usize::from(c.slot()) * 13 + 11,
            (Some(c), Face::DrawTwo) => usize::from(c.slot()) * 13 + 12,
            (None, Face::Wild) => 52,
            (None, Face::WildDrawFour) => 53,
            _ => panic!("invalid CardFace for Classic mode: {cf:?}"),
        },
        ModeKind::NoMercy => match (cf.color, cf.face) {
            (Some(c), Face::Number(n)) if n <= 9 => usize::from(c.slot()) * 16 + usize::from(n),
            (Some(c), Face::DrawTwo) => usize::from(c.slot()) * 16 + 10,
            (Some(c), Face::DrawFour) => usize::from(c.slot()) * 16 + 11,
            (Some(c), Face::Skip) => usize::from(c.slot()) * 16 + 12,
            (Some(c), Face::SkipEveryone) => usize::from(c.slot()) * 16 + 13,
            (Some(c), Face::Reverse) => usize::from(c.slot()) * 16 + 14,
            (Some(c), Face::DiscardAll) => usize::from(c.slot()) * 16 + 15,
            (None, Face::WildReverseDrawFour) => 64,
            (None, Face::WildDrawSix) => 65,
            (None, Face::WildDrawTen) => 66,
            (None, Face::WildColorRoulette) => 67,
            _ => panic!("invalid CardFace for NoMercy mode: {cf:?}"),
        },
        ModeKind::Flip => match (cf.color, cf.face) {
            (Some(c), f)
                if matches!(c, Color::Red | Color::Yellow | Color::Green | Color::Blue) =>
            {
                let offset = match f {
                    Face::Number(n) if (1..=9).contains(&n) => usize::from(n - 1),
                    Face::DrawOne => 9,
                    Face::Reverse => 10,
                    Face::Skip => 11,
                    Face::Flip => 12,
                    _ => panic!("invalid Light CardFace for Flip mode: {cf:?}"),
                };
                usize::from(c.slot()) * 13 + offset
            }
            (Some(c), f)
                if matches!(c, Color::Pink | Color::Teal | Color::Orange | Color::Purple) =>
            {
                let offset = match f {
                    Face::Number(n) if (1..=9).contains(&n) => usize::from(n - 1),
                    Face::DrawFive => 9,
                    Face::Reverse => 10,
                    Face::SkipEveryone => 11,
                    Face::Flip => 12,
                    _ => panic!("invalid Dark CardFace for Flip mode: {cf:?}"),
                };
                54 + usize::from(c.slot()) * 13 + offset
            }
            (None, Face::WildDrawTwo) => 53,
            (None, Face::WildDrawColor) => 107,
            (None, Face::Wild) => match side {
                Side::Light => 52,
                Side::Dark => 106,
            },
            _ => panic!("invalid CardFace for Flip mode: {cf:?}"),
        },
    }
}

pub fn face_index(mode: ModeKind, cf: &CardFace) -> usize {
    face_index_on_side(mode, Side::Light, cf)
}

pub fn face_from_index(mode: ModeKind, idx: usize) -> CardFace {
    match mode {
        ModeKind::Classic => {
            assert!(idx < 54, "Classic face index out of range: {idx}");
            match idx {
                52 => CardFace::new(None, Face::Wild),
                53 => CardFace::new(None, Face::WildDrawFour),
                _ => {
                    let color = LIGHT_COLORS[idx / 13];
                    let face = match idx % 13 {
                        n @ 0..=9 => Face::Number(n as u8),
                        10 => Face::Skip,
                        11 => Face::Reverse,
                        _ => Face::DrawTwo,
                    };
                    CardFace::new(Some(color), face)
                }
            }
        }
        ModeKind::NoMercy => {
            assert!(idx < 68, "NoMercy face index out of range: {idx}");
            match idx {
                64 => CardFace::new(None, Face::WildReverseDrawFour),
                65 => CardFace::new(None, Face::WildDrawSix),
                66 => CardFace::new(None, Face::WildDrawTen),
                67 => CardFace::new(None, Face::WildColorRoulette),
                _ => {
                    let color = LIGHT_COLORS[idx / 16];
                    let face = match idx % 16 {
                        n @ 0..=9 => Face::Number(n as u8),
                        10 => Face::DrawTwo,
                        11 => Face::DrawFour,
                        12 => Face::Skip,
                        13 => Face::SkipEveryone,
                        14 => Face::Reverse,
                        _ => Face::DiscardAll,
                    };
                    CardFace::new(Some(color), face)
                }
            }
        }
        ModeKind::Flip => {
            assert!(idx < 108, "Flip face index out of range: {idx}");
            if idx < 54 {
                match idx {
                    52 => CardFace::new(None, Face::Wild),
                    53 => CardFace::new(None, Face::WildDrawTwo),
                    _ => {
                        let color = LIGHT_COLORS[idx / 13];
                        let face = match idx % 13 {
                            n @ 0..=8 => Face::Number((n + 1) as u8),
                            9 => Face::DrawOne,
                            10 => Face::Reverse,
                            11 => Face::Skip,
                            _ => Face::Flip,
                        };
                        CardFace::new(Some(color), face)
                    }
                }
            } else {
                let d_idx = idx - 54;
                match d_idx {
                    52 => CardFace::new(None, Face::Wild),
                    53 => CardFace::new(None, Face::WildDrawColor),
                    _ => {
                        let color = DARK_COLORS[d_idx / 13];
                        let face = match d_idx % 13 {
                            n @ 0..=8 => Face::Number((n + 1) as u8),
                            9 => Face::DrawFive,
                            10 => Face::Reverse,
                            11 => Face::SkipEveryone,
                            _ => Face::Flip,
                        };
                        CardFace::new(Some(color), face)
                    }
                }
            }
        }
    }
}

pub fn action_mask(state: &GameState, seat: Seat) -> Vec<bool> {
    let mode = state.config.mode;
    let a = action_size(mode);
    let mut mask = vec![false; a];
    for act in state.legal_actions(seat) {
        if let Some(idx) = action_to_index(state, seat, &act) {
            mask[idx] = true;
        }
    }
    mask
}

pub fn index_to_action(state: &GameState, seat: Seat, idx: usize) -> Result<Action, EngineError> {
    let mode = state.config.mode;
    let k = face_count(mode);
    let a = action_size(mode);
    if idx >= a || usize::from(seat) >= usize::from(state.config.num_players) {
        return Err(EngineError::IllegalAction);
    }

    let n = state.config.num_players;
    let legal = state.legal_actions(seat);

    let candidate = if idx < k {
        if mode == ModeKind::Flip {
            let valid_range = match state.side {
                Side::Light => idx < 54,
                Side::Dark => idx >= 54,
            };
            if !valid_range {
                return Err(EngineError::IllegalAction);
            }
        }
        let target_face = face_from_index(mode, idx);
        let best_cid = legal
            .iter()
            .filter_map(|act| match *act {
                Action::Play { card } if state.card(card).face(state.side) == target_face => {
                    Some(card)
                }
                _ => None,
            })
            .min();
        match best_cid {
            Some(card) => Action::Play { card },
            None => return Err(EngineError::IllegalAction),
        }
    } else {
        let rel = idx - k;
        match rel {
            0 => Action::Draw,
            1 => Action::Pass,
            2..=5 => {
                let slot = (rel - 2) as u8;
                let color = Color::from_slot(slot, state.side).ok_or(EngineError::IllegalAction)?;
                Action::ChooseColor { color }
            }
            6..=14 => {
                let r = (rel - 6 + 1) as u8;
                if r >= n {
                    return Err(EngineError::IllegalAction);
                }
                Action::ChooseTarget {
                    target: (seat + r) % n,
                }
            }
            15 => Action::CallUno,
            16 => Action::Challenge,
            17 => Action::Accept,
            18..=26 => {
                let r = (rel - 18 + 1) as u8;
                if r >= n {
                    return Err(EngineError::IllegalAction);
                }
                Action::CatchUno {
                    target: (seat + r) % n,
                }
            }
            _ => return Err(EngineError::IllegalAction),
        }
    };

    if legal.contains(&candidate) {
        Ok(candidate)
    } else {
        Err(EngineError::IllegalAction)
    }
}

pub fn action_to_index(state: &GameState, seat: Seat, a: &Action) -> Option<usize> {
    let mode = state.config.mode;
    let k = face_count(mode);
    let n = state.config.num_players;

    match *a {
        Action::Play { card } => {
            if usize::from(card) >= state.cards.len() {
                return None;
            }
            let cf = state.card(card).face(state.side);
            Some(face_index_on_side(mode, state.side, &cf))
        }
        Action::Draw => Some(k),
        Action::Pass => Some(k + 1),
        Action::ChooseColor { color } => Some(k + 2 + usize::from(color.slot())),
        Action::ChooseTarget { target } => {
            if target >= n || target == seat {
                return None;
            }
            let r = (target + n - seat) % n;
            Some(k + 6 + usize::from(r - 1))
        }
        Action::CallUno => Some(k + 15),
        Action::Challenge => Some(k + 16),
        Action::Accept => Some(k + 17),
        Action::CatchUno { target } => {
            if target >= n || target == seat {
                return None;
            }
            let r = (target + n - seat) % n;
            Some(k + 18 + usize::from(r - 1))
        }
        Action::JumpIn { .. } | Action::Timeout | Action::EndByTime => None,
    }
}

pub fn observe(state: &GameState, seat: Seat) -> Vec<f32> {
    let view = state.view(seat);
    observe_view(&view, seat)
}

pub fn observe_view(view: &PlayerView, seat: Seat) -> Vec<f32> {
    let mode = view.config.mode;
    let k = face_count(mode);
    let n = view.config.num_players;
    let deck_count = f32::from(view.config.deck_count).max(1.0);
    let single_deck_cards = match mode {
        ModeKind::Classic => 108usize,
        ModeKind::Flip => 112usize,
        ModeKind::NoMercy => 168usize,
    };
    let total_cards = (single_deck_cards * usize::from(view.config.deck_count)) as f32;

    let mut obs = Vec::with_capacity(obs_size(mode));

    // 1. 手牌 (K)
    let mut hand_counts = vec![0.0f32; k];
    for card in &view.my_hand {
        let idx = face_index_on_side(mode, view.side, &card.face(view.side));
        hand_counts[idx] += 1.0;
    }
    obs.extend_from_slice(&hand_counts);

    // 2. 頂牌 (K)
    let mut top_vec = vec![0.0f32; k];
    let top_idx = face_index_on_side(mode, view.side, &view.top_card.face(view.side));
    top_vec[top_idx] = 1.0;
    obs.extend_from_slice(&top_vec);

    // 3. 目前顏色 (4)
    let mut color_vec = [0.0f32; 4];
    if let Some(ac) = view.active_color {
        color_vec[usize::from(ac.slot())] = 1.0;
    }
    obs.extend_from_slice(&color_vec);

    // 4. 階段 (5): Play, ChooseColor, ChooseTarget, Challenge, DrawnCardDecision
    let mut phase_vec = [0.0f32; 5];
    match view.phase {
        Phase::Play => phase_vec[0] = 1.0,
        Phase::ChooseColor => phase_vec[1] = 1.0,
        Phase::ChooseTarget => phase_vec[2] = 1.0,
        Phase::Challenge => phase_vec[3] = 1.0,
        Phase::DrawnCardDecision => phase_vec[4] = 1.0,
        Phase::RoundOver | Phase::GameOver => {}
    }
    obs.extend_from_slice(&phase_vec);

    // 5. 抽到的牌 (K)
    let mut drawn_vec = vec![0.0f32; k];
    if view.phase == Phase::DrawnCardDecision {
        if let Some(cid) = view.drawn_card {
            if let Some(card) = view.my_hand.iter().find(|c| c.id == cid) {
                let idx = face_index_on_side(mode, view.side, &card.face(view.side));
                drawn_vec[idx] = 1.0;
            }
        }
    }
    obs.extend_from_slice(&drawn_vec);

    // 6. 方向 (1)
    obs.push(f32::from(view.direction));

    // 7. 累積抽牌 (1)
    obs.push(f32::from(view.pending_draw) / 10.0);

    // 8. 對手 (9 x 6)
    let my_team = view.players.get(usize::from(seat)).and_then(|p| p.team);
    let target_denom = match view.config.scoring {
        Scoring::Points { target } if !view.config.elimination && target > 0 => Some(target as f32),
        _ => None,
    };

    for r in 1..10u8 {
        if r < n {
            let opp_seat = (seat + r) % n;
            let opp = &view.players[usize::from(opp_seat)];
            obs.push(1.0);
            obs.push(f32::from(opp.hand_size) / 25.0);
            obs.push(if opp.uno_called { 1.0 } else { 0.0 });
            obs.push(if opp.eliminated { 1.0 } else { 0.0 });
            let is_teammate = my_team.is_some() && opp.team == my_team;
            obs.push(if is_teammate { 1.0 } else { 0.0 });
            obs.push(target_denom.map_or(0.0, |t| (opp.score as f32) / t));
        } else {
            obs.extend_from_slice(&[0.0; 6]);
        }
    }

    // 9. 自己 (3)
    if let Some(me_p) = view.players.get(usize::from(seat)) {
        obs.push(f32::from(me_p.hand_size) / 25.0);
        obs.push(if me_p.uno_called { 1.0 } else { 0.0 });
        obs.push(target_denom.map_or(0.0, |t| (me_p.score as f32) / t));
    } else {
        obs.extend_from_slice(&[0.0; 3]);
    }

    // 10. 已見棄牌 (K)
    let mut discard_counts = vec![0.0f32; k];
    for card in &view.discard {
        let idx = face_index_on_side(mode, view.side, &card.face(view.side));
        discard_counts[idx] += 1.0 / deck_count;
    }
    obs.extend_from_slice(&discard_counts);

    // 11. 牌堆 (1)
    obs.push(f32::from(view.draw_pile_size) / total_cards);

    // 12. UNO 視窗 (10)
    for r in 0..10u8 {
        let active = r < n && view.uno_window == Some((seat + r) % n);
        obs.push(if active { 1.0 } else { 0.0 });
    }

    // 13. Flip 專用 (1 + 9*K)
    if mode == ModeKind::Flip {
        obs.push(match view.side {
            Side::Light => 0.0,
            Side::Dark => 1.0,
        });
        let back_side = view.side.flip();
        for r in 1..10u8 {
            let mut back_counts = vec![0.0f32; k];
            if r < n {
                let opp_seat = (seat + r) % n;
                if let Some(backs) = &view.players[usize::from(opp_seat)].hand_backs {
                    for cf in backs {
                        let idx = face_index_on_side(mode, back_side, cf);
                        back_counts[idx] += 1.0;
                    }
                }
            }
            obs.extend_from_slice(&back_counts);
        }
    }

    debug_assert_eq!(obs.len(), obs_size(mode));
    obs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuleConfig;
    use crate::events::Phase;
    use crate::rng::Rng;

    #[test]
    fn encode_dimensions_match_spec() {
        assert_eq!(face_count(ModeKind::Classic), 54);
        assert_eq!(face_count(ModeKind::NoMercy), 68);
        assert_eq!(face_count(ModeKind::Flip), 108);

        assert_eq!(action_size(ModeKind::Classic), 81);
        assert_eq!(action_size(ModeKind::NoMercy), 95);
        assert_eq!(action_size(ModeKind::Flip), 135);

        assert_eq!(obs_size(ModeKind::Classic), 295);
        assert_eq!(OBS_VERSION, 1);
    }

    #[test]
    fn encode_face_table_round_trip() {
        for mode in [ModeKind::Classic, ModeKind::NoMercy, ModeKind::Flip] {
            let k = face_count(mode);
            for i in 0..k {
                let cf = face_from_index(mode, i);
                if mode == ModeKind::Flip {
                    let side = if i < 54 { Side::Light } else { Side::Dark };
                    assert_eq!(face_index_on_side(mode, side, &cf), i);
                } else {
                    assert_eq!(face_index(mode, &cf), i);
                }
            }
        }
    }

    #[test]
    fn encode_mask_legal_invariant_and_obs_bounds_across_all_modes() {
        for (mode_id, players) in [
            ("classic", 4u8),
            ("party", 5),
            ("flip", 4),
            ("no_mercy", 4),
            ("teams", 6),
            ("elimination", 4),
        ] {
            let mut cfg = RuleConfig::preset(mode_id, players).unwrap();
            cfg.jump_in = false;
            let mode = cfg.mode;
            let expected_obs_len = obs_size(mode);
            let expected_act_len = action_size(mode);

            for seed in 1u64..=3 {
                let mut state = GameState::new(cfg.clone(), seed * 17).unwrap();
                let mut rng = Rng::new(seed * 999);
                let mut steps = 0usize;

                while !state.is_over() && steps < 400 {
                    if state.phase == Phase::RoundOver {
                        state.next_round().unwrap();
                        continue;
                    }

                    for seat in 0..players {
                        let obs = observe(&state, seat);
                        assert_eq!(obs.len(), expected_obs_len);
                        for &v in &obs {
                            assert!(
                                v.is_finite() && (-1.0..=50.0).contains(&v),
                                "obs value {v} out of [-1, 50] in {mode_id}"
                            );
                        }

                        let mask = action_mask(&state, seat);
                        assert_eq!(mask.len(), expected_act_len);
                        let legal = state.legal_actions(seat);

                        // mask[i] == true <=> index_to_action(i) 成功且結果在 legal_actions 中
                        for (i, &allowed) in mask.iter().enumerate() {
                            match index_to_action(&state, seat, i) {
                                Ok(act) => {
                                    assert!(
                                        allowed,
                                        "index {i} succeeded with {act:?} but mask was false"
                                    );
                                    assert!(legal.contains(&act));
                                }
                                Err(_) => {
                                    assert!(
                                        !allowed,
                                        "index {i} had mask=true but index_to_action failed"
                                    );
                                }
                            }
                        }

                        // 每個非 JumpIn 合法動作皆對應某個 true 索引
                        for act in &legal {
                            if !matches!(act, Action::JumpIn { .. }) {
                                let idx = action_to_index(&state, seat, act)
                                    .expect("non-JumpIn legal action must map to an index");
                                assert!(
                                    mask[idx],
                                    "legal action {act:?} mapped to {idx} where mask was false"
                                );
                            }
                        }
                    }

                    let dp = state.decision_player().unwrap();
                    let legal_indices: Vec<usize> = action_mask(&state, dp)
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, b)| b.then_some(i))
                        .collect();
                    assert!(!legal_indices.is_empty());
                    let chosen_idx = legal_indices[(rng.next_u64() as usize) % legal_indices.len()];
                    let act = index_to_action(&state, dp, chosen_idx).unwrap();
                    state.apply(dp, act).unwrap();
                    steps += 1;
                }
            }
        }
    }
}
