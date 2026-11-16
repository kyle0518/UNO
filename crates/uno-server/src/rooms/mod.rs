//! 多人房間模組（INTERFACES §7.3）：每個房間一個 tokio task（actor），
//! 房間狀態只在該 task 內修改，外部透過 [`RoomHandle`] 以命令溝通。

pub mod bots;
pub mod registry;
pub mod room;
pub mod timers;

#[cfg(test)]
mod tests;

use tokio::sync::{mpsc, oneshot};
use uno_core::Seat;
use uno_protocol::{ClientMsg, ErrorCode, ServerMsg, UserPublic};

pub use registry::{generate_room_code, RoomRegistry};
pub use room::{RoomHandle, RoomParams};

/// 送往單一連線的訊息通道（WebSocket 層在 T7.3 接上）。
pub type OutTx = mpsc::UnboundedSender<ServerMsg>;
pub type OutRx = mpsc::UnboundedReceiver<ServerMsg>;

/// 房間 actor 命令。除 [`RoomCommand::Msg`] 對應 `ClientMsg` 外，
/// 其餘為連線管理與內部排程（bot、計時器、斷線）命令；
/// 排程命令皆帶 `epoch`/`turn` 供 actor 驗證未過期。
pub enum RoomCommand {
    /// 加入房間（或重連）：成功時 actor 會回送 RoomState／快照。
    Join {
        user: UserPublic,
        as_spectator: bool,
        out: OutTx,
        reply: oneshot::Sender<Result<(), ErrorCode>>,
    },
    /// 使用者主動離開（釋放座位並移除連線）。
    Leave { user_id: i64 },
    /// 連線中斷（保留座位，啟動代打與 60 秒轉 bot 計時）。
    Disconnect { user_id: i64 },
    /// 處理一則客戶端訊息（不含 Hello 與進房路由，由 WebSocket 層先行處理）。
    Msg { user_id: i64, msg: ClientMsg },
    /// Bot（或斷線代打）回合排程到點。
    BotTurn { epoch: u64, turn: u32, seat: Seat },
    /// Bot 抓 UNO 排程到點。
    BotCatch {
        epoch: u64,
        target: Seat,
        seat: Seat,
    },
    /// 回合計時到期。
    TurnTimeout { epoch: u64, turn: u32, seat: Seat },
    /// 對局總時限到期。
    GameTimeout { epoch: u64 },
    /// RoundOver 後自動進入下一輪（8 秒）。
    AutoNextRound { epoch: u64, round: u32 },
    /// 斷線逾 60 秒：座位永久轉為 bot。
    DisconnectExpired { user_id: i64, epoch: u64 },
    /// 查詢目前 RoomInfo（registry 列表用）。
    QueryInfo {
        reply: oneshot::Sender<uno_protocol::RoomInfo>,
    },
}
