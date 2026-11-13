//! UNO AI Bot 介面與實作（Random、Heuristic、Hard ISMCTS，以及預留的 ONNX 介面）。

pub mod hard;
pub mod heuristic;
pub mod random;
pub(crate) mod tactical;

use std::fmt;
use std::str::FromStr;

use thiserror::Error;
use uno_core::{Action, PlayerView};

pub use hard::{HardBot, DEFAULT_SIMS, MOBILE_SIMS};
pub use heuristic::HeuristicBot;
pub use random::RandomBot;

/// 所有電腦玩家的共同介面；`act` 回傳的動作必須屬於 `state_view.legal`。
pub trait Bot: Send {
    /// 回傳 Bot 的簡短識別名稱（如 `"random"`、`"heuristic"`）。
    fn name(&self) -> &'static str;

    /// 根據當前玩家的公開與自身資訊視野選擇合法動作。
    fn act(&mut self, state_view: &PlayerView) -> Action;
}

/// 可建立的 Bot 類型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BotKind {
    Random,
    Heuristic,
    Hard,
    Onnx(String),
}

impl fmt::Display for BotKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotKind::Random => f.write_str("random"),
            BotKind::Heuristic => f.write_str("heuristic"),
            BotKind::Hard => f.write_str("hard"),
            BotKind::Onnx(path) => write!(f, "onnx:{path}"),
        }
    }
}

impl FromStr for BotKind {
    type Err = BotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "random" => Ok(BotKind::Random),
            "heuristic" => Ok(BotKind::Heuristic),
            "hard" => Ok(BotKind::Hard),
            other => {
                if let Some(path) = other.strip_prefix("onnx:") {
                    if path.is_empty() {
                        return Err(BotError::InvalidKind(other.to_string()));
                    }
                    return Ok(BotKind::Onnx(path.to_string()));
                }
                Err(BotError::InvalidKind(other.to_string()))
            }
        }
    }
}

/// 建立或載入 Bot 時可能產生的錯誤。
#[derive(Debug, Error)]
pub enum BotError {
    #[error("未知的 bot 類型：{0}")]
    InvalidKind(String),
    #[error("此建置未實作或未啟用 Hard bot")]
    HardUnavailable,
    #[error("此建置未啟用 onnx feature（模型路徑：{0}）")]
    OnnxUnsupported(String),
    #[error("載入 ONNX 模型 '{path}' 失敗：{reason}")]
    ModelLoad { path: String, reason: String },
}

/// 依 `BotKind` 與亂數種子建立 Bot 實例。
pub fn make_bot(kind: &BotKind, seed: u64) -> Result<Box<dyn Bot>, BotError> {
    match kind {
        BotKind::Random => Ok(Box::new(RandomBot::new(seed))),
        BotKind::Heuristic => Ok(Box::new(HeuristicBot::new())),
        BotKind::Hard => Ok(Box::new(HardBot::new(seed, DEFAULT_SIMS))),
        BotKind::Onnx(path) => Err(BotError::OnnxUnsupported(path.clone())),
    }
}
