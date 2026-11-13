use proptest::prelude::{prop_assert, proptest, ProptestConfig};
use uno_core::{
    Action, EngineError, GameState, ModeKind, Phase, Rng, RuleConfig, Scoring, Seat, StackDraw,
    UnoRule, Wild4Rule,
};

const ALL_MODES: [&str; 8] = [
    "classic",
    "party",
    "flip",
    "no_mercy",
    "teams",
    "elimination",
    "blitz",
    "daily",
];

fn expected_total_cards(cfg: &RuleConfig) -> usize {
    let base = match cfg.mode {
        ModeKind::Classic => 108,
        ModeKind::Flip => 112,
        ModeKind::NoMercy => 168,
    };
    base * usize::from(cfg.deck_count)
}

fn preset_player_count(mode_id: &str, raw: u8) -> u8 {
    match mode_id {
        "no_mercy" => 2 + (raw % 5), // 2..=6
        "teams" => [4, 6][usize::from(raw % 2)],
        "elimination" => 3 + (raw % 8), // 3..=10
        "daily" => 4,
        _ => 2 + (raw % 9), // 2..=10
    }
}

fn assert_card_conservation(state: &GameState, expected_total: usize) {
    let g = state.god_view();
    let mut seen = vec![false; expected_total];
    let mut count = 0usize;

    for card in g
        .hands
        .iter()
        .flatten()
        .chain(&g.draw_pile)
        .chain(&g.discard)
    {
        let idx = usize::from(card.id);
        assert!(idx < expected_total, "card id out of bounds: {idx}");
        assert!(!seen[idx], "duplicate card id: {idx}");
        seen[idx] = true;
        count += 1;
    }

    assert_eq!(count, expected_total);
    assert!(seen.into_iter().all(|s| s));
}

fn assert_view_invariants(state: &GameState, seat: Seat) {
    let view = state.view(seat);
    assert_eq!(view.me, Some(seat));
    let is_flip = view.config.mode == ModeKind::Flip;
    assert_eq!(
        view.draw_pile_top_back.is_some(),
        is_flip && view.draw_pile_size > 0
    );
    for p in &view.players {
        if is_flip && p.seat != seat {
            let backs = p
                .hand_backs
                .as_ref()
                .expect("Flip mode must populate opponent hand_backs");
            assert_eq!(backs.len(), usize::from(p.hand_size));
        } else {
            assert!(p.hand_backs.is_none());
        }
        if p.seat == seat {
            assert_eq!(view.my_hand.len(), usize::from(p.hand_size));
        }
    }
}

fn assert_valid_rankings(rankings: &[Seat], num_players: u8) {
    let n = usize::from(num_players);
    assert_eq!(rankings.len(), n);
    let mut seen = vec![false; n];
    for &seat in rankings {
        let idx = usize::from(seat);
        assert!(idx < n, "ranking seat out of bounds: {idx}");
        assert!(!seen[idx], "duplicate seat in rankings: {idx}");
        seen[idx] = true;
    }
    assert!(seen.into_iter().all(|s| s));
}

fn run_random_game(
    cfg: RuleConfig,
    seed: u64,
    verify_invariants: bool,
    record_history: bool,
    max_steps: usize,
) -> (Vec<(Seat, Action)>, u64) {
    let expected_total = expected_total_cards(&cfg);
    let num_players = cfg.num_players;
    let mut state = GameState::new(cfg.clone(), seed).expect("valid config");
    let mut action_rng = Rng::new(seed ^ 0x9E37_79B9_7F4A_7C15);
    let mut history = Vec::new();
    let mut steps = 0usize;

    while !state.is_over() {
        if verify_invariants {
            assert_card_conservation(&state, expected_total);
        }

        if state.phase() == Phase::RoundOver {
            state
                .next_round()
                .expect("RoundOver next_round must succeed");
            continue;
        }

        if steps >= max_steps {
            let act = Action::EndByTime;
            state.apply(0, act).expect("EndByTime must succeed");
            if record_history {
                history.push((0, act));
            }
            break;
        }

        let dp = state
            .decision_player()
            .expect("non-over active phase must have decision_player");

        if verify_invariants && steps.is_multiple_of(16) {
            assert_view_invariants(&state, dp);
        }

        let actions = state.legal_actions(dp);
        assert!(
            !actions.is_empty(),
            "decision_player must have legal actions in phase {:?}",
            state.phase()
        );

        let mut chosen: Option<(Seat, Action)> = None;

        // Occasional JumpIn when enabled
        if cfg.jump_in && state.phase() == Phase::Play && action_rng.below(6) == 0 {
            let offset = action_rng.below(u64::from(num_players)) as u8;
            let jumper = (dp + 1 + offset) % num_players;
            if jumper != dp {
                let j_acts = state.legal_actions(jumper);
                if let Some(&jump_act) = j_acts.iter().find(|a| matches!(a, Action::JumpIn { .. }))
                {
                    chosen = Some((jumper, jump_act));
                }
            }
        }

        // Occasional CatchUno when Manual UNO is active
        if chosen.is_none() && cfg.uno_rule == UnoRule::Manual && action_rng.below(4) == 0 {
            let catcher = (dp + 1) % num_players;
            let catcher_actions = state.legal_actions(catcher);
            if let Some(&catch_act) = catcher_actions
                .iter()
                .find(|a| matches!(a, Action::CatchUno { .. }))
            {
                chosen = Some((catcher, catch_act));
            }
        }

        let (seat, action) = chosen.unwrap_or_else(|| {
            let idx = action_rng.below(actions.len() as u64) as usize;
            (dp, actions[idx])
        });

        state
            .apply(seat, action)
            .expect("legal action must apply cleanly");
        if record_history {
            history.push((seat, action));
        }
        steps += 1;
    }

    if verify_invariants {
        assert_card_conservation(&state, expected_total);
    }
    let rankings = state.rankings().expect("GameOver must have rankings");
    assert_valid_rankings(rankings, num_players);
    assert_eq!(state.apply(0, Action::Draw), Err(EngineError::GameOver));

    let final_hash = if record_history {
        state.state_hash()
    } else {
        0
    };
    (history, final_hash)
}

fn replay_and_verify(cfg: RuleConfig, seed: u64, history: &[(Seat, Action)], expected_hash: u64) {
    let expected_total = expected_total_cards(&cfg);
    let num_players = cfg.num_players;
    let mut replay = GameState::new(cfg, seed).unwrap();
    for &(seat, action) in history {
        if replay.phase() == Phase::RoundOver {
            replay.next_round().unwrap();
        }
        replay.apply(seat, action).unwrap();
    }
    if replay.phase() == Phase::RoundOver {
        replay.next_round().unwrap();
    }
    assert_card_conservation(&replay, expected_total);
    assert!(replay.is_over());
    assert_eq!(replay.state_hash(), expected_hash);
    assert_eq!(replay.rankings().unwrap().len(), usize::from(num_players));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn prop_all_preset_modes_random_play(
        mode_idx in 0usize..8,
        raw_players in 0u8..9,
        seed in 0u64..100_000,
    ) {
        let mode_id = ALL_MODES[mode_idx];
        let num_players = preset_player_count(mode_id, raw_players);
        let cfg = RuleConfig::preset(mode_id, num_players).unwrap();

        let (history, expected_hash) = run_random_game(cfg.clone(), seed, true, true, 4000);
        replay_and_verify(cfg, seed, &history, expected_hash);
    }

    #[test]
    fn prop_custom_house_rules_random_play(
        mode_sel in 0u8..3,
        raw_players in 2u8..=8,
        seed in 0u64..100_000,
        stack_sel in 0u8..4,
        seven_zero in proptest::prelude::any::<bool>(),
        jump_in in proptest::prelude::any::<bool>(),
        draw_to_match in proptest::prelude::any::<bool>(),
        force_play in proptest::prelude::any::<bool>(),
        w4_sel in 0u8..3,
        manual_uno in proptest::prelude::any::<bool>(),
        deck_count in 1u8..=2,
        hand_size in 4u8..=10,
        enable_teams in proptest::prelude::any::<bool>(),
        enable_elim in proptest::prelude::any::<bool>(),
    ) {
        let mode = match mode_sel {
            0 => ModeKind::Classic,
            1 => ModeKind::Flip,
            _ => ModeKind::NoMercy,
        };
        let num_players = if mode == ModeKind::NoMercy {
            2 + (raw_players % 5)
        } else {
            raw_players
        };
        let mut cfg = RuleConfig::preset("custom", num_players).unwrap();
        cfg.mode = mode;
        cfg.deck_count = deck_count;
        cfg.hand_size = hand_size;
        cfg.stack_draw = match stack_sel {
            0 => StackDraw::None,
            1 => StackDraw::Same,
            2 => StackDraw::Progressive,
            _ => StackDraw::NoMercy,
        };
        cfg.seven_zero = seven_zero;
        cfg.jump_in = jump_in;
        cfg.draw_to_match = draw_to_match;
        cfg.force_play = force_play;
        cfg.wild4_rule = match w4_sel {
            0 => Wild4Rule::Challenge,
            1 => Wild4Rule::Strict,
            _ => Wild4Rule::Free,
        };
        cfg.uno_rule = if manual_uno { UnoRule::Manual } else { UnoRule::Auto };
        cfg.scoring = Scoring::Points { target: 200 };
        cfg.teams = if enable_teams && (num_players == 4 || num_players == 6) {
            Some(2)
        } else {
            None
        };
        cfg.elimination = enable_elim && num_players >= 3 && mode != ModeKind::NoMercy;
        prop_assert!(cfg.validate().is_ok());

        let (history, expected_hash) = run_random_game(cfg.clone(), seed, true, true, 3000);
        replay_and_verify(cfg, seed, &history, expected_hash);
    }
}

#[test]
fn multi_round_deterministic_replay_2_to_10_players() {
    for num_players in 2u8..=10 {
        let mut cfg = RuleConfig::preset("classic", num_players).unwrap();
        if num_players >= 8 {
            cfg.deck_count = 2;
        }
        cfg.scoring = Scoring::Points { target: 80 };

        for offset in 0u64..2 {
            let seed = u64::from(num_players) * 1000 + offset;
            let (history, expected_hash) = run_random_game(cfg.clone(), seed, false, true, 4000);

            let mut s1 = GameState::new(cfg.clone(), seed).unwrap();
            let mut s2 = GameState::new(cfg.clone(), seed).unwrap();
            assert_eq!(s1.state_hash(), s2.state_hash());

            for (step, &(seat, action)) in history.iter().enumerate() {
                if s1.phase() == Phase::RoundOver {
                    let r1 = s1.next_round().unwrap();
                    let r2 = s2.next_round().unwrap();
                    assert_eq!(r1, r2);
                }
                let e1 = s1.apply(seat, action).unwrap();
                let e2 = s2.apply(seat, action).unwrap();
                assert_eq!(e1, e2);
                if step % 64 == 0 {
                    assert_eq!(s1.state_hash(), s2.state_hash());
                }
            }
            assert_eq!(s1.state_hash(), expected_hash);
            assert_eq!(s2.state_hash(), expected_hash);
        }
    }
}

#[test]
#[ignore = "全模式各 20,000 場壓力測試：cargo test -p uno-core --release --test random_play stress_all_modes_20k_each -- --ignored --nocapture"]
fn stress_all_modes_20k_each() {
    let start = std::time::Instant::now();
    for mode_id in ALL_MODES {
        let mode_start = std::time::Instant::now();
        for i in 0u64..20_000 {
            let num_players = preset_player_count(mode_id, (i % 9) as u8);
            let mut cfg = RuleConfig::preset(mode_id, num_players).unwrap();
            if num_players >= 8 && mode_id != "no_mercy" {
                cfg.deck_count = 2;
            }
            let _ = run_random_game(cfg, i ^ 0xA5A5_5A5A, false, false, 3000);
        }
        println!(
            "mode {:<12}: 20,000 games in {:.2?}",
            mode_id,
            mode_start.elapsed()
        );
    }
    println!(
        "all 8 modes x 20,000 games completed in {:.2?}",
        start.elapsed()
    );
}
