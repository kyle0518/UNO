use uno_bots::{make_bot, Bot, BotKind, HardBot, HeuristicBot, DEFAULT_SIMS, MOBILE_SIMS};
use uno_core::{Action, GameState, Phase, RuleConfig};

#[test]
fn hard_bot_fixed_seed_is_deterministic_and_legal() {
    for mode_id in ["classic", "flip", "no_mercy"] {
        let cfg = RuleConfig::preset(mode_id, 4).expect("valid preset");
        let mut state = GameState::new(cfg, 123).expect("state");

        // 推進到有超過 1 個合法動作的盤面，確認確實執行 ISMCTS 模擬
        for _ in 0..30 {
            if matches!(state.phase(), Phase::RoundOver | Phase::GameOver) {
                break;
            }
            let seat = state.decision_player().expect("decision player");
            let view = state.view(seat);
            if view.legal.len() >= 2 {
                let mut bot_a = HardBot::new(999, 60);
                let mut bot_b = HardBot::new(999, 60);
                let act_a = bot_a.act(&view);
                let act_b = bot_b.act(&view);

                assert_eq!(act_a, act_b, "HardBot must be deterministic for fixed seed");
                assert!(view.legal.contains(&act_a), "HardBot action must be legal");
                break;
            }
            let a = HeuristicBot::new().act(&view);
            state.apply(seat, a).expect("valid step");
        }
    }
}

#[test]
fn hard_bot_plays_full_game_across_modes_legally() {
    assert_eq!(DEFAULT_SIMS, 400);
    assert_eq!(MOBILE_SIMS, 200);

    let via_factory = make_bot(&BotKind::Hard, 7).expect("make_bot Hard");
    assert_eq!(via_factory.name(), "hard");

    for (idx, mode_id) in ["classic", "flip", "no_mercy", "teams"].iter().enumerate() {
        let mut cfg = RuleConfig::preset(mode_id, 4).expect("preset");
        if matches!(cfg.scoring, uno_core::Scoring::Points { .. }) {
            cfg.scoring = uno_core::Scoring::SingleRound;
        }
        let mut state = GameState::new(cfg, 500 + idx as u64).expect("state");
        let mut hard = HardBot::new(77 + idx as u64, 40);
        let mut heur = HeuristicBot::new();

        let mut steps = 0_usize;
        while !matches!(state.phase(), Phase::RoundOver | Phase::GameOver) && steps < 400 {
            steps += 1;
            if let Some(u) = state.view(0).uno_window {
                let uv = state.view(u);
                if uv.legal.contains(&Action::CallUno) {
                    let act = if u == 0 { hard.act(&uv) } else { heur.act(&uv) };
                    assert_eq!(act, Action::CallUno);
                    state.apply(u, act).expect("call uno");
                    continue;
                }
            }
            let seat = state.decision_player().expect("decision player");
            let view = state.view(seat);
            let act = if seat == 0 {
                hard.act(&view)
            } else {
                heur.act(&view)
            };
            assert!(view.legal.contains(&act));
            state.apply(seat, act).expect("apply legal");
        }
        assert!(
            steps < 400,
            "mode {mode_id} did not finish round within limit"
        );
    }
}
