use super::*;
use crate::card::{Card, Face};
use crate::config::{RuleConfig, StackDraw, UnoRule, Wild4Rule};
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

/// 建構可控牌序：依座位指定起始手牌、指定起始翻開牌、並可指定牌堆頂（最先抽到的牌）。
fn custom_deck(
    cfg: &RuleConfig,
    hands_spec: &[Vec<(Option<Color>, Face)>],
    start_spec: (Option<Color>, Face),
    top_draws: &[(Option<Color>, Face)],
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

    let mut picked_draws = Vec::new();
    for &(c, f) in top_draws {
        picked_draws.push(pick_card(&cards, &mut used, c, f));
    }

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
    order.extend(picked_draws);
    order.extend(remaining);
    order
}

#[test]
fn r2_2_6_start_wild_keeps_first_player() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    let order = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Green), Face::Number(3)),
                (Some(Color::Yellow), Face::Number(4)),
            ],
        ],
        (None, Face::Wild),
        &[],
    );
    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    assert_eq!(s.phase(), Phase::ChooseColor);
    assert_eq!(s.decision_player(), Some(1));
    s.apply(
        1,
        Action::ChooseColor {
            color: Color::Green,
        },
    )
    .unwrap();
    // R2.2.6：選完色後第一位玩家（seat 1）依然出牌
    assert_eq!(s.phase(), Phase::Play);
    assert_eq!(s.decision_player(), Some(1));
    let green3 = s.hands[1][0];
    s.apply(1, Action::Play { card: green3 }).unwrap();
    assert_eq!(s.decision_player(), Some(2));
}

#[test]
fn r2_4_skip_and_reverse_4p_and_2p() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    let order = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Skip),
                (Some(Color::Red), Face::Reverse),
            ],
            vec![
                (Some(Color::Yellow), Face::Number(3)),
                (Some(Color::Green), Face::Number(4)),
            ],
            vec![
                (Some(Color::Green), Face::Number(5)),
                (Some(Color::Blue), Face::Number(6)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut state = GameState::new_with_deck(cfg.clone(), order).unwrap();
    assert_eq!(state.decision_player(), Some(1));
    let skip_card = state.hands[1][0];
    let events = state.apply(1, Action::Play { card: skip_card }).unwrap();
    assert!(events.contains(&Event::PlayerSkipped { seat: 2 }));
    assert_eq!(state.decision_player(), Some(3));

    // 4 人局 Reverse
    let order2 = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Reverse),
                (Some(Color::Yellow), Face::Number(2)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut state2 = GameState::new_with_deck(cfg, order2).unwrap();
    let rev = state2.hands[1][0];
    let ev2 = state2.apply(1, Action::Play { card: rev }).unwrap();
    assert!(ev2.contains(&Event::DirectionChanged { direction: -1 }));
    assert_eq!(state2.decision_player(), Some(0));

    // 2 人局 Reverse 等同 Skip
    let mut cfg2p = RuleConfig::preset("classic", 2).unwrap();
    cfg2p.hand_size = 2;
    let order3 = custom_deck(
        &cfg2p,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Green), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Reverse),
                (Some(Color::Yellow), Face::Number(2)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut state3 = GameState::new_with_deck(cfg2p, order3).unwrap();
    assert_eq!(state3.decision_player(), Some(1));
    let rev2 = state3.hands[1][0];
    state3.apply(1, Action::Play { card: rev2 }).unwrap();
    assert_eq!(state3.decision_player(), Some(1));
}

#[test]
fn r2_5_voluntary_draw_decision_force_play_and_draw_to_match() {
    let mut cfg = RuleConfig::preset("classic", 2).unwrap();
    cfg.hand_size = 2;
    let order = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Green), Face::Number(2)),
            ],
            vec![
                (Some(Color::Blue), Face::Number(3)),
                (Some(Color::Green), Face::Number(4)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[(Some(Color::Red), Face::Number(5))],
    );
    let mut state = GameState::new_with_deck(cfg.clone(), order).unwrap();
    state.apply(1, Action::Draw).unwrap();
    assert_eq!(state.phase(), Phase::DrawnCardDecision);
    assert_eq!(state.decision_player(), Some(1));
    let drawn_id = state.drawn_card.unwrap();
    state.apply(1, Action::Play { card: drawn_id }).unwrap();
    assert_eq!(state.phase(), Phase::Play);
    assert_eq!(state.decision_player(), Some(0));

    // force_play 時不可 Pass
    let mut cfg_fp = cfg.clone();
    cfg_fp.force_play = true;
    let order_fp = custom_deck(
        &cfg_fp,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Green), Face::Number(2)),
            ],
            vec![
                (Some(Color::Blue), Face::Number(3)),
                (Some(Color::Green), Face::Number(4)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[(Some(Color::Red), Face::Number(5))],
    );
    let mut s_fp = GameState::new_with_deck(cfg_fp, order_fp).unwrap();
    s_fp.apply(1, Action::Draw).unwrap();
    assert_eq!(s_fp.apply(1, Action::Pass), Err(EngineError::IllegalAction));
    let did = s_fp.drawn_card.unwrap();
    s_fp.apply(1, Action::Play { card: did }).unwrap();
    assert_eq!(s_fp.decision_player(), Some(0));

    // draw_to_match 連續抽到符合牌才停
    cfg.draw_to_match = true;
    let order2 = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Green), Face::Number(2)),
            ],
            vec![
                (Some(Color::Blue), Face::Number(3)),
                (Some(Color::Green), Face::Number(4)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[
            (Some(Color::Blue), Face::Number(6)),
            (Some(Color::Yellow), Face::Number(7)),
            (Some(Color::Red), Face::Number(8)),
        ],
    );
    let mut state2 = GameState::new_with_deck(cfg, order2).unwrap();
    let ev = state2.apply(1, Action::Draw).unwrap();
    assert_eq!(state2.phase(), Phase::DrawnCardDecision);
    assert_eq!(state2.hands[1].len(), 5);
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::CardsDrawn {
            seat: 1,
            count: 3,
            ..
        }
    )));
    state2.apply(1, Action::Pass).unwrap();
    assert_eq!(state2.decision_player(), Some(0));
}

#[test]
fn r2_6_wild_draw_four_challenge_and_accept() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    cfg.wild4_rule = Wild4Rule::Challenge;

    // 1) Bluff: seat 1 holds Red 2 + Wild4 on Red 9 -> illegal bluff -> Challenge succeeds
    let order_bluff = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Red), Face::Number(2)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s1 = GameState::new_with_deck(cfg.clone(), order_bluff).unwrap();
    let w4 = s1.hands[1][0];
    s1.apply(1, Action::Play { card: w4 }).unwrap();
    assert_eq!(s1.phase(), Phase::ChooseColor);
    s1.apply(1, Action::ChooseColor { color: Color::Blue })
        .unwrap();
    assert_eq!(s1.phase(), Phase::Challenge);
    assert_eq!(s1.decision_player(), Some(2));
    let ev = s1.apply(2, Action::Challenge).unwrap();
    assert!(ev.contains(&Event::ChallengeResolved {
        challenger: 2,
        offender: 1,
        success: true
    }));
    assert_eq!(s1.hands[1].len(), 5);
    assert_eq!(s1.hands[2].len(), 2);
    assert_eq!(s1.phase(), Phase::Play);
    assert_eq!(s1.decision_player(), Some(2));

    // 2) Honest: seat 1 has NO Red -> Challenge fails -> challenger draws 6 and skipped
    let order_honest = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Green), Face::Number(2)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s2 = GameState::new_with_deck(cfg.clone(), order_honest).unwrap();
    let w4_2 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: w4_2 }).unwrap();
    s2.apply(
        1,
        Action::ChooseColor {
            color: Color::Green,
        },
    )
    .unwrap();
    let ev2 = s2.apply(2, Action::Challenge).unwrap();
    assert!(ev2.contains(&Event::ChallengeResolved {
        challenger: 2,
        offender: 1,
        success: false
    }));
    assert_eq!(s2.hands[2].len(), 8);
    assert_eq!(s2.decision_player(), Some(3));

    // 3) Accept: challenger accepts -> draws 4 and skipped
    let order_acc = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Red), Face::Number(2)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s3 = GameState::new_with_deck(cfg, order_acc).unwrap();
    let w4_3 = s3.hands[1][0];
    s3.apply(1, Action::Play { card: w4_3 }).unwrap();
    s3.apply(
        1,
        Action::ChooseColor {
            color: Color::Yellow,
        },
    )
    .unwrap();
    s3.apply(2, Action::Accept).unwrap();
    assert_eq!(s3.hands[2].len(), 6);
    assert_eq!(s3.decision_player(), Some(3));
}

#[test]
fn r2_7_uno_window_catch_self_call_precall_and_wild_non_close() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    cfg.uno_rule = UnoRule::Manual;

    // 1) 忘喊被抓
    let order = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(3)),
                (Some(Color::Blue), Face::Number(4)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    let c1 = s.hands[1][0];
    s.apply(1, Action::Play { card: c1 }).unwrap();
    assert_eq!(s.uno_window, Some(1));
    assert!(s.legal_actions(2).contains(&Action::CatchUno { target: 1 }));
    assert!(s.legal_actions(1).contains(&Action::CallUno));
    s.apply(3, Action::CatchUno { target: 1 }).unwrap();
    assert_eq!(s.hands[1].len(), 3);
    assert_eq!(s.uno_window, None);

    // 2) 剩 2 張預先喊 UNO
    let order2 = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::Number(3)),
                (Some(Color::Blue), Face::Number(4)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s2 = GameState::new_with_deck(cfg.clone(), order2).unwrap();
    s2.apply(1, Action::CallUno).unwrap();
    let c2 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: c2 }).unwrap();
    assert_eq!(s2.uno_window, None);
    assert!(s2.uno_called[1]);

    // 3) R2.7.1: Wild 倒數第 2 張選色不關閉視窗，下一家動作後關閉
    let mut cfg3 = cfg;
    cfg3.hand_size = 3;
    let order3 = custom_deck(
        &cfg3,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Green), Face::Number(8)),
                (Some(Color::Blue), Face::Number(8)),
            ],
            vec![
                (Some(Color::Red), Face::Number(3)),
                (None, Face::Wild),
                (Some(Color::Green), Face::Number(4)),
            ],
            vec![
                (Some(Color::Red), Face::Number(5)),
                (Some(Color::Yellow), Face::Number(5)),
                (Some(Color::Green), Face::Number(7)),
            ],
            vec![
                (Some(Color::Red), Face::Number(6)),
                (Some(Color::Blue), Face::Number(6)),
                (Some(Color::Green), Face::Number(6)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s3 = GameState::new_with_deck(cfg3, order3).unwrap();
    for seat in [1u8, 2, 3, 0] {
        let cid = s3.hands[usize::from(seat)][0];
        s3.apply(seat, Action::Play { card: cid }).unwrap();
    }

    let wild = s3.hands[1][0];
    s3.apply(1, Action::Play { card: wild }).unwrap();
    assert_eq!(s3.uno_window, Some(1));
    s3.apply(
        1,
        Action::ChooseColor {
            color: Color::Yellow,
        },
    )
    .unwrap();
    assert_eq!(s3.uno_window, Some(1));
    assert_eq!(s3.decision_player(), Some(2));
    s3.apply(2, Action::Draw).unwrap();
    assert_eq!(s3.uno_window, None);
}

#[test]
fn r2_8_last_card_draw_two_and_wild_draw_four() {
    let mut cfg = RuleConfig::preset("classic", 2).unwrap();
    cfg.hand_size = 1;

    // 最後一張為 WildDrawFour：先 ChooseColor，不進 Challenge，直接抽 4 並算入點數
    let order = custom_deck(
        &cfg,
        &[
            vec![(Some(Color::Red), Face::Number(1))],
            vec![(None, Face::WildDrawFour)],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    let w4 = s.hands[1][0];
    s.apply(1, Action::Play { card: w4 }).unwrap();
    assert_eq!(s.phase(), Phase::ChooseColor);
    let ev = s
        .apply(1, Action::ChooseColor { color: Color::Blue })
        .unwrap();
    assert!(!matches!(s.phase(), Phase::Challenge));
    assert_eq!(s.hands[0].len(), 5);
    assert_eq!(s.phase(), Phase::RoundOver);
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::RoundEnded {
            winner: Some(1),
            ..
        }
    )));

    // 最後一張為 DrawTwo：對手仍先抽 2 張並計入總點數
    let order2 = custom_deck(
        &cfg,
        &[
            vec![(Some(Color::Blue), Face::Number(1))],
            vec![(Some(Color::Red), Face::DrawTwo)],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[
            (Some(Color::Green), Face::Number(5)),
            (Some(Color::Yellow), Face::Number(7)),
        ],
    );
    let mut s2 = GameState::new_with_deck(cfg, order2).unwrap();
    let d2 = s2.hands[1][0];
    let ev2 = s2.apply(1, Action::Play { card: d2 }).unwrap();
    assert_eq!(s2.hands[0].len(), 3);
    assert!(ev2.contains(&Event::RoundEnded {
        winner: Some(1),
        scores: vec![0, 13],
        hand_points: vec![13, 0],
        totals: vec![0, 13],
    }));
}

#[test]
fn r2_10_timeout_challenge_wild_color_and_end_by_time() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    let order = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![(None, Face::Wild), (Some(Color::Green), Face::Number(4))],
            vec![
                (Some(Color::Blue), Face::Number(5)),
                (Some(Color::Blue), Face::Number(6)),
            ],
        ],
        (Some(Color::Yellow), Face::Number(9)),
        &[(Some(Color::Yellow), Face::Number(3))],
    );
    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    // Play Timeout -> Draw -> Pass
    s.apply(1, Action::Timeout).unwrap();
    assert_eq!(s.hands[1].len(), 3);
    assert_eq!(s.decision_player(), Some(2));

    assert_eq!(s.apply(1, Action::Draw), Err(EngineError::NotYourTurn));

    // ChooseColor Timeout 選最多顏色（Green > 其他）
    let mut cfg_w4 = cfg;
    cfg_w4.hand_size = 3;
    let order_w4 = custom_deck(
        &cfg_w4,
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
                (Some(Color::Yellow), Face::Number(3)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Green), Face::Number(4)),
                (Some(Color::Green), Face::Number(5)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s2 = GameState::new_with_deck(cfg_w4, order_w4).unwrap();
    let w4 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: w4 }).unwrap();
    s2.apply(1, Action::Timeout).unwrap();
    assert_eq!(s2.active_color, Some(Color::Green));
    assert_eq!(s2.phase(), Phase::Challenge);
    assert_eq!(s2.decision_player(), Some(2));
    // Challenge Timeout -> Accept (draw 4 + skip to seat 3)
    s2.apply(2, Action::Timeout).unwrap();
    assert_eq!(s2.hands[2].len(), 7);
    assert_eq!(s2.decision_player(), Some(3));

    // EndByTime 立即結算
    let ev = s.apply(0, Action::EndByTime).unwrap();
    assert_eq!(s.phase(), Phase::GameOver);
    assert!(s.rankings().is_some());
    assert!(ev.iter().any(|e| matches!(e, Event::GameEnded { .. })));
    assert_eq!(s.apply(2, Action::Draw), Err(EngineError::GameOver));
}

#[test]
fn r2_4_draw2_stack_when_enabled() {
    let mut cfg = RuleConfig::preset("classic", 2).unwrap();
    cfg.hand_size = 2;
    cfg.stack_draw = StackDraw::Same;
    let order = custom_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::DrawTwo),
                (Some(Color::Green), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::DrawTwo),
                (Some(Color::Yellow), Face::Number(3)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let d2_p1 = s.hands[1][0];
    s.apply(1, Action::Play { card: d2_p1 }).unwrap();
    assert_eq!(s.pending_draw, 2);
    let d2_p0 = s.hands[0][0];
    s.apply(0, Action::Play { card: d2_p0 }).unwrap();
    assert_eq!(s.pending_draw, 4);
    s.apply(1, Action::Draw).unwrap();
    assert_eq!(s.pending_draw, 0);
    assert_eq!(s.hands[1].len(), 5);
    assert_eq!(s.decision_player(), Some(0));
}
