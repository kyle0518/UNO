use uno_bots::{make_bot, Bot, BotError, BotKind, HeuristicBot, RandomBot};
use uno_core::{Action, GameState, Phase, RuleConfig, Scoring};

fn run_single_game(config: RuleConfig, seed: u64, kinds: &[BotKind]) {
    let mut state = GameState::new(config, seed).expect("valid config");
    let mut bots: Vec<Box<dyn Bot>> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| make_bot(k, seed.wrapping_add(i as u64 + 1)).expect("valid bot"))
        .collect();

    let mut steps = 0_usize;
    while !matches!(state.phase(), Phase::GameOver) && steps < 5_000 {
        steps += 1;

        if matches!(state.phase(), Phase::RoundOver) {
            state.next_round().expect("next_round should succeed");
            continue;
        }

        // 若 UNO 視窗開啟，檢查喊 UNO 玩家是否都會優先喊 CallUno
        if let Some(uno_seat) = state.view(0).uno_window {
            let uno_view = state.view(uno_seat);
            if uno_view.legal.contains(&Action::CallUno) {
                let uno_action = bots[usize::from(uno_seat)].act(&uno_view);
                assert!(uno_view.legal.contains(&uno_action));
                assert_eq!(uno_action, Action::CallUno);
                state.apply(uno_seat, uno_action).expect("call uno valid");
                continue;
            }
        }

        let seat = state
            .decision_player()
            .expect("active phase has decision_player");
        let view = state.view(seat);
        assert!(
            !view.legal.is_empty(),
            "decision player must have legal moves"
        );

        let action = bots[usize::from(seat)].act(&view);
        assert!(
            view.legal.contains(&action),
            "bot {} returned illegal action {action:?} in mode {}",
            bots[usize::from(seat)].name(),
            view.config.mode_id
        );

        state
            .apply(seat, action)
            .expect("action from view.legal must apply");
    }

    assert!(steps < 5_000, "game exceeded step limit (seed={seed})");
}

#[test]
fn thousand_games_all_actions_legal() {
    let mode_ids = [
        "classic",
        "party",
        "flip",
        "no_mercy",
        "teams",
        "elimination",
        "blitz",
    ];

    for game_idx in 0_u64..1_000 {
        let mode_id = mode_ids[(game_idx as usize) % mode_ids.len()];
        let num_players = match mode_id {
            "teams" => {
                if game_idx.is_multiple_of(2) {
                    4
                } else {
                    6
                }
            }
            "elimination" => 3 + ((game_idx % 4) as u8),
            _ => 2 + ((game_idx % 5) as u8),
        };

        let mut cfg = RuleConfig::preset(mode_id, num_players).expect("valid preset");
        if matches!(cfg.scoring, Scoring::Points { .. }) && !cfg.elimination {
            cfg.scoring = Scoring::Points { target: 100 };
        }

        let kinds: Vec<BotKind> = (0..usize::from(num_players))
            .map(|seat| {
                if (game_idx as usize + seat).is_multiple_of(2) {
                    BotKind::Heuristic
                } else {
                    BotKind::Random
                }
            })
            .collect();

        run_single_game(cfg, 10_000 + game_idx, &kinds);
    }
}

#[test]
fn heuristic_prioritizes_call_and_catch_uno() {
    let mut cfg = RuleConfig::preset("classic", 3).expect("valid preset");
    cfg.seven_zero = true;

    let state = GameState::new(cfg, 42).expect("valid config");
    let mut view = state.view(0);

    // 測試 CallUno 優先
    view.legal = vec![Action::Draw, Action::CallUno];
    let mut bot = HeuristicBot::new();
    assert_eq!(bot.act(&view), Action::CallUno);

    // RandomBot 也優先 CallUno
    let mut rand_bot = RandomBot::new(1);
    assert_eq!(rand_bot.act(&view), Action::CallUno);

    // 測試 CatchUno 優先於其他動作
    view.legal = vec![Action::Draw, Action::CatchUno { target: 2 }];
    assert_eq!(bot.act(&view), Action::CatchUno { target: 2 });
}

#[test]
fn make_bot_and_kind_parsing() {
    assert_eq!("random".parse::<BotKind>().ok(), Some(BotKind::Random));
    assert_eq!(
        "heuristic".parse::<BotKind>().ok(),
        Some(BotKind::Heuristic)
    );
    assert_eq!("hard".parse::<BotKind>().ok(), Some(BotKind::Hard));
    assert!("unknown".parse::<BotKind>().is_err());

    let r = make_bot(&BotKind::Random, 1).expect("random bot");
    assert_eq!(r.name(), "random");

    let h = make_bot(&BotKind::Heuristic, 1).expect("heuristic bot");
    assert_eq!(h.name(), "heuristic");

    assert!(matches!(
        make_bot(&BotKind::Onnx("m.onnx".into()), 1),
        Err(BotError::OnnxUnsupported(_))
    ));
}
