use super::*;
use crate::card::{Card, Face, Side};
use crate::config::{RuleConfig, Wild4Rule};
use crate::deck::build_deck;
use crate::events::DrawReason;
use crate::rng::Rng;

fn pick_light(cards: &[Card], used: &mut [bool], color: Option<Color>, face: Face) -> CardId {
    let found = cards
        .iter()
        .find(|c| !used[usize::from(c.id)] && c.light.color == color && c.light.face == face)
        .expect("requested light card must exist")
        .id;
    used[usize::from(found)] = true;
    found
}

fn pick_with_both<F>(cards: &[Card], used: &mut [bool], pred: F) -> CardId
where
    F: Fn(&Card) -> bool,
{
    let found = cards
        .iter()
        .find(|c| !used[usize::from(c.id)] && pred(c))
        .expect("requested flip card pair must exist")
        .id;
    used[usize::from(found)] = true;
    found
}

fn flip_deck(
    cfg: &RuleConfig,
    hands_spec: &[Vec<(Option<Color>, Face)>],
    start_id: CardId,
    top_draws: &[CardId],
    used: &mut [bool],
    cards: &[Card],
) -> Vec<CardId> {
    let mut picked_hands = Vec::with_capacity(hands_spec.len());
    for spec in hands_spec {
        let mut h = Vec::new();
        for &(c, f) in spec {
            h.push(pick_light(cards, used, c, f));
        }
        picked_hands.push(h);
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
    order.extend_from_slice(top_draws);
    order.extend(remaining);
    order
}

#[test]
fn flip_r4_1_7_side_flip_reverses_draw_pile_updates_view_backs_and_wild_flip_top() {
    let mut cfg = RuleConfig::preset("flip", 4).unwrap();
    cfg.hand_size = 2;
    let cards = build_deck(&cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    // 找一張 Light=Red Flip、Dark=有色牌，再測試另一張 Dark=Wild 翻牌
    let red_flip_colored_back = pick_with_both(&cards, &mut used, |c| {
        c.light.color == Some(Color::Red)
            && c.light.face == Face::Flip
            && c.dark.expect("flip").color.is_some()
    });
    let expected_dark_color = cards[usize::from(red_flip_colored_back)]
        .dark
        .unwrap()
        .color
        .unwrap();

    let start_id = pick_light(&cards, &mut used, Some(Color::Red), Face::Number(5));
    let remaining_cards: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used[usize::from(id)])
        .collect();
    // 建 deck：手動把 red_flip_colored_back 放入 seat 1 第 0 張
    let mut order = Vec::with_capacity(cards.len());
    let n = usize::from(cfg.num_players);
    let h_size = usize::from(cfg.hand_size);
    let mut rem_iter = remaining_cards.into_iter();
    for round_idx in 0..h_size {
        for offset in 0..n {
            let seat = (1 + offset) % n;
            if seat == 1 && round_idx == 0 {
                order.push(red_flip_colored_back);
            } else {
                order.push(rem_iter.next().unwrap());
            }
        }
    }
    order.push(start_id);
    order.extend(rem_iter);

    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    assert_eq!(s.side, Side::Light);

    // R4.1.6: 亮面時 PlayerView.hand_backs 只含對手暗面，自己為 None；draw_pile_top_back 含暗面
    let v0_light = s.view(0);
    assert!(v0_light.players[0].hand_backs.is_none());
    let expected_p1_dark_backs: Vec<_> = s.hands[1]
        .iter()
        .map(|&id| s.card(id).dark.unwrap())
        .collect();
    assert_eq!(v0_light.players[1].hand_backs, Some(expected_p1_dark_backs));
    let expected_pile_top_back = s.card(*s.draw_pile.last().unwrap()).dark.unwrap();
    assert_eq!(v0_light.draw_pile_top_back, Some(expected_pile_top_back));
    // 確保對手正面不洩漏在 PlayerView 的 public players JSON 中
    let p1_pub_json = serde_json::to_string(&v0_light.players[1]).unwrap();
    assert!(!p1_pub_json.contains("Red"));

    // 記錄翻轉前的 draw_pile 底部與頂部
    let bottom_before = s.draw_pile[0];
    let top_before = *s.draw_pile.last().unwrap();

    let ev = s
        .apply(
            1,
            Action::Play {
                card: red_flip_colored_back,
            },
        )
        .unwrap();
    assert!(ev.contains(&Event::SideFlipped { side: Side::Dark }));
    assert_eq!(s.side, Side::Dark);
    assert_eq!(s.active_color, Some(expected_dark_color));
    assert_eq!(*s.draw_pile.last().unwrap(), bottom_before);
    assert_eq!(s.draw_pile[0], top_before);
    assert_eq!(s.decision_player(), Some(2));

    // 翻成暗面後，view(0).players[1].hand_backs 改成亮面！
    let v0_dark = s.view(0);
    let expected_p1_light_backs: Vec<_> = s.hands[1].iter().map(|&id| s.card(id).light).collect();
    assert_eq!(v0_dark.players[1].hand_backs, Some(expected_p1_light_backs));
    let expected_dark_pile_back = s.card(*s.draw_pile.last().unwrap()).light;
    assert_eq!(v0_dark.draw_pile_top_back, Some(expected_dark_pile_back));

    // 測試 Flip 牌背面為 WildDrawColor 時：需由出 Flip 的人選色，且「不觸發」WildDrawColor 抽牌效果！
    let mut used2 = vec![false; cards.len()];
    let flip_to_wild_color = pick_with_both(&cards, &mut used2, |c| {
        c.light.face == Face::Flip && c.dark.expect("flip").face == Face::WildDrawColor
    });
    let wild_flip_color = cards[usize::from(flip_to_wild_color)].light.color.unwrap();
    let start2 = pick_light(&cards, &mut used2, Some(wild_flip_color), Face::Number(2));
    let rem2: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used2[usize::from(id)])
        .collect();
    let mut rem2_iter = rem2.into_iter();
    let mut order2 = Vec::with_capacity(cards.len());
    for round_idx in 0..h_size {
        for offset in 0..n {
            let seat = (1 + offset) % n;
            if seat == 1 && round_idx == 0 {
                order2.push(flip_to_wild_color);
            } else {
                order2.push(rem2_iter.next().unwrap());
            }
        }
    }
    order2.push(start2);
    order2.extend(rem2_iter);

    let mut sw = GameState::new_with_deck(cfg, order2).unwrap();
    let p2_before = sw.hands[2].len();
    sw.apply(
        1,
        Action::Play {
            card: flip_to_wild_color,
        },
    )
    .unwrap();
    assert_eq!(sw.side, Side::Dark);
    assert_eq!(sw.phase(), Phase::ChooseColor);
    assert_eq!(sw.decision_player(), Some(1));
    sw.apply(1, Action::ChooseColor { color: Color::Pink })
        .unwrap();
    assert_eq!(sw.phase(), Phase::Play);
    assert_eq!(sw.decision_player(), Some(2));
    assert_eq!(sw.hands[2].len(), p2_before); // WildDrawColor 功能未觸發！
}

#[test]
fn flip_light_draw_one_and_wild_draw_two_challenge() {
    let mut cfg = RuleConfig::preset("flip", 4).unwrap();
    cfg.hand_size = 2;
    cfg.wild4_rule = Wild4Rule::Challenge;
    let cards = build_deck(&cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    let start_id = pick_light(&cards, &mut used, Some(Color::Red), Face::Number(9));
    // seat 1 持有 Red DrawOne 與 WildDrawTwo，無 Blue；測試 DrawOne 後再測 WildDrawTwo Bluff 與 Honest
    let order = flip_deck(
        &cfg,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Blue), Face::Number(2)),
            ],
            vec![
                (Some(Color::Red), Face::DrawOne),
                (Some(Color::Green), Face::Number(4)),
            ],
            vec![
                (None, Face::WildDrawTwo),
                (Some(Color::Red), Face::Number(6)), // 對 Red Bluff！
            ],
            vec![
                (None, Face::WildDrawTwo),
                (Some(Color::Green), Face::Number(8)), // 對 Blue Honest！
            ],
        ],
        start_id,
        &[],
        &mut used,
        &cards,
    );

    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    let d1 = s.hands[1][0];
    s.apply(1, Action::Play { card: d1 }).unwrap();
    assert_eq!(s.hands[2].len(), 3); // seat 2 抽 1 張並被跳過
    assert_eq!(s.decision_player(), Some(3));

    // seat 3 出 WildDrawTwo（手上無 Red -> Honest），選 Blue，seat 0 質疑失敗應抽 4 張（2+2）
    let wd2_s3 = s.hands[3][0];
    s.apply(3, Action::Play { card: wd2_s3 }).unwrap();
    s.apply(3, Action::ChooseColor { color: Color::Blue })
        .unwrap();
    assert_eq!(s.phase(), Phase::Challenge);
    assert_eq!(s.decision_player(), Some(0));
    let ev = s.apply(0, Action::Challenge).unwrap();
    assert!(ev.contains(&Event::ChallengeResolved {
        challenger: 0,
        offender: 3,
        success: false
    }));
    assert_eq!(s.hands[0].len(), 6); // 2 + 4
    assert_eq!(s.decision_player(), Some(1));

    // 再讓 seat 1 Pass/Draw？直接建 Bluff 局驗證質疑成功罰抽 2 張
    let mut cfg_b = RuleConfig::preset("flip", 3).unwrap();
    cfg_b.hand_size = 2;
    cfg_b.wild4_rule = Wild4Rule::Challenge;
    let cards_b = build_deck(&cfg_b, &mut Rng::new(0));
    let mut used_b = vec![false; cards_b.len()];
    let start_b = pick_light(&cards_b, &mut used_b, Some(Color::Red), Face::Number(7));
    let order_b = flip_deck(
        &cfg_b,
        &[
            vec![
                (Some(Color::Blue), Face::Number(1)),
                (Some(Color::Green), Face::Number(2)),
            ],
            vec![
                (None, Face::WildDrawTwo),
                (Some(Color::Red), Face::Number(3)),
            ],
        ],
        start_b,
        &[],
        &mut used_b,
        &cards_b,
    );
    let mut sb = GameState::new_with_deck(cfg_b, order_b).unwrap();
    let wd2_b = sb.hands[1][0];
    sb.apply(1, Action::Play { card: wd2_b }).unwrap();
    sb.apply(
        1,
        Action::ChooseColor {
            color: Color::Green,
        },
    )
    .unwrap();
    let ev_b = sb.apply(2, Action::Challenge).unwrap();
    assert!(ev_b.contains(&Event::ChallengeResolved {
        challenger: 2,
        offender: 1,
        success: true
    }));
    assert_eq!(sb.hands[1].len(), 3); // 1 + 2 罰抽
    assert_eq!(sb.hands[2].len(), 2);
    assert_eq!(sb.decision_player(), Some(2));
}

#[test]
fn flip_dark_draw_five_skip_everyone_and_wild_draw_color_challenge() {
    let mut cfg = RuleConfig::preset("flip", 4).unwrap();
    cfg.hand_size = 2;
    cfg.wild4_rule = Wild4Rule::Challenge;
    let cards = build_deck(&cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    // 起始為 Light Flip，翻面後暗面有色牌 -> 直接在 Dark 面開局！
    let start_flip = pick_with_both(&cards, &mut used, |c| {
        c.light.face == Face::Flip && c.dark.expect("flip").color == Some(Color::Pink)
    });

    // 給 seat 1：Dark SkipEveryone(Pink) + Dark DrawFive(Pink)
    let s1_skip_all = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").color == Some(Color::Pink)
            && c.dark.expect("flip").face == Face::SkipEveryone
    });
    let s1_draw5 = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").color == Some(Color::Pink)
            && c.dark.expect("flip").face == Face::DrawFive
    });
    // 給 seat 3：Dark WildDrawColor + Dark NonPink (Teal) -> Honest
    let s3_wdc = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").face == Face::WildDrawColor
    });
    let s3_teal = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").color == Some(Color::Teal)
            && matches!(c.dark.expect("flip").face, Face::Number(_))
    });

    // 控制暗面翻轉後牌堆頂依序為 [Purple, Orange, Teal] (注意起始 Flip 會 reverse draw_pile！)
    let draw_p1 = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").color == Some(Color::Purple)
    });
    let draw_p2 = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").color == Some(Color::Orange)
    });
    let draw_teal = pick_with_both(&cards, &mut used, |c| {
        c.dark.expect("flip").color == Some(Color::Teal)
    });

    let mut rem: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used[usize::from(id)])
        .collect();
    let mut rem_iter = rem.drain(..);

    let mut order = Vec::with_capacity(cards.len());
    // hand_size = 2，發牌 1,2,3,0
    for r in 0..2 {
        for offset in 0..4 {
            let seat = (1 + offset) % 4;
            match (seat, r) {
                (1, 0) => order.push(s1_skip_all),
                (1, 1) => order.push(s1_draw5),
                (3, 0) => order.push(s3_wdc),
                (3, 1) => order.push(s3_teal),
                _ => order.push(rem_iter.next().unwrap()),
            }
        }
    }
    order.push(start_flip);
    // setup_round 從後面 pop 當作 draw_pile，但開局 Flip 會 reverse() draw_pile！
    // 原本 order 後段被 reversed 成 draw_pile（所以最後三個元素在 order 中是牌堆底），
    // 開局 Flip reverse() 後，「order 最尾端」會反轉變成新的「draw_pile 頂端」！
    order.extend(rem_iter);
    order.push(draw_teal);
    order.push(draw_p2);
    order.push(draw_p1);

    let mut s = GameState::new_with_deck(cfg, order).unwrap();
    assert_eq!(s.side, Side::Dark);
    assert_eq!(s.active_color, Some(Color::Pink));
    assert_eq!(s.decision_player(), Some(1));

    // SkipEveryone：跳過 2,3,0，由 seat 1 再出一次
    let ev_sa = s.apply(1, Action::Play { card: s1_skip_all }).unwrap();
    assert!(ev_sa.contains(&Event::PlayerSkipped { seat: 2 }));
    assert!(ev_sa.contains(&Event::PlayerSkipped { seat: 3 }));
    assert!(ev_sa.contains(&Event::PlayerSkipped { seat: 0 }));
    assert_eq!(s.decision_player(), Some(1));

    // seat 1 最後一張打 DrawFive (Pink) -> seat 2 先抽 5 張（R2.8.1），seat 1 贏得該輪（R4.1.10 暗面計分）
    let ev_end = s.apply(1, Action::Play { card: s1_draw5 }).unwrap();
    assert_eq!(s.hands[2].len(), 7); // 2 + 5
    assert_eq!(s.phase(), Phase::RoundOver);
    assert!(ev_end.iter().any(|e| matches!(
        e,
        Event::RoundEnded {
            winner: Some(1),
            ..
        }
    )));

    // 測試 WildDrawColor 質疑失敗（一直抽到該色為止 + 額外 2 張）
    let mut cfg2 = RuleConfig::preset("flip", 4).unwrap();
    cfg2.hand_size = 2;
    cfg2.wild4_rule = Wild4Rule::Challenge;
    let cards2 = build_deck(&cfg2, &mut Rng::new(0));
    let mut used2 = vec![false; cards2.len()];
    let start_flip2 = pick_with_both(&cards2, &mut used2, |c| {
        c.light.face == Face::Flip && c.dark.expect("flip").color == Some(Color::Pink)
    });
    let s1_wdc2 = pick_with_both(&cards2, &mut used2, |c| {
        c.dark.expect("flip").face == Face::WildDrawColor
    });
    let s1_other2 = pick_with_both(&cards2, &mut used2, |c| {
        c.dark.expect("flip").color == Some(Color::Teal)
    });
    let c_purple = pick_with_both(&cards2, &mut used2, |c| {
        c.dark.expect("flip").color == Some(Color::Purple)
    });
    let c_orange = pick_with_both(&cards2, &mut used2, |c| {
        c.dark.expect("flip").color == Some(Color::Orange)
    });
    let c_teal = pick_with_both(&cards2, &mut used2, |c| {
        c.dark.expect("flip").color == Some(Color::Teal)
    });
    let c_extra1 = pick_with_both(&cards2, &mut used2, |_| true);
    let c_extra2 = pick_with_both(&cards2, &mut used2, |_| true);
    let mut rem2: Vec<CardId> = cards2
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used2[usize::from(id)])
        .collect();
    let mut rem2_iter = rem2.drain(..);
    let mut order2 = Vec::with_capacity(cards2.len());
    for r in 0..2 {
        for offset in 0..4 {
            let seat = (1 + offset) % 4;
            match (seat, r) {
                (1, 0) => order2.push(s1_wdc2),
                (1, 1) => order2.push(s1_other2),
                _ => order2.push(rem2_iter.next().unwrap()),
            }
        }
    }
    order2.push(start_flip2);
    order2.extend(rem2_iter);
    order2.push(c_extra2);
    order2.push(c_extra1);
    order2.push(c_teal);
    order2.push(c_orange);
    order2.push(c_purple);

    let mut swc = GameState::new_with_deck(cfg2, order2).unwrap();
    swc.apply(1, Action::Play { card: s1_wdc2 }).unwrap();
    swc.apply(1, Action::ChooseColor { color: Color::Teal })
        .unwrap();
    assert_eq!(swc.phase(), Phase::Challenge);
    assert_eq!(swc.decision_player(), Some(2));
    // seat 1 手牌只有 Teal（沒有 Pink）-> Honest，seat 2 質疑失敗：抽 Purple, Orange, Teal (3 張 DrawColor) + 額外 2 張 = 共 +5 張！
    let ev_wc = swc.apply(2, Action::Challenge).unwrap();
    assert!(ev_wc.contains(&Event::ChallengeResolved {
        challenger: 2,
        offender: 1,
        success: false
    }));
    assert!(ev_wc.iter().any(|e| matches!(
        e,
        Event::CardsDrawn {
            seat: 2,
            count: 3,
            reason: DrawReason::DrawColor,
            ..
        }
    )));
    assert!(ev_wc.iter().any(|e| matches!(
        e,
        Event::CardsDrawn {
            seat: 2,
            count: 2,
            reason: DrawReason::Challenge,
            ..
        }
    )));
    assert_eq!(swc.hands[2].len(), 7); // 2 + 3 + 2
    assert_eq!(swc.decision_player(), Some(3));
}

#[test]
fn flip_r4_1_12_start_card_flip_wild_and_wild_draw_two_reshuffle() {
    let mut cfg = RuleConfig::preset("flip", 4).unwrap();
    cfg.hand_size = 2;
    let cards = build_deck(&cfg, &mut Rng::new(0));
    let mut used = vec![false; cards.len()];

    // Flip 起始牌翻面後是 Wild -> 莊家 (seat 0) 選色，再輪 first (seat 1) 出牌
    let start_flip_wild = pick_with_both(&cards, &mut used, |c| {
        c.light.face == Face::Flip && c.dark.expect("flip").face == Face::WildDrawColor
    });
    let rem: Vec<CardId> = cards
        .iter()
        .map(|c| c.id)
        .filter(|&id| !used[usize::from(id)])
        .collect();
    let mut rem_iter = rem.into_iter();
    let mut order = Vec::with_capacity(cards.len());
    for _ in 0..8 {
        order.push(rem_iter.next().unwrap());
    }
    order.push(start_flip_wild);
    order.extend(rem_iter);

    let mut s = GameState::new_with_deck(cfg.clone(), order).unwrap();
    assert_eq!(s.side, Side::Dark);
    assert_eq!(s.phase(), Phase::ChooseColor);
    assert_eq!(s.decision_player(), Some(0)); // 莊家選色
    s.apply(
        0,
        Action::ChooseColor {
            color: Color::Orange,
        },
    )
    .unwrap();
    assert_eq!(s.phase(), Phase::Play);
    assert_eq!(s.decision_player(), Some(1)); // first 開始，且 WildDrawColor 未觸發！
    assert_eq!(s.hands[1].len(), 2);

    // 正常 GameState::new 絕不會出現 WildDrawTwo 作為未重翻的起始牌
    for seed in 0..300 {
        let g = GameState::new(cfg.clone(), seed).unwrap();
        assert_ne!(g.top_card().light.face, Face::WildDrawTwo);
    }
}
