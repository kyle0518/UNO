use crate::card::{Card, CardId, Color, Face};
use crate::config::{RuleConfig, UnoRule};
use crate::deck::build_deck;
use crate::events::{Action, DrawReason, EliminationReason, Event, Phase};
use crate::rng::Rng;
use crate::state::GameState;

fn take_card(cards: &[Card], used: &mut [bool], color: Option<Color>, face: Face) -> CardId {
    let found = cards
        .iter()
        .find(|c| !used[usize::from(c.id)] && c.light.color == color && c.light.face == face)
        .expect("requested card must exist in unused cards")
        .id;
    used[usize::from(found)] = true;
    found
}

fn no_mercy_deck(
    cfg: &RuleConfig,
    hands_spec: &[Vec<(Option<Color>, Face)>],
    start_spec: (Option<Color>, Face),
    top_draw_specs: &[(Option<Color>, Face)],
) -> Vec<CardId> {
    let cards = build_deck(cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    let mut picked_hands: Vec<Vec<CardId>> = Vec::with_capacity(hands_spec.len());
    for spec in hands_spec {
        let mut h = Vec::with_capacity(spec.len());
        for &(c, f) in spec {
            h.push(take_card(&cards, &mut used, c, f));
        }
        picked_hands.push(h);
    }

    let start_id = take_card(&cards, &mut used, start_spec.0, start_spec.1);
    let mut top_draws = Vec::with_capacity(top_draw_specs.len());
    for &(c, f) in top_draw_specs {
        top_draws.push(take_card(&cards, &mut used, c, f));
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
            let cid = picked_hands
                .get(seat)
                .and_then(|v| v.get(round_idx))
                .copied()
                .unwrap_or_else(|| remaining.pop().unwrap());
            order.push(cid);
        }
    }
    order.push(start_id);
    order.extend(top_draws);
    order.extend(remaining);
    order
}

#[test]
fn no_mercy_r4_2_3_r4_2_4_wild_reverse_draw_four_six_ten_and_two_player() {
    let mut cfg = RuleConfig::preset("no_mercy", 4).unwrap();
    cfg.hand_size = 2;
    let order = no_mercy_deck(
        &cfg,
        &[
            // seat 0: WildDrawSix + Blue 2
            vec![
                (None, Face::WildDrawSix),
                (Some(Color::Blue), Face::Number(2)),
            ],
            // seat 1: WildReverseDrawFour + Red 1
            vec![
                (None, Face::WildReverseDrawFour),
                (Some(Color::Red), Face::Number(1)),
            ],
            // seat 2: Blue 3 + Yellow 4
            vec![
                (Some(Color::Blue), Face::Number(3)),
                (Some(Color::Yellow), Face::Number(4)),
            ],
            // seat 3: DrawFour + WildDrawTen
            vec![
                (Some(Color::Red), Face::DrawFour),
                (None, Face::WildDrawTen),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    assert_eq!(s.current, 1);

    let wrd4 = s.hands[1][0];
    s.apply(1, Action::Play { card: wrd4 }).unwrap();
    assert_eq!(s.phase, Phase::ChooseColor);
    let ev = s
        .apply(1, Action::ChooseColor { color: Color::Blue })
        .unwrap();
    assert!(ev.contains(&Event::DirectionChanged { direction: -1 }));
    assert!(ev.contains(&Event::PendingDrawChanged { amount: 4 }));
    assert_eq!(s.direction, -1);
    assert_eq!(s.pending_draw, 4);
    assert_eq!(s.current, 0); // 反轉後的下一位

    // seat 0 疊 WildDrawSix -> +10，換 seat 3
    let wd6 = s.hands[0][0];
    s.apply(0, Action::Play { card: wd6 }).unwrap();
    let ev = s
        .apply(
            0,
            Action::ChooseColor {
                color: Color::Green,
            },
        )
        .unwrap();
    assert!(ev.contains(&Event::PendingDrawChanged { amount: 10 }));
    assert_eq!(s.pending_draw, 10);
    assert_eq!(s.current, 3);

    // seat 3 的 DrawFour 價值 4 < 頂牌 WildDrawSix 價值 6，不能疊；只能疊 WildDrawTen
    let d4 = s.hands[3][0];
    let wd10 = s.hands[3][1];
    assert!(s.legal_actions(3).contains(&Action::Play { card: wd10 }));
    assert!(!s.legal_actions(3).contains(&Action::Play { card: d4 }));
    s.apply(3, Action::Play { card: wd10 }).unwrap();
    let ev = s
        .apply(
            3,
            Action::ChooseColor {
                color: Color::Yellow,
            },
        )
        .unwrap();
    assert!(ev.contains(&Event::PendingDrawChanged { amount: 20 }));
    assert_eq!(s.current, 2);

    // seat 2 無法疊，選擇 Draw 抽 20 張並被跳過，輪到 seat 1
    let ev = s.apply(2, Action::Draw).unwrap();
    assert!(ev.contains(&Event::PlayerSkipped { seat: 2 }));
    assert_eq!(s.hands[2].len(), 22);
    assert_eq!(s.pending_draw, 0);
    assert_eq!(s.current, 1);

    // 兩人局中出 WildReverseDrawFour：反轉方向但下一位仍是對手承受 +4
    let mut cfg2 = RuleConfig::preset("no_mercy", 2).unwrap();
    cfg2.hand_size = 2;
    let order2 = no_mercy_deck(
        &cfg2,
        &[
            vec![
                (Some(Color::Blue), Face::Number(2)),
                (Some(Color::Green), Face::Number(5)),
            ],
            vec![
                (None, Face::WildReverseDrawFour),
                (Some(Color::Red), Face::Number(1)),
            ],
        ],
        (Some(Color::Red), Face::Number(9)),
        &[],
    );
    let mut s2 = GameState::new_with_deck(cfg2, order2).unwrap();
    let c2 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: c2 }).unwrap();
    s2.apply(1, Action::ChooseColor { color: Color::Blue })
        .unwrap();
    assert_eq!(s2.direction, -1);
    assert_eq!(s2.pending_draw, 4);
    assert_eq!(s2.current, 0);
}

#[test]
fn no_mercy_r4_2_5_discard_all_discards_same_color_preserves_top_and_handles_uno() {
    let mut cfg = RuleConfig::preset("no_mercy", 2).unwrap();
    cfg.hand_size = 4;
    cfg.uno_rule = UnoRule::Manual;
    let order = no_mercy_deck(
        &cfg,
        &[
            // seat 0: 4 cards same green -> DiscardAll empties whole hand
            vec![
                (Some(Color::Green), Face::Number(1)),
                (Some(Color::Green), Face::Number(2)),
                (Some(Color::Green), Face::Skip),
                (Some(Color::Green), Face::DiscardAll),
            ],
            // seat 1: Red DiscardAll + Red 7 + Red 9 + Blue 3 -> DiscardAll leaves Blue 3 & opens UNO window
            vec![
                (Some(Color::Red), Face::DiscardAll),
                (Some(Color::Red), Face::Number(7)),
                (Some(Color::Red), Face::Number(9)),
                (Some(Color::Blue), Face::Number(3)),
            ],
        ],
        (Some(Color::Red), Face::Number(4)),
        &[],
    );
    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let discard_all_id = s.hands[1][0];
    let ev = s
        .apply(
            1,
            Action::Play {
                card: discard_all_id,
            },
        )
        .unwrap();

    // Red 7 / Red 9 一併棄掉，頂牌仍是 Red DiscardAll，且 Red 7 不觸發換手牌
    assert_eq!(s.top_card().id, discard_all_id);
    assert_eq!(s.hands[1].len(), 1);
    assert_eq!(s.card(s.hands[1][0]).light.color, Some(Color::Blue));
    assert_eq!(s.phase, Phase::Play);
    assert_eq!(s.current, 0);
    assert_eq!(s.uno_window, Some(1));
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::CardsDiscarded { seat: 1, cards } if cards.len() == 2)));

    // seat 0 全部都是 Green + Green DiscardAll，一牌清空不開 UNO 視窗直接獲勝
    let green_da = s.hands[0][3];
    let ev = s.apply(0, Action::Play { card: green_da }).unwrap();
    assert_eq!(s.hands[0].len(), 0);
    assert_eq!(s.phase, Phase::GameOver);
    assert_eq!(s.uno_window, None);
    assert_eq!(s.rankings.as_deref(), Some(&[0, 1][..]));
    assert!(!ev.iter().any(|e| matches!(e, Event::UnoCalled { seat: 0 })));
}

#[test]
fn no_mercy_r4_2_6_r4_2_7_wild_color_roulette_and_skip_everyone() {
    let mut cfg = RuleConfig::preset("no_mercy", 3).unwrap();
    cfg.hand_size = 2;
    let order = no_mercy_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Red), Face::Number(5)),
                (Some(Color::Blue), Face::Number(5)),
            ],
            vec![
                (Some(Color::Green), Face::SkipEveryone),
                (None, Face::WildColorRoulette),
            ],
            vec![
                (Some(Color::Blue), Face::Number(6)),
                (Some(Color::Yellow), Face::Number(7)),
            ],
        ],
        (Some(Color::Green), Face::Number(2)),
        &[
            (Some(Color::Red), Face::Number(3)),
            (Some(Color::Yellow), Face::Number(4)),
            (Some(Color::Green), Face::Number(8)),
        ],
    );
    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    assert_eq!(s.current, 1);

    // seat 1 出 SkipEveryone：其他人被跳過，自己再出一次
    let se = s.hands[1][0];
    let ev = s.apply(1, Action::Play { card: se }).unwrap();
    assert_eq!(s.current, 1);
    assert!(ev.contains(&Event::PlayerSkipped { seat: 2 }));
    assert!(ev.contains(&Event::PlayerSkipped { seat: 0 }));

    // seat 1 緊接著打出最後一張 WildColorRoulette 選綠色：
    // R4.2.6 + R2.8.1：seat 2 翻牌直到抽出 Green 8（共 3 張），被跳過，然後 seat 1 出完牌贏得遊戲
    let roulette = s.hands[1][0];
    s.apply(1, Action::Play { card: roulette }).unwrap();
    let ev = s
        .apply(
            1,
            Action::ChooseColor {
                color: Color::Green,
            },
        )
        .unwrap();
    assert_eq!(s.hands[2].len(), 5);
    assert_eq!(s.phase, Phase::GameOver);
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::CardsDrawn {
            seat: 2,
            count: 3,
            reason: DrawReason::Roulette,
            ..
        }
    )));
    assert!(ev.contains(&Event::PlayerSkipped { seat: 2 }));

    // 非最後一張 WildColorRoulette：victim 抽到指定色並被跳過，輪到下下一位（seat 0）
    let mut cfg2 = RuleConfig::preset("no_mercy", 3).unwrap();
    cfg2.hand_size = 2;
    let order2 = no_mercy_deck(
        &cfg2,
        &[
            vec![
                (Some(Color::Red), Face::Number(5)),
                (Some(Color::Blue), Face::Number(5)),
            ],
            vec![
                (None, Face::WildColorRoulette),
                (Some(Color::Red), Face::Number(1)),
            ],
            vec![
                (Some(Color::Blue), Face::Number(6)),
                (Some(Color::Yellow), Face::Number(7)),
            ],
        ],
        (Some(Color::Green), Face::Number(2)),
        &[
            (Some(Color::Red), Face::Number(3)),
            (Some(Color::Yellow), Face::Number(4)),
            (Some(Color::Green), Face::Number(8)),
        ],
    );
    let mut s2 = GameState::new_with_deck(cfg2, order2).unwrap();
    let roulette2 = s2.hands[1][0];
    s2.apply(1, Action::Play { card: roulette2 }).unwrap();
    s2.apply(
        1,
        Action::ChooseColor {
            color: Color::Green,
        },
    )
    .unwrap();
    assert_eq!(s2.hands[2].len(), 5);
    assert_eq!(s2.phase, Phase::Play);
    assert_eq!(s2.current, 0);
}

#[test]
fn no_mercy_r4_2_8_r4_2_9_mercy_25_elimination_bottom_shuffle_and_rankings() {
    let mut cfg = RuleConfig::preset("no_mercy", 3).unwrap();
    cfg.hand_size = 15;
    let cards = build_deck(&cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    let mut seat1_hand = vec![
        take_card(&cards, &mut used, None, Face::WildDrawTen),
        take_card(&cards, &mut used, Some(Color::Red), Face::Number(1)),
    ];
    let mut seat2_hand = Vec::new();
    let mut seat0_hand = Vec::new();
    let mut pool: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used[usize::from(id)])
        .collect();
    while seat1_hand.len() < 15 {
        seat1_hand.push(pool.pop().unwrap());
    }
    for _ in 0..15 {
        seat2_hand.push(pool.pop().unwrap());
    }
    for _ in 0..15 {
        seat0_hand.push(pool.pop().unwrap());
    }
    let raw_start = pool.pop().unwrap();
    let start_id = if cards[usize::from(raw_start)].light.color.is_none() {
        let pos = pool
            .iter()
            .position(|&id| cards[usize::from(id)].light.color.is_some())
            .unwrap();
        let alt = pool.remove(pos);
        pool.push(raw_start);
        alt
    } else {
        raw_start
    };

    let mut order = Vec::with_capacity(cards.len());
    for i in 0..15 {
        order.push(seat1_hand[i]);
        order.push(seat2_hand[i]);
        order.push(seat0_hand[i]);
    }
    order.push(start_id);
    order.extend(pool);

    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    assert_eq!(s.current, 1);
    let pile_len_before = s.draw_pile.len();
    // 抽走 10 張後新的牌堆頂端應維持不變，seat 2 的 25 張放在底部
    let expected_top_after_10 = s.draw_pile[pile_len_before - 11];

    let wd10 = s.hands[1][0];
    s.apply(1, Action::Play { card: wd10 }).unwrap();
    s.apply(1, Action::ChooseColor { color: Color::Red })
        .unwrap();
    assert_eq!(s.current, 2);
    assert_eq!(s.pending_draw, 10);

    // seat 2 接受 10 張成為 15 + 10 = 25 張 -> 立即觸發 Mercy 淘汰並跳到 seat 0
    let ev = s.apply(2, Action::Draw).unwrap();
    assert!(ev.contains(&Event::PlayerEliminated {
        seat: 2,
        reason: EliminationReason::Mercy
    }));
    assert!(s.eliminated[2]);
    assert!(s.hands[2].is_empty());
    assert_eq!(s.draw_pile.len(), pile_len_before - 10 + 25);
    assert_eq!(s.draw_pile.last().copied(), Some(expected_top_after_10));
    assert_eq!(s.phase, Phase::Play);
    assert_eq!(s.current, 0);

    let total = s.hands.iter().map(Vec::len).sum::<usize>() + s.draw_pile.len() + s.discard.len();
    assert_eq!(total, 168);

    // 兩人局 draw_to_match 第 25 張立即淘汰，對手自動勝出，名次為 [存活者, 淘汰者]
    let mut cfg3 = RuleConfig::preset("no_mercy", 2).unwrap();
    cfg3.hand_size = 15;
    let mut used3 = vec![false; cards.len()];
    let start3 = take_card(&cards, &mut used3, Some(Color::Blue), Face::Number(9));
    let unplayable: Vec<CardId> = cards
        .iter()
        .filter(|c| {
            !used3[usize::from(c.id)]
                && c.light.color.is_some()
                && c.light.color != Some(Color::Blue)
                && c.light.face != Face::Number(9)
        })
        .map(|c| c.id)
        .collect();
    assert!(unplayable.len() >= 25);
    let mut s1_h = Vec::with_capacity(15);
    for &cid in &unplayable[..15] {
        used3[usize::from(cid)] = true;
        s1_h.push(cid);
    }
    let mut next_draws = Vec::with_capacity(10);
    for &cid in &unplayable[15..25] {
        used3[usize::from(cid)] = true;
        next_draws.push(cid);
    }
    let mut rest: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used3[usize::from(id)])
        .collect();
    let mut order3 = Vec::with_capacity(cards.len());
    for &cid in &s1_h {
        order3.push(cid);
        order3.push(rest.pop().unwrap());
    }
    order3.push(start3);
    order3.extend(next_draws);
    order3.extend(rest);

    let mut s3 = GameState::new_with_deck(cfg3, order3).unwrap();
    assert_eq!(s3.hands[1].len(), 15);
    let ev3 = s3.apply(1, Action::Draw).unwrap();
    assert!(ev3.contains(&Event::PlayerEliminated {
        seat: 1,
        reason: EliminationReason::Mercy
    }));
    assert_eq!(s3.phase, Phase::GameOver);
    assert_eq!(s3.rankings.as_deref(), Some(&[0, 1][..]));
}
