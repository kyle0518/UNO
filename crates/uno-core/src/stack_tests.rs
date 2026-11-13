use super::*;
use crate::card::{Card, Face};
use crate::config::{RuleConfig, StackDraw, UnoRule, Wild4Rule};
use crate::deck::build_deck;
use crate::events::DrawReason;
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

fn stack_deck(
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
fn stack_same_ignores_color_and_blocks_cross_type_and_non_draw() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 3;
    cfg.stack_draw = StackDraw::Same;
    cfg.wild4_rule = Wild4Rule::Challenge;

    let order = stack_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Green), Face::Number(1)),
                (Some(Color::Red), Face::Skip),
                (None, Face::Wild),
            ],
            vec![
                (Some(Color::Red), Face::DrawTwo),
                (Some(Color::Red), Face::Number(5)),
                (None, Face::Wild),
            ],
            vec![
                (Some(Color::Blue), Face::DrawTwo),
                (None, Face::WildDrawFour),
                (Some(Color::Red), Face::Number(6)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Yellow), Face::Number(2)),
                (Some(Color::Blue), Face::Number(4)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
    );

    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let p1_d2 = s.hands[1][0];
    let ev1 = s.apply(1, Action::Play { card: p1_d2 }).unwrap();
    assert_eq!(s.pending_draw, 2);
    assert!(ev1.contains(&Event::PendingDrawChanged { amount: 2 }));
    assert_eq!(s.decision_player(), Some(2));

    // seat 2: Same 模式下可用異色 DrawTwo 疊，不可用 WildDrawFour 或同色數字牌（R3.1.1 / R3.1.2 / R3.1.5）
    let legal2 = s.legal_actions(2);
    assert!(legal2.contains(&Action::Play {
        card: s.hands[2][0]
    }));
    assert!(!legal2.contains(&Action::Play {
        card: s.hands[2][1]
    }));
    assert!(!legal2.contains(&Action::Play {
        card: s.hands[2][2]
    }));
    assert!(legal2.contains(&Action::Draw));

    let p2_d2 = s.hands[2][0];
    s.apply(2, Action::Play { card: p2_d2 }).unwrap();
    assert_eq!(s.pending_draw, 4);
    assert_eq!(s.decision_player(), Some(3));

    // seat 3 僅有 WildDrawFour，不可疊在 DrawTwo 上 -> 只能 Draw 4 張並被跳過
    let legal3 = s.legal_actions(3);
    assert_eq!(legal3, vec![Action::Draw]);
    let ev3 = s.apply(3, Action::Draw).unwrap();
    assert_eq!(s.pending_draw, 0);
    assert_eq!(s.hands[3].len(), 7); // 3 + 4
    assert!(ev3.contains(&Event::PendingDrawChanged { amount: 0 }));
    assert!(ev3.iter().any(|e| matches!(
        e,
        Event::CardsDrawn {
            seat: 3,
            count: 4,
            reason: DrawReason::Stack,
            ..
        }
    )));
    assert_eq!(s.decision_player(), Some(0));
}

#[test]
fn stack_progressive_allows_w4_on_d2_blocks_d2_on_w4_and_disables_challenge() {
    let mut cfg = RuleConfig::preset("classic", 4).unwrap();
    cfg.hand_size = 2;
    cfg.stack_draw = StackDraw::Progressive;
    cfg.wild4_rule = Wild4Rule::Challenge; // R3.1.6: 開啟疊加時 Challenge 視為 Free
    cfg.uno_rule = UnoRule::Auto;

    let order = stack_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Green), Face::DrawTwo),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::DrawTwo),
                (Some(Color::Red), Face::Number(5)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Yellow), Face::Number(3)),
            ],
            vec![(Some(Color::Yellow), Face::DrawTwo), (None, Face::Wild)],
        ],
        (Some(Color::Red), Face::Number(9)),
    );

    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let d2 = s.hands[1][0];
    s.apply(1, Action::Play { card: d2 }).unwrap();
    assert_eq!(s.pending_draw, 2);
    assert_eq!(s.decision_player(), Some(2));

    // seat 2 用 WildDrawFour 疊在 +2 上，仍需 ChooseColor（R3.1.7），且不進 Challenge（R3.1.6）
    let w4 = s.hands[2][0];
    s.apply(2, Action::Play { card: w4 }).unwrap();
    assert_eq!(s.phase(), Phase::ChooseColor);
    assert_eq!(s.decision_player(), Some(2));
    s.apply(2, Action::ChooseColor { color: Color::Blue })
        .unwrap();
    assert_eq!(s.phase(), Phase::Play);
    assert_eq!(s.pending_draw, 6);
    assert_eq!(s.decision_player(), Some(3));

    // seat 3 手上的 DrawTwo 不能疊在 WildDrawFour 上（R3.1.3）
    assert_eq!(s.legal_actions(3), vec![Action::Draw]);
    s.apply(3, Action::Draw).unwrap();
    assert_eq!(s.hands[3].len(), 8); // 2 + 6
    assert_eq!(s.pending_draw, 0);
    assert_eq!(s.decision_player(), Some(0));

    // R3.1.6：無疊加中出 +4（pending_draw=0）在 stack 開啟時同樣不進 Challenge
    let order2 = stack_deck(
        &RuleConfig {
            hand_size: 2,
            stack_draw: StackDraw::Progressive,
            wild4_rule: Wild4Rule::Challenge,
            ..RuleConfig::preset("classic", 3).unwrap()
        },
        &[
            vec![
                (Some(Color::Red), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (None, Face::WildDrawFour),
                (Some(Color::Red), Face::Number(3)),
            ],
            vec![
                (Some(Color::Green), Face::Number(4)),
                (Some(Color::Yellow), Face::Number(5)),
            ],
        ],
        (Some(Color::Red), Face::Number(8)),
    );
    let mut cfg2 = RuleConfig::preset("classic", 3).unwrap();
    cfg2.hand_size = 2;
    cfg2.stack_draw = StackDraw::Progressive;
    cfg2.wild4_rule = Wild4Rule::Challenge;
    let mut s2 = GameState::new_with_deck(cfg2, order2).unwrap();
    let w4_2 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: w4_2 }).unwrap();
    s2.apply(
        1,
        Action::ChooseColor {
            color: Color::Green,
        },
    )
    .unwrap();
    assert_eq!(s2.phase(), Phase::Play);
    assert_eq!(s2.pending_draw, 4);
    assert_eq!(s2.decision_player(), Some(2));
}

#[test]
fn stack_no_mercy_draw_value_hierarchy_checks_can_stack() {
    let mut cfg = RuleConfig::preset("no_mercy", 2).unwrap();
    cfg.hand_size = 3;
    let order = stack_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::DrawTwo),
                (Some(Color::Green), Face::DrawFour),
                (None, Face::WildDrawTen),
            ],
            vec![
                (Some(Color::Red), Face::DrawFour),
                (None, Face::WildDrawSix),
                (Some(Color::Red), Face::Number(1)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
    );

    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    // seat 1 主動打出 DrawFour (pending_draw = 4) -> seat 0 可用 DrawFour(4) 與 WildDrawTen(10)，不可用 DrawTwo(2)
    let p1_d4 = s.hands[1][0];
    s.apply(1, Action::Play { card: p1_d4 }).unwrap();
    assert_eq!(s.pending_draw, 4);
    assert_eq!(s.decision_player(), Some(0));

    let legal0 = s.legal_actions(0);
    assert!(!legal0.contains(&Action::Play {
        card: s.hands[0][0]
    }));
    assert!(legal0.contains(&Action::Play {
        card: s.hands[0][1]
    }));
    assert!(legal0.contains(&Action::Play {
        card: s.hands[0][2]
    }));
}
