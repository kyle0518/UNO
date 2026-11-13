use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::card::{Card, CardId, Color, Seat, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Phase {
    Play,
    ChooseColor,
    ChooseTarget,
    Challenge,
    DrawnCardDecision,
    RoundOver,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum Action {
    Play { card: CardId },
    Draw,
    Pass,
    ChooseColor { color: Color },
    ChooseTarget { target: Seat },
    CallUno,
    CatchUno { target: Seat },
    Challenge,
    Accept,
    JumpIn { card: CardId },
    Timeout,
    EndByTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum Event {
    RoundStarted {
        round: u32,
        dealer: Seat,
        hand_sizes: Vec<u8>,
    },
    StartCardFlipped {
        card: Card,
    },
    TurnStarted {
        seat: Seat,
    },
    CardPlayed {
        seat: Seat,
        card: Card,
        jump_in: bool,
    },
    CardsDiscarded {
        seat: Seat,
        cards: Vec<Card>,
    },
    CardsDrawn {
        seat: Seat,
        count: u8,
        cards: Option<Vec<Card>>,
        reason: DrawReason,
    },
    DeckReshuffled {
        size: u16,
    },
    ColorChosen {
        seat: Seat,
        color: Color,
    },
    PlayerSkipped {
        seat: Seat,
    },
    DirectionChanged {
        direction: i8,
    },
    PendingDrawChanged {
        amount: u16,
    },
    ChallengeResolved {
        challenger: Seat,
        offender: Seat,
        success: bool,
    },
    UnoCalled {
        seat: Seat,
    },
    UnoCaught {
        catcher: Seat,
        target: Seat,
        penalty: u8,
    },
    HandsSwapped {
        a: Seat,
        b: Seat,
    },
    HandsRotated {
        direction: i8,
    },
    SideFlipped {
        side: Side,
    },
    PlayerEliminated {
        seat: Seat,
        reason: EliminationReason,
    },
    RoundEnded {
        winner: Option<Seat>,
        scores: Vec<i32>,
        hand_points: Vec<u32>,
        totals: Vec<i32>,
    },
    GameEnded {
        rankings: Vec<Seat>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DrawReason {
    Voluntary,
    Penalty,
    Stack,
    Challenge,
    Uno,
    Roulette,
    DrawColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EliminationReason {
    Mercy,
    Elimination,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("not your turn")]
    NotYourTurn,
    #[error("illegal action")]
    IllegalAction,
    #[error("game over")]
    GameOver,
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}
