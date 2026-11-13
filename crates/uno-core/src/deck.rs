use crate::card::{Card, CardFace, Color, Face, Side};
use crate::config::{ModeKind, RuleConfig};
use crate::rng::Rng;

fn push_n(out: &mut Vec<CardFace>, color: Option<Color>, face: Face, count: usize) {
    for _ in 0..count {
        out.push(CardFace::new(color, face));
    }
}

fn build_classic_faces(deck_count: u8) -> Vec<CardFace> {
    let mut faces = Vec::with_capacity(108 * usize::from(deck_count));
    for _ in 0..deck_count {
        for color in Color::colors_for_side(Side::Light) {
            push_n(&mut faces, Some(color), Face::Number(0), 1);
            for n in 1..=9 {
                push_n(&mut faces, Some(color), Face::Number(n), 2);
            }
            push_n(&mut faces, Some(color), Face::Skip, 2);
            push_n(&mut faces, Some(color), Face::Reverse, 2);
            push_n(&mut faces, Some(color), Face::DrawTwo, 2);
        }
        push_n(&mut faces, None, Face::Wild, 4);
        push_n(&mut faces, None, Face::WildDrawFour, 4);
    }
    faces
}

fn build_flip_faces(deck_count: u8) -> (Vec<CardFace>, Vec<CardFace>) {
    let cap = 112 * usize::from(deck_count);
    let mut light = Vec::with_capacity(cap);
    let mut dark = Vec::with_capacity(cap);
    for _ in 0..deck_count {
        for color in Color::colors_for_side(Side::Light) {
            for n in 1..=9 {
                push_n(&mut light, Some(color), Face::Number(n), 2);
            }
            push_n(&mut light, Some(color), Face::DrawOne, 2);
            push_n(&mut light, Some(color), Face::Reverse, 2);
            push_n(&mut light, Some(color), Face::Skip, 2);
            push_n(&mut light, Some(color), Face::Flip, 2);
        }
        push_n(&mut light, None, Face::Wild, 4);
        push_n(&mut light, None, Face::WildDrawTwo, 4);

        for color in Color::colors_for_side(Side::Dark) {
            for n in 1..=9 {
                push_n(&mut dark, Some(color), Face::Number(n), 2);
            }
            push_n(&mut dark, Some(color), Face::DrawFive, 2);
            push_n(&mut dark, Some(color), Face::Reverse, 2);
            push_n(&mut dark, Some(color), Face::SkipEveryone, 2);
            push_n(&mut dark, Some(color), Face::Flip, 2);
        }
        push_n(&mut dark, None, Face::Wild, 4);
        push_n(&mut dark, None, Face::WildDrawColor, 4);
    }
    (light, dark)
}

fn build_no_mercy_faces(deck_count: u8) -> Vec<CardFace> {
    let mut faces = Vec::with_capacity(168 * usize::from(deck_count));
    for _ in 0..deck_count {
        for color in Color::colors_for_side(Side::Light) {
            for n in 0..=9 {
                push_n(&mut faces, Some(color), Face::Number(n), 2);
            }
            push_n(&mut faces, Some(color), Face::DrawTwo, 3);
            push_n(&mut faces, Some(color), Face::DrawFour, 2);
            push_n(&mut faces, Some(color), Face::Skip, 3);
            push_n(&mut faces, Some(color), Face::SkipEveryone, 2);
            push_n(&mut faces, Some(color), Face::Reverse, 3);
            push_n(&mut faces, Some(color), Face::DiscardAll, 3);
        }
        push_n(&mut faces, None, Face::WildReverseDrawFour, 8);
        push_n(&mut faces, None, Face::WildDrawSix, 4);
        push_n(&mut faces, None, Face::WildDrawTen, 4);
        push_n(&mut faces, None, Face::WildColorRoulette, 8);
    }
    faces
}

/// 依設定與決定性 Rng 建立牌組；CardId 從 0 連續編號（RULES §2.1 / §4.1.2-4.1.4 / §4.2.1）。
pub fn build_deck(config: &RuleConfig, rng: &mut Rng) -> Vec<Card> {
    match config.mode {
        ModeKind::Classic => build_classic_faces(config.deck_count)
            .into_iter()
            .enumerate()
            .map(|(i, light)| Card {
                id: i as u16,
                light,
                dark: None,
            })
            .collect(),
        ModeKind::Flip => {
            let (light, mut dark) = build_flip_faces(config.deck_count);
            rng.shuffle(&mut dark);
            light
                .into_iter()
                .zip(dark)
                .enumerate()
                .map(|(i, (l, d))| Card {
                    id: i as u16,
                    light: l,
                    dark: Some(d),
                })
                .collect()
        }
        ModeKind::NoMercy => build_no_mercy_faces(config.deck_count)
            .into_iter()
            .enumerate()
            .map(|(i, light)| Card {
                id: i as u16,
                light,
                dark: None,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn count_faces(
        iter: impl Iterator<Item = CardFace>,
    ) -> BTreeMap<(Option<u8>, &'static str), usize> {
        let mut map = BTreeMap::new();
        for cf in iter {
            let c = cf.color.map(|c| c as u8);
            let k = match cf.face {
                Face::Number(n) => match n {
                    0 => "0",
                    1 => "1",
                    2 => "2",
                    3 => "3",
                    4 => "4",
                    5 => "5",
                    6 => "6",
                    7 => "7",
                    8 => "8",
                    9 => "9",
                    _ => "other",
                },
                Face::Skip => "skip",
                Face::Reverse => "reverse",
                Face::DrawTwo => "draw2",
                Face::Wild => "wild",
                Face::WildDrawFour => "wild4",
                Face::DrawOne => "draw1",
                Face::Flip => "flip",
                Face::WildDrawTwo => "wild_draw2",
                Face::DrawFive => "draw5",
                Face::SkipEveryone => "skip_all",
                Face::WildDrawColor => "wild_draw_color",
                Face::DrawFour => "draw4",
                Face::DiscardAll => "discard_all",
                Face::WildReverseDrawFour => "wild_rev_draw4",
                Face::WildDrawSix => "wild_draw6",
                Face::WildDrawTen => "wild_draw10",
                Face::WildColorRoulette => "wild_roulette",
            };
            *map.entry((c, k)).or_insert(0) += 1;
        }
        map
    }

    #[test]
    fn r2_1_classic_deck_counts_and_ids() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let mut rng = Rng::new(1);
        let deck = build_deck(&cfg, &mut rng);
        assert_eq!(deck.len(), 108);
        for (idx, c) in deck.iter().enumerate() {
            assert_eq!(usize::from(c.id), idx);
            assert!(c.dark.is_none());
        }
        let counts = count_faces(deck.iter().map(|c| c.light));
        assert_eq!(counts[&(Some(Color::Red as u8), "0")], 1);
        assert_eq!(counts[&(Some(Color::Red as u8), "5")], 2);
        assert_eq!(counts[&(Some(Color::Blue as u8), "skip")], 2);
        assert_eq!(counts[&(None, "wild")], 4);
        assert_eq!(counts[&(None, "wild4")], 4);

        let mut cfg2 = cfg;
        cfg2.deck_count = 2;
        let deck2 = build_deck(&cfg2, &mut rng);
        assert_eq!(deck2.len(), 216);
        for (idx, c) in deck2.iter().enumerate() {
            assert_eq!(usize::from(c.id), idx);
        }
    }

    #[test]
    fn r4_1_flip_deck_counts_and_deterministic_pairing() {
        let cfg = RuleConfig::preset("flip", 4).unwrap();
        let d1 = build_deck(&cfg, &mut Rng::new(99));
        let d2 = build_deck(&cfg, &mut Rng::new(99));
        let d3 = build_deck(&cfg, &mut Rng::new(100));
        assert_eq!(d1.len(), 112);
        assert_eq!(d1, d2);
        assert_ne!(d1, d3);
        for (idx, c) in d1.iter().enumerate() {
            assert_eq!(usize::from(c.id), idx);
            assert!(c.dark.is_some());
        }
        let light_counts = count_faces(d1.iter().map(|c| c.light));
        assert!(!light_counts.contains_key(&(Some(Color::Red as u8), "0")));
        assert_eq!(light_counts[&(Some(Color::Red as u8), "1")], 2);
        assert_eq!(light_counts[&(Some(Color::Red as u8), "draw1")], 2);
        assert_eq!(light_counts[&(Some(Color::Green as u8), "flip")], 2);
        assert_eq!(light_counts[&(None, "wild")], 4);
        assert_eq!(light_counts[&(None, "wild_draw2")], 4);

        let dark_counts = count_faces(d1.iter().map(|c| c.dark.unwrap()));
        assert_eq!(dark_counts[&(Some(Color::Pink as u8), "9")], 2);
        assert_eq!(dark_counts[&(Some(Color::Teal as u8), "draw5")], 2);
        assert_eq!(dark_counts[&(Some(Color::Orange as u8), "skip_all")], 2);
        assert_eq!(dark_counts[&(Some(Color::Purple as u8), "flip")], 2);
        assert_eq!(dark_counts[&(None, "wild")], 4);
        assert_eq!(dark_counts[&(None, "wild_draw_color")], 4);
    }

    #[test]
    fn r4_2_1_no_mercy_deck_counts() {
        let cfg = RuleConfig::preset("no_mercy", 4).unwrap();
        let deck = build_deck(&cfg, &mut Rng::new(7));
        assert_eq!(deck.len(), 168);
        for (idx, c) in deck.iter().enumerate() {
            assert_eq!(usize::from(c.id), idx);
            assert!(c.dark.is_none());
        }
        let counts = count_faces(deck.iter().map(|c| c.light));
        assert_eq!(counts[&(Some(Color::Red as u8), "0")], 2);
        assert_eq!(counts[&(Some(Color::Yellow as u8), "draw2")], 3);
        assert_eq!(counts[&(Some(Color::Green as u8), "draw4")], 2);
        assert_eq!(counts[&(Some(Color::Blue as u8), "discard_all")], 3);
        assert_eq!(counts[&(Some(Color::Red as u8), "skip_all")], 2);
        assert_eq!(counts[&(None, "wild_rev_draw4")], 8);
        assert_eq!(counts[&(None, "wild_draw6")], 4);
        assert_eq!(counts[&(None, "wild_draw10")], 4);
        assert_eq!(counts[&(None, "wild_roulette")], 8);
    }
}
