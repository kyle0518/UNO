use crate::card::{Card, CardId, Color, Face};
use crate::config::{RuleConfig, Scoring, UnoRule};
use crate::deck::build_deck;
use crate::events::{Action, EliminationReason, EngineError, Event, Phase};
use crate::rng::Rng;
use crate::state::GameState;

fn pick_unused(cards: &[Card], used: &mut [bool], color: Option<Color>, face: Face) -> CardId {
    let cid = cards
        .iter()
        .find(|c| !used[usize::from(c.id)] && c.light.color == color && c.light.face == face)
        .expect("requested card must exist in unused pool")
        .id;
    used[usize::from(cid)] = true;
    cid
}

fn rigged_deck(
    cfg: &RuleConfig,
    hands_spec: &[Vec<(Option<Color>, Face)>],
    start_spec: (Option<Color>, Face),
) -> Vec<CardId> {
    let cards = build_deck(cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    let mut hands: Vec<Vec<CardId>> = Vec::with_capacity(hands_spec.len());
    for spec in hands_spec {
        let mut h = Vec::with_capacity(spec.len());
        for &(c, f) in spec {
            h.push(pick_unused(&cards, &mut used, c, f));
        }
        hands.push(h);
    }
    let start_id = pick_unused(&cards, &mut used, start_spec.0, start_spec.1);

    let mut remaining: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used[usize::from(id)])
        .collect();

    let n = usize::from(cfg.num_players);
    let h_size = usize::from(cfg.hand_size);
    let mut order = Vec::with_capacity(cards.len());
    for round_idx in 0..h_size {
        for offset in 0..n {
            let seat = (1 + offset) % n;
            let cid = hands
                .get(seat)
                .and_then(|v| v.get(round_idx))
                .copied()
                .unwrap_or_else(|| remaining.pop().unwrap());
            order.push(cid);
        }
    }
    order.push(start_id);
    order.extend(remaining);
    order
}

#[test]
fn teams_r4_3_four_and_six_player_scoring_privacy_and_catch_block() {
    // 4 人（2v2）：Team 0 = [0, 2], Team 1 = [1, 3]
    let mut cfg4 = RuleConfig::preset("teams", 4).unwrap();
    cfg4.hand_size = 2;
    cfg4.uno_rule = UnoRule::Manual;
    let order4 = rigged_deck(
        &cfg4,
        &[
            vec![
                (Some(Color::Blue), Face::Number(9)),
                (Some(Color::Green), Face::Number(9)),
            ],
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Yellow), Face::Number(4)),
            ],
            vec![
                (Some(Color::Green), Face::Number(7)),
                (Some(Color::Yellow), Face::Number(8)),
            ],
            vec![
                (Some(Color::Blue), Face::Number(6)),
                (Some(Color::Red), Face::Number(2)),
            ],
        ],
        (Some(Color::Red), Face::Number(5)),
    );
    let mut s4 = GameState::new_with_deck(cfg4, order4).unwrap();

    // seat 1 沒喊 UNO 打剩 1 張 -> 開啟 UNO 視窗
    let c1 = s4.hands[1][0];
    s4.apply(1, Action::Play { card: c1 }).unwrap();
    assert_eq!(s4.uno_window, Some(1));

    // R4.3.3：隊友 seat 3 看不到 seat 1 手牌，且不能 CatchUno 隊友
    let v3 = s4.view(3);
    assert_eq!(v3.players[1].team, Some(1));
    assert_eq!(v3.players[3].team, Some(1));
    assert_eq!(v3.players[1].hand_size, 1);
    assert!(v3.players[1].hand_backs.is_none());
    assert!(!v3.legal.contains(&Action::CatchUno { target: 1 }));
    assert_eq!(
        s4.apply(3, Action::CatchUno { target: 1 }),
        Err(EngineError::IllegalAction)
    );

    // 敵隊 seat 0 可以抓 seat 1
    assert!(s4
        .legal_actions(0)
        .contains(&Action::CatchUno { target: 1 }));
    s4.apply(0, Action::CatchUno { target: 1 }).unwrap();
    assert_eq!(s4.hands[1].len(), 3);

    // 6 人（3v3）：Team 0 = [0, 2, 4], Team 1 = [1, 3, 5]
    let mut cfg6 = RuleConfig::preset("teams", 6).unwrap();
    cfg6.hand_size = 1;
    cfg6.scoring = Scoring::Points { target: 60 };
    let order6 = rigged_deck(
        &cfg6,
        &[
            vec![(Some(Color::Blue), Face::Skip)],      // 20 (Team 0)
            vec![(Some(Color::Red), Face::Number(3))],  // 贏家 (Team 1)
            vec![(None, Face::Wild)],                   // 50 (Team 0)
            vec![(None, Face::WildDrawFour)],           // 50 (Team 1, 不計分)
            vec![(Some(Color::Yellow), Face::Reverse)], // 20 (Team 0)
            vec![(Some(Color::Green), Face::DrawTwo)],  // 20 (Team 1, 不計分)
        ],
        (Some(Color::Red), Face::Number(8)),
    );
    let mut s6 = GameState::new_with_deck(cfg6, order6).unwrap();
    let win6 = s6.hands[1][0];
    let ev6 = s6.apply(1, Action::Play { card: win6 }).unwrap();

    // 敵隊 [0, 2, 4] 總點數 = 20 + 50 + 20 = 90 >= 60 -> Team 1 [1, 3, 5] 全員 +90 獲勝
    assert_eq!(s6.phase, Phase::GameOver);
    assert!(ev6.contains(&Event::RoundEnded {
        winner: Some(1),
        scores: vec![0, 90, 0, 90, 0, 90],
        hand_points: vec![20, 0, 50, 50, 20, 20],
        totals: vec![0, 90, 0, 90, 0, 90],
    }));
    // Team 1 排前（同 90 分依手牌點數 0 < 20 < 50），Team 0 排後
    assert_eq!(s6.rankings.as_deref(), Some(&[1, 5, 3, 0, 4, 2][..]));
}

#[test]
fn elimination_r4_4_tie_breaks_no_scoring_and_full_4p_tournament() {
    // 1. 驗證 R4.4.1 同分比張數多、再同分比座位號大；R4.4.4 忽略 scoring 且 scores 全 0
    let mut cfg = RuleConfig::preset("elimination", 4).unwrap();
    cfg.hand_size = 1;
    cfg.scoring = Scoring::SingleRound; // 故意設 SingleRound 確認淘汰賽忽略它、仍只淘汰 1 人進入下一輪
    let order = rigged_deck(
        &cfg,
        &[
            vec![(Some(Color::Blue), Face::Skip)],      // 20 pts, 1 card
            vec![(Some(Color::Red), Face::Number(2))],  // 贏家 -> 0 pts
            vec![(Some(Color::Green), Face::Reverse)],  // 20 pts, 1 card (與 0, 3 同分同張數)
            vec![(Some(Color::Yellow), Face::DrawTwo)], // 20 pts, 1 card -> 座位號 3 最大，應被淘汰
        ],
        (Some(Color::Red), Face::Number(7)),
    );
    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let win_c = s.hands[1][0];
    let ev = s.apply(1, Action::Play { card: win_c }).unwrap();

    assert_eq!(s.phase, Phase::RoundOver);
    assert_eq!(s.scores, vec![0, 0, 0, 0]);
    assert!(ev.contains(&Event::RoundEnded {
        winner: Some(1),
        scores: vec![0, 0, 0, 0],
        hand_points: vec![20, 0, 20, 20],
        totals: vec![0, 0, 0, 0],
    }));
    assert!(ev.contains(&Event::PlayerEliminated {
        seat: 3,
        reason: EliminationReason::Elimination
    }));
    assert_eq!(s.eliminated, vec![false, false, false, true]);
    assert_eq!(s.elimination_order, vec![3]);

    // R4.4.2：下一輪發牌不再發給已淘汰的 seat 3，莊家輪轉亦維持有效座位
    let ev2 = s.next_round().unwrap();
    assert_eq!(s.round, 2);
    assert!(s.hands[3].is_empty());
    assert!(s.legal_actions(3).is_empty());
    assert!(ev2.iter().any(|e| matches!(
        e,
        Event::RoundStarted { round: 2, hand_sizes, .. }
            if hand_sizes == &[1, 1, 1, 0]
    )));

    // 1b. 同分但手牌張數較多者淘汰：seat 2 持有 Green Skip + Green 0 (20分, 2張) vs seat 0 Yellow Skip (20分, 1張)
    let mut cfg_tie = RuleConfig::preset("elimination", 4).unwrap();
    cfg_tie.hand_size = 1;
    let cards_t = build_deck(&cfg_tie, &mut Rng::new(0));
    let mut used_t = vec![false; cards_t.len()];
    let p1 = pick_unused(&cards_t, &mut used_t, Some(Color::Blue), Face::Number(5));
    let p2 = pick_unused(&cards_t, &mut used_t, Some(Color::Green), Face::Skip);
    let p3 = pick_unused(&cards_t, &mut used_t, Some(Color::Red), Face::Number(1)); // 贏家
    let p0 = pick_unused(&cards_t, &mut used_t, Some(Color::Yellow), Face::Skip);
    let st_card = pick_unused(&cards_t, &mut used_t, Some(Color::Red), Face::Number(7));
    let dr1 = pick_unused(&cards_t, &mut used_t, Some(Color::Blue), Face::Number(0));
    let dr2 = pick_unused(&cards_t, &mut used_t, Some(Color::Green), Face::Number(0));
    let rest_t: Vec<CardId> = cards_t
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used_t[usize::from(id)])
        .collect();
    let mut order_t = vec![p1, p2, p3, p0, st_card, dr1, dr2];
    order_t.extend(rest_t);
    let mut st_len_tie = GameState::new_with_deck(cfg_tie, order_t).unwrap();
    st_len_tie.apply(1, Action::Draw).unwrap(); // Blue 0 (5 pts)
    st_len_tie.apply(2, Action::Draw).unwrap(); // Green 0 -> 20 pts, 2 cards
    let ev_len = st_len_tie.apply(3, Action::Play { card: p3 }).unwrap(); // Red 1 wins
                                                                          // seat 2 (20 pts, 2 cards) 與 seat 0 (20 pts, 1 card) 同分，張數多的 seat 2 被淘汰
    assert!(ev_len.contains(&Event::PlayerEliminated {
        seat: 2,
        reason: EliminationReason::Elimination
    }));

    // 2. 完整 4 人淘汰賽從第 1 輪打到最後僅剩 1 人獲勝（共 3 位依序淘汰）
    let mut full_cfg = RuleConfig::preset("elimination", 4).unwrap();
    full_cfg.scoring = Scoring::Points { target: 1 }; // 驗證分數目標 1 也不會提前結束淘汰賽
    let mut game = GameState::new(full_cfg, 2026).unwrap();
    let mut rng = Rng::new(9999);
    let mut steps = 0usize;
    let mut rounds_ended = 0usize;

    while game.phase != Phase::GameOver {
        steps += 1;
        assert!(steps < 20_000, "elimination tournament exceeded step cap");

        let total = game.hands.iter().map(Vec::len).sum::<usize>()
            + game.draw_pile.len()
            + game.discard.len();
        assert_eq!(total, 108);

        if game.phase == Phase::RoundOver {
            rounds_ended += 1;
            assert_eq!(game.scores, vec![0, 0, 0, 0]);
            game.next_round().unwrap();
            for (seat_idx, &is_elim) in game.eliminated.iter().enumerate() {
                if is_elim {
                    assert!(game.hands[seat_idx].is_empty());
                    assert!(game.legal_actions(seat_idx as u8).is_empty());
                }
            }
            continue;
        }

        let actor = game
            .decision_player()
            .expect("active non-over game must have decision_player");
        assert!(!game.eliminated[usize::from(actor)]);
        let actions = game.legal_actions(actor);
        let idx = (rng.next_u64() as usize) % actions.len();
        let ev = game.apply(actor, actions[idx]).unwrap();
        for e in ev {
            if let Event::RoundEnded { scores, totals, .. } = e {
                assert_eq!(scores, vec![0, 0, 0, 0]);
                assert_eq!(totals, vec![0, 0, 0, 0]);
            }
        }
    }

    assert_eq!(rounds_ended, 2); // 第 1,2 輪後 RoundOver；第 3 輪結束直接 GameOver
    assert_eq!(game.scores, vec![0, 0, 0, 0]);
    assert_eq!(game.elimination_order.len(), 3);
    let survivor = (0..4u8)
        .find(|&p| !game.eliminated[usize::from(p)])
        .expect("one survivor must remain");
    let expected_rankings: Vec<u8> = std::iter::once(survivor)
        .chain(game.elimination_order.iter().copied().rev())
        .collect();
    assert_eq!(game.rankings.as_ref(), Some(&expected_rankings));
}
