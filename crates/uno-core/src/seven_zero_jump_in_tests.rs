use super::*;
use crate::card::{Card, Face};
use crate::config::{RuleConfig, StackDraw, UnoRule};
use crate::deck::build_deck;
use crate::rng::Rng;

fn pick_card(cards: &[Card], used: &mut [bool], color: Option<Color>, face: Face) -> CardId {
    let found = cards
        .iter()
        .find(|c| !used[usize::from(c.id)] && c.light.color == color && c.light.face == face)
        .expect("requested card must exist in unused cards")
        .id;
    used[usize::from(found)] = true;
    found
}

fn sz_deck(
    cfg: &RuleConfig,
    hands_spec: &[Vec<(Option<Color>, Face)>],
    start_spec: (Option<Color>, Face),
) -> Vec<CardId> {
    let cards = build_deck(cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    let mut picked_hands: Vec<Vec<CardId>> = Vec::with_capacity(hands_spec.len());
    for spec in hands_spec {
        let mut h = Vec::new();
        for &(c, f) in spec {
            h.push(pick_card(&cards, &mut used, c, f));
        }
        picked_hands.push(h);
    }
    let start_id = pick_card(&cards, &mut used, start_spec.0, start_spec.1);

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
            if let Some(id) = picked_hands
                .get(seat)
                .and_then(|v| v.get(round_idx))
                .copied()
            {
                order.push(id);
            } else {
                order.push(remaining.pop().unwrap());
            }
        }
    }
    order.push(start_id);
    order.extend(remaining);
    order
}

#[test]
fn seven_zero_seven_swaps_target_and_clears_uno_without_opening_window() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    cfg.seven_zero = true;
    cfg.uno_rule = UnoRule::Manual;

    let order = sz_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(7)),
                (Some(Color::Green), Face::Number(3)),
            ],
            vec![
                (Some(Color::Yellow), Face::Number(4)),
                (Some(Color::Yellow), Face::Number(5)),
            ],
            vec![
                (Some(Color::Green), Face::Number(6)),
                (Some(Color::Blue), Face::Number(8)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
    );

    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let s1_keep = s.hands[1][1];
    let s3_orig = s.hands[3].clone();
    let seven = s.hands[1][0];
    s.apply(1, Action::Play { card: seven }).unwrap();
    assert_eq!(s.phase(), Phase::ChooseTarget);
    assert_eq!(s.decision_player(), Some(1));
    assert_eq!(s.uno_window, Some(1));

    // seat 1 選擇 seat 3 交換手牌 -> 互換後 seat 3 只剩 1 張，但依 R3.2.3 不開 UNO 視窗
    let ev = s.apply(1, Action::ChooseTarget { target: 3 }).unwrap();
    assert!(ev.contains(&Event::HandsSwapped { a: 1, b: 3 }));
    assert_eq!(s.hands[1], s3_orig);
    assert_eq!(s.hands[3], vec![s1_keep]);
    assert_eq!(s.uno_window, None);
    assert!(s.uno_called.iter().all(|&u| !u));
    assert_eq!(s.phase(), Phase::Play);
    assert_eq!(s.decision_player(), Some(2));
}

#[test]
fn seven_zero_zero_rotates_in_current_direction_and_last_card_skips_effect() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    cfg.seven_zero = true;
    cfg.uno_rule = UnoRule::Manual;

    let order = sz_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(0)),
                (Some(Color::Green), Face::Number(3)),
            ],
            vec![
                (Some(Color::Yellow), Face::Number(4)),
                (Some(Color::Yellow), Face::Number(5)),
            ],
            vec![
                (Some(Color::Green), Face::Number(6)),
                (Some(Color::Blue), Face::Number(8)),
            ],
        ],
        (Some(Color::Red), Face::Reverse), // 莊家為 0，起始 Reverse 使方向 = -1、seat 0 先出？改用 Blue 9 先手
    );
    drop(order);

    let order2 = sz_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(0)),
                (Some(Color::Green), Face::Number(3)),
            ],
            vec![
                (Some(Color::Yellow), Face::Number(4)),
                (Some(Color::Yellow), Face::Number(5)),
            ],
            vec![
                (Some(Color::Green), Face::Number(6)),
                (Some(Color::Blue), Face::Number(8)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
    );
    let mut s = GameState::new_with_deck(cfg.clone(), order2).unwrap();
    let h0 = s.hands[0].clone();
    let h1_after_play = vec![s.hands[1][1]];
    let h2 = s.hands[2].clone();
    let h3 = s.hands[3].clone();

    let zero = s.hands[1][0];
    let ev = s.apply(1, Action::Play { card: zero }).unwrap();
    assert!(ev.contains(&Event::HandsRotated { direction: 1 }));
    assert_eq!(s.hands[2], h1_after_play);
    assert_eq!(s.hands[3], h2);
    assert_eq!(s.hands[0], h3);
    assert_eq!(s.hands[1], h0);
    // R3.2.3：收到 1 張牌的 seat 2 不會開啟 UNO 視窗
    assert_eq!(s.uno_window, None);
    assert_eq!(s.decision_player(), Some(2));

    // 逆時針 direction = -1 傳牌驗證
    cfg.hand_size = 2;
    let order_rev = sz_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Reverse),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Green), Face::Number(3)),
            ],
            vec![
                (Some(Color::Yellow), Face::Number(4)),
                (Some(Color::Yellow), Face::Number(5)),
            ],
            vec![
                (Some(Color::Green), Face::Number(0)),
                (Some(Color::Blue), Face::Number(8)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
    );
    let mut sr = GameState::new_with_deck(cfg.clone(), order_rev).unwrap();
    let r_card = sr.hands[1][0];
    sr.apply(1, Action::Play { card: r_card }).unwrap(); // Red 1 -> seat 2 -> Draw? 讓 Reverse 由 seat 1 打或 start Reverse
    let _ = sr;

    // 起始為 Red Reverse (dir = -1, current = 0)，讓 0 出 Yellow/Red Reverse 改為測試 start Reverse 即可
    let order_sr = sz_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(0)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Green), Face::Number(3)),
            ],
            vec![
                (Some(Color::Yellow), Face::Number(4)),
                (Some(Color::Yellow), Face::Number(5)),
            ],
            vec![
                (Some(Color::Green), Face::Number(6)),
                (Some(Color::Blue), Face::Number(8)),
            ],
        ],
        (Some(Color::Red), Face::Reverse),
    );
    let mut s_rev = GameState::new_with_deck(cfg.clone(), order_sr).unwrap();
    assert_eq!(s_rev.decision_player(), Some(0));
    let exp_to_3 = vec![s_rev.hands[0][1]];
    let exp_to_2 = s_rev.hands[3].clone();
    let exp_to_1 = s_rev.hands[2].clone();
    let exp_to_0 = s_rev.hands[1].clone();
    let z0 = s_rev.hands[0][0];
    let ev_r = s_rev.apply(0, Action::Play { card: z0 }).unwrap();
    assert!(ev_r.contains(&Event::HandsRotated { direction: -1 }));
    assert_eq!(s_rev.hands[3], exp_to_3);
    assert_eq!(s_rev.hands[2], exp_to_2);
    assert_eq!(s_rev.hands[1], exp_to_1);
    assert_eq!(s_rev.hands[0], exp_to_0);
    assert_eq!(s_rev.decision_player(), Some(3));

    // 最後一張為 7 / 0 時直接獲勝，不進入 ChooseTarget / 不觸發傳牌
    cfg.hand_size = 1;
    let order_last7 = sz_deck(
        &cfg,
        &[
            vec![(Some(Color::Blue), Face::Number(1))],
            vec![(Some(Color::Red), Face::Number(7))],
        ],
        (Some(Color::Red), Face::Number(9)),
    );
    let mut sl7 = GameState::new_with_deck(cfg.clone(), order_last7).unwrap();
    let last7 = sl7.hands[1][0];
    let ev_l7 = sl7.apply(1, Action::Play { card: last7 }).unwrap();
    assert_eq!(sl7.phase(), Phase::RoundOver);
    assert!(!ev_l7
        .iter()
        .any(|e| matches!(e, Event::HandsSwapped { .. })));

    let order_last0 = sz_deck(
        &cfg,
        &[
            vec![(Some(Color::Blue), Face::Number(1))],
            vec![(Some(Color::Red), Face::Number(0))],
        ],
        (Some(Color::Red), Face::Number(9)),
    );
    let mut sl0 = GameState::new_with_deck(cfg, order_last0).unwrap();
    let last0 = sl0.hands[1][0];
    let ev_l0 = sl0.apply(1, Action::Play { card: last0 }).unwrap();
    assert_eq!(sl0.phase(), Phase::RoundOver);
    assert!(!ev_l0
        .iter()
        .any(|e| matches!(e, Event::HandsRotated { .. })));
}

#[test]
fn jump_in_allows_identical_colored_card_blocks_wild_and_pending_draw_and_advances_from_jumper() {
    let mut cfg = RuleConfig::preset("party", 4).unwrap();
    cfg.hand_size = 3;
    cfg.stack_draw = StackDraw::Progressive;

    let order = sz_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Green), Face::Number(1)),
                (Some(Color::Red), Face::Number(5)), // 與場上 Red 5 完全相同，可 JumpIn
                (None, Face::Wild),
            ],
            vec![
                (Some(Color::Red), Face::Skip),
                (Some(Color::Red), Face::Number(6)),
                (Some(Color::Blue), Face::DrawTwo),
            ],
            vec![
                (Some(Color::Red), Face::DrawTwo),
                (Some(Color::Yellow), Face::Number(5)), // 不同顏色數字 5，不可 JumpIn
                (Some(Color::Green), Face::Number(3)),
            ],
            vec![
                (Some(Color::Blue), Face::Skip),
                (Some(Color::Green), Face::Number(7)),
                (Some(Color::Yellow), Face::Number(8)),
            ],
        ],
        (Some(Color::Red), Face::Number(5)),
    );

    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    assert_eq!(s.decision_player(), Some(1));

    // 目前玩家 seat 1 不能用 JumpIn（只能 Play）
    let p1_r6 = s.hands[1][1];
    assert!(!s.legal_actions(1).contains(&Action::JumpIn { card: p1_r6 }));
    assert_eq!(
        s.apply(1, Action::JumpIn { card: p1_r6 }),
        Err(EngineError::IllegalAction)
    );

    // seat 2 只有同數不同色的 Yellow 5，不能 JumpIn
    let p2_y5 = s.hands[2][1];
    assert!(!s.legal_actions(2).contains(&Action::JumpIn { card: p2_y5 }));

    // seat 0 持有完全相同的 Red 5 -> 可搶出，搶出後輪到 seat 0 的下一家 seat 1（R3.3.3）
    let p0_r5 = s.hands[0][1];
    assert!(s.legal_actions(0).contains(&Action::JumpIn { card: p0_r5 }));
    let ev = s.apply(0, Action::JumpIn { card: p0_r5 }).unwrap();
    assert!(ev.contains(&Event::CardPlayed {
        seat: 0,
        card: s.card(p0_r5),
        jump_in: true,
    }));
    assert_eq!(s.decision_player(), Some(1));

    // pending_draw > 0 時不可 JumpIn（R3.3.2）
    let mut cfg2 = cfg;
    cfg2.hand_size = 2;
    let order2 = sz_deck(
        &cfg2,
        &[
            vec![
                (Some(Color::Green), Face::Number(2)),
                (Some(Color::Blue), Face::Number(1)),
            ],
            vec![
                (Some(Color::Red), Face::DrawTwo),
                (Some(Color::Yellow), Face::Number(2)),
            ],
            vec![
                (Some(Color::Green), Face::Number(3)),
                (Some(Color::Blue), Face::Number(4)),
            ],
            vec![
                (Some(Color::Red), Face::DrawTwo),
                (Some(Color::Yellow), Face::Number(5)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
    );
    let mut s2 = GameState::new_with_deck(cfg2, order2).unwrap();
    let d2 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: d2 }).unwrap();
    assert_eq!(s2.pending_draw, 2);
    // seat 3 雖持有完全相同的 Red DrawTwo，但 pending_draw > 0 禁止搶出
    assert!(!s2.legal_actions(3).contains(&Action::JumpIn {
        card: s2.hands[3][0]
    }));

    // 萬用牌不能搶出（R3.3.2）：場上為 Wild 且非當前玩家手上也有 Wild
    let mut cfg_w = RuleConfig::preset("party", 3).unwrap();
    cfg_w.hand_size = 2;
    let order_w = sz_deck(
        &cfg_w,
        &[
            vec![(None, Face::Wild), (Some(Color::Red), Face::Number(1))],
            vec![(None, Face::Wild), (Some(Color::Blue), Face::Number(2))],
            vec![(None, Face::Wild), (Some(Color::Green), Face::Number(3))],
        ],
        (Some(Color::Red), Face::Number(9)),
    );
    let mut sw = GameState::new_with_deck(cfg_w, order_w).unwrap();
    let w1 = sw.hands[1][0];
    sw.apply(1, Action::Play { card: w1 }).unwrap();
    sw.apply(
        1,
        Action::ChooseColor {
            color: Color::Green,
        },
    )
    .unwrap();
    assert_eq!(sw.decision_player(), Some(2));
    assert!(!sw
        .legal_actions(0)
        .iter()
        .any(|a| matches!(a, Action::JumpIn { .. })));
}
