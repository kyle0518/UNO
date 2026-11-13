use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::ModeKind;

pub type Seat = u8;
pub type CardId = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
    Pink,
    Teal,
    Orange,
    Purple,
}

impl Color {
    /// 回傳顏色槽索引（0..=3）：Red/Pink=0, Yellow/Teal=1, Green/Orange=2, Blue/Purple=3。
    pub const fn slot(self) -> u8 {
        match self {
            Self::Red | Self::Pink => 0,
            Self::Yellow | Self::Teal => 1,
            Self::Green | Self::Orange => 2,
            Self::Blue | Self::Purple => 3,
        }
    }

    /// 依目前面與顏色槽（0..=3）取得實際顏色。
    pub const fn from_slot(slot: u8, side: Side) -> Option<Self> {
        match (side, slot) {
            (Side::Light, 0) => Some(Self::Red),
            (Side::Light, 1) => Some(Self::Yellow),
            (Side::Light, 2) => Some(Self::Green),
            (Side::Light, 3) => Some(Self::Blue),
            (Side::Dark, 0) => Some(Self::Pink),
            (Side::Dark, 1) => Some(Self::Teal),
            (Side::Dark, 2) => Some(Self::Orange),
            (Side::Dark, 3) => Some(Self::Purple),
            _ => None,
        }
    }

    /// 取得該面可用的四種顏色（依槽位 0..=3 順序）。
    pub const fn colors_for_side(side: Side) -> [Self; 4] {
        match side {
            Side::Light => [Self::Red, Self::Yellow, Self::Green, Self::Blue],
            Side::Dark => [Self::Pink, Self::Teal, Self::Orange, Self::Purple],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "k", content = "v", rename_all = "snake_case")]
#[ts(export)]
pub enum Face {
    Number(u8),
    Skip,
    Reverse,
    DrawTwo,
    Wild,
    WildDrawFour,
    // Flip
    DrawOne,
    Flip,
    WildDrawTwo,
    DrawFive,
    SkipEveryone,
    WildDrawColor,
    // No Mercy
    DrawFour,
    DiscardAll,
    WildReverseDrawFour,
    WildDrawSix,
    WildDrawTen,
    WildColorRoulette,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CardFace {
    pub color: Option<Color>,
    pub face: Face,
}

impl CardFace {
    pub const fn new(color: Option<Color>, face: Face) -> Self {
        Self { color, face }
    }

    pub const fn is_wild(&self) -> bool {
        self.color.is_none()
    }

    /// 回傳抽牌卡的抽牌張數；非抽牌卡（或不定張數如 WildDrawColor / WildColorRoulette）回傳 None。
    pub const fn draw_value(&self) -> Option<u16> {
        match self.face {
            Face::DrawOne => Some(1),
            Face::DrawTwo | Face::WildDrawTwo => Some(2),
            Face::DrawFour | Face::WildDrawFour | Face::WildReverseDrawFour => Some(4),
            Face::DrawFive => Some(5),
            Face::WildDrawSix => Some(6),
            Face::WildDrawTen => Some(10),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Card {
    pub id: CardId,
    pub light: CardFace,
    pub dark: Option<CardFace>,
}

impl Card {
    /// 取得該牌在指定面朝上時的牌面。
    pub fn face(&self, side: Side) -> CardFace {
        match side {
            Side::Light => self.light,
            Side::Dark => self
                .dark
                .expect("flip dark face must exist when side is Dark"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Side {
    Light,
    Dark,
}

impl Side {
    pub const fn flip(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

/// 計算單張牌面在指定模式與牌面方向下的計分點數（RULES §2.9.1 / §4.1.10 / §4.2.9）。
pub fn points(face: &CardFace, mode: ModeKind, side: Side) -> u32 {
    if let Face::Number(n) = face.face {
        return u32::from(n);
    }
    match mode {
        ModeKind::Classic => match face.face {
            Face::Skip | Face::Reverse | Face::DrawTwo => 20,
            Face::Wild | Face::WildDrawFour => 50,
            _ if face.color.is_some() => 20,
            _ => 50,
        },
        ModeKind::Flip => match side {
            Side::Light => match face.face {
                Face::DrawOne => 10,
                Face::Reverse | Face::Skip | Face::Flip => 20,
                Face::Wild => 40,
                Face::WildDrawTwo => 50,
                _ if face.color.is_some() => 20,
                _ => 50,
            },
            Side::Dark => match face.face {
                Face::DrawFive | Face::Reverse | Face::SkipEveryone | Face::Flip => 20,
                Face::Wild => 40,
                Face::WildDrawColor => 60,
                _ if face.color.is_some() => 20,
                _ => 60,
            },
        },
        ModeKind::NoMercy => {
            if face.color.is_some() {
                20
            } else {
                50
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r2_9_1_classic_card_points() {
        for n in 0..=9 {
            let cf = CardFace::new(Some(Color::Red), Face::Number(n));
            assert_eq!(points(&cf, ModeKind::Classic, Side::Light), u32::from(n));
        }
        for f in [Face::Skip, Face::Reverse, Face::DrawTwo] {
            let cf = CardFace::new(Some(Color::Blue), f);
            assert_eq!(points(&cf, ModeKind::Classic, Side::Light), 20);
        }
        for f in [Face::Wild, Face::WildDrawFour] {
            let cf = CardFace::new(None, f);
            assert_eq!(points(&cf, ModeKind::Classic, Side::Light), 50);
        }
    }

    #[test]
    fn r4_1_10_flip_card_points() {
        assert_eq!(
            points(
                &CardFace::new(Some(Color::Green), Face::Number(7)),
                ModeKind::Flip,
                Side::Light
            ),
            7
        );
        assert_eq!(
            points(
                &CardFace::new(Some(Color::Green), Face::DrawOne),
                ModeKind::Flip,
                Side::Light
            ),
            10
        );
        for f in [Face::Reverse, Face::Skip, Face::Flip] {
            assert_eq!(
                points(
                    &CardFace::new(Some(Color::Yellow), f),
                    ModeKind::Flip,
                    Side::Light
                ),
                20
            );
        }
        assert_eq!(
            points(
                &CardFace::new(None, Face::Wild),
                ModeKind::Flip,
                Side::Light
            ),
            40
        );
        assert_eq!(
            points(
                &CardFace::new(None, Face::WildDrawTwo),
                ModeKind::Flip,
                Side::Light
            ),
            50
        );

        for f in [
            Face::DrawFive,
            Face::Reverse,
            Face::SkipEveryone,
            Face::Flip,
        ] {
            assert_eq!(
                points(
                    &CardFace::new(Some(Color::Pink), f),
                    ModeKind::Flip,
                    Side::Dark
                ),
                20
            );
        }
        assert_eq!(
            points(&CardFace::new(None, Face::Wild), ModeKind::Flip, Side::Dark),
            40
        );
        assert_eq!(
            points(
                &CardFace::new(None, Face::WildDrawColor),
                ModeKind::Flip,
                Side::Dark
            ),
            60
        );
    }

    #[test]
    fn r4_2_9_no_mercy_card_points() {
        assert_eq!(
            points(
                &CardFace::new(Some(Color::Red), Face::Number(0)),
                ModeKind::NoMercy,
                Side::Light
            ),
            0
        );
        for f in [
            Face::DrawTwo,
            Face::DrawFour,
            Face::Skip,
            Face::SkipEveryone,
            Face::Reverse,
            Face::DiscardAll,
        ] {
            assert_eq!(
                points(
                    &CardFace::new(Some(Color::Blue), f),
                    ModeKind::NoMercy,
                    Side::Light
                ),
                20
            );
        }
        for f in [
            Face::WildReverseDrawFour,
            Face::WildDrawSix,
            Face::WildDrawTen,
            Face::WildColorRoulette,
        ] {
            assert_eq!(
                points(&CardFace::new(None, f), ModeKind::NoMercy, Side::Light),
                50
            );
        }
    }

    #[test]
    fn color_slots_and_card_helpers() {
        assert_eq!(Color::Red.slot(), 0);
        assert_eq!(Color::Pink.slot(), 0);
        assert_eq!(Color::Yellow.slot(), 1);
        assert_eq!(Color::Teal.slot(), 1);
        assert_eq!(Color::Green.slot(), 2);
        assert_eq!(Color::Orange.slot(), 2);
        assert_eq!(Color::Blue.slot(), 3);
        assert_eq!(Color::Purple.slot(), 3);

        assert_eq!(
            CardFace::new(Some(Color::Red), Face::DrawTwo).draw_value(),
            Some(2)
        );
        assert_eq!(
            CardFace::new(None, Face::WildDrawTen).draw_value(),
            Some(10)
        );
        assert_eq!(
            CardFace::new(Some(Color::Red), Face::Skip).draw_value(),
            None
        );
        assert!(CardFace::new(None, Face::Wild).is_wild());
        assert!(!CardFace::new(Some(Color::Red), Face::Number(1)).is_wild());

        let c = Card {
            id: 5,
            light: CardFace::new(Some(Color::Red), Face::Number(3)),
            dark: Some(CardFace::new(Some(Color::Pink), Face::DrawFive)),
        };
        assert_eq!(c.face(Side::Light).face, Face::Number(3));
        assert_eq!(c.face(Side::Dark).face, Face::DrawFive);
    }
}
