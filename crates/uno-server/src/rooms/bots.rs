//! Bot 座位輔助：建立 bot 實例、依座位種類決定 CatchUno 機率、
//! 以及斷線真人座位的 Heuristic 代打。

use std::str::FromStr;

use rand::RngExt;
use uno_bots::{make_bot, Bot, BotKind, HeuristicBot};
use uno_core::{Action, PlayerView, Seat};
use uno_protocol::ErrorCode;

/// Bot 行動前的思考延遲範圍（INTERFACES §7.3：600–1200ms）。
pub fn bot_think_delay_ms() -> u64 {
    rand::rng().random_range(600..=1200)
}

/// 斷線真人座位的代打延遲（§7.3：1 秒）。
pub const TAKEOVER_DELAY_MS: u64 = 1000;

/// Bot 抓 UNO 的延遲上限（§7.3：1.5 秒內抓）。
pub fn bot_catch_delay_ms() -> u64 {
    rand::rng().random_range(200..=1200)
}

/// 依 bot 種類回傳 CatchUno 機率（百分比）：random 0%、heuristic/onnx 30%、hard 70%。
pub fn catch_chance_percent(bot_kind: &str) -> u32 {
    match BotKind::from_str(bot_kind) {
        Ok(BotKind::Random) => 0,
        Ok(BotKind::Hard) => 70,
        Ok(BotKind::Hell) => 90,
        _ => 30,
    }
}

/// 擲一次 CatchUno 機率（百分之一精度以整數擲骰避免浮點）。
pub fn roll_catch(bot_kind: &str) -> bool {
    let chance = catch_chance_percent(bot_kind);
    chance > 0 && rand::rng().random_range(0..100) < chance
}

/// 驗證 bot 字串可解析且可建立（AddBot 時先行檢查）。
pub fn validate_bot_kind(bot_kind: &str) -> Result<BotKind, ErrorCode> {
    let kind = BotKind::from_str(bot_kind).map_err(|_| ErrorCode::BadRequest)?;
    make_bot(&kind, 0).map_err(|_| ErrorCode::BadRequest)?;
    Ok(kind)
}

/// 建立座位 bot 實例（種子依對局 seed 與座位衍生）。
pub fn create_seat_bot(bot_kind: &str, game_seed: u64, seat: Seat) -> Option<Box<dyn Bot>> {
    let kind = BotKind::from_str(bot_kind).ok()?;
    make_bot(
        &kind,
        game_seed ^ (u64::from(seat) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
    )
    .ok()
}

/// 斷線代打：以 HeuristicBot 依該座位視野選一個合法動作。
pub fn takeover_action(view: &PlayerView) -> Option<Action> {
    if view.legal.is_empty() {
        return None;
    }
    let mut bot = HeuristicBot::new();
    let action = bot.act(view);
    if view.legal.contains(&action) {
        Some(action)
    } else {
        view.legal.first().copied()
    }
}

/// 確保 bot 回傳動作合法，否則退回第一個合法動作。
pub fn ensure_legal(view: &PlayerView, action: Action) -> Option<Action> {
    if view.legal.contains(&action) {
        Some(action)
    } else {
        view.legal.first().copied()
    }
}
