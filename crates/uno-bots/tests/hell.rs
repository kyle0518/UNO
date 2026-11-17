use uno_bots::{make_bot, Bot, BotKind, HellBot, HeuristicBot, HELL_SIMS};
use uno_core::{Action, GameState, Phase, RuleConfig, Scoring};

#[test]
fn hell_bot_kind_parses_and_builds() {
    assert_eq!(HELL_SIMS, 1_500);
    let bot = make_bot(&BotKind::Hell, 7).expect("make_bot Hell");
    assert_eq!(bot.name(), "hell");
    assert_eq!(format!("{}", BotKind::Hell), "hell");
}

#[test]
fn hell_bot_fixed_seed_is_deterministic_and_legal() {
    for mode_id in ["classic", "flip", "no_mercy"] {
        let cfg = RuleConfig::preset(mode_id, 4).expect("valid preset");
        let mut state = GameState::new(cfg, 321).expect("state");
        for _ in 0..30 {
            if matches!(state.phase(), Phase::RoundOver | Phase::GameOver) {
                break;
            }
            let seat = state.decision_player().expect("decision player");
            let view = state.view(seat);
            if view.legal.len() >= 2 {
                let mut bot_a = HellBot::new(555, 60);
                let mut bot_b = HellBot::new(555, 60);
                let act_a = bot_a.act(&view);
                let act_b = bot_b.act(&view);
                assert_eq!(act_a, act_b, "HellBot must be deterministic for fixed seed");
                assert!(view.legal.contains(&act_a), "HellBot action must be legal");
                break;
            }
            let a = HeuristicBot::new().act(&view);
            state.apply(seat, a).expect("valid step");
        }
    }
}

#[test]
fn hell_bot_plays_full_round_legally_across_modes() {
    for (idx, mode_id) in ["classic", "flip", "no_mercy", "teams"].iter().enumerate() {
        let mut cfg = RuleConfig::preset(mode_id, 4).expect("preset");
        if matches!(cfg.scoring, Scoring::Points { .. }) {
            cfg.scoring = Scoring::SingleRound;
        }
        let mut state = GameState::new(cfg, 900 + idx as u64).expect("state");
        let mut hell = HellBot::new(42 + idx as u64, 30);
        let mut heur = HeuristicBot::new();
        let mut steps = 0_usize;
        while !matches!(state.phase(), Phase::RoundOver | Phase::GameOver) && steps < 400 {
            steps += 1;
            if let Some(u) = state.view(0).uno_window {
                let uv = state.view(u);
                if uv.legal.contains(&Action::CallUno) {
                    let act = if u == 0 { hell.act(&uv) } else { heur.act(&uv) };
                    assert_eq!(act, Action::CallUno);
                    state.apply(u, act).expect("call uno");
                    continue;
                }
            }
            let Some(seat) = state.decision_player() else {
                break;
            };
            let view = state.view(seat);
            let act = if seat == 0 {
                hell.act(&view)
            } else {
                heur.act(&view)
            };
            assert!(
                view.legal.contains(&act),
                "action must be legal in {mode_id}"
            );
            state.apply(seat, act).expect("apply legal");
        }
        assert!(
            matches!(state.phase(), Phase::RoundOver | Phase::GameOver),
            "round should finish in {mode_id}"
        );
    }
}
