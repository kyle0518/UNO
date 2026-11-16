//! WebSocket 協議型別（INTERFACES §7，路徑 `/ws`，JSON 文字訊息）。
//!
//! 所有型別以 `#[serde(tag = "type", rename_all = "snake_case")]`（列舉）定義，
//! 並以 ts-rs 匯出到 `apps/web/src/generated/`。

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uno_core::{Action, Event, PlayerView, RuleConfig, Seat};

/// 客戶端送出的第一則訊息需帶的協議版本。
pub const PROTOCOL_VERSION: u32 = 1;

/// 公開的使用者資訊（不含任何憑證），與伺服器 DB 層的 `PublicUser` 形狀一致。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserPublic {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub is_guest: bool,
}

/// 回合計時（deadline 為伺服器 epoch ms；客戶端用 Ping/Pong 估時差）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TurnTimer {
    pub seat: Seat,
    #[ts(type = "number")]
    pub deadline_ms: u64,
    pub total_ms: u32,
}

/// Elo 變動（單一真人玩家）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EloChange {
    #[ts(type = "number")]
    pub user_id: i64,
    pub before: i32,
    pub after: i32,
}

/// 伺服器錯誤碼（INTERFACES §7.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    RoomNotFound,
    RoomFull,
    NotHost,
    NotYourTurn,
    IllegalAction,
    GameInProgress,
    RateLimited,
    Internal,
}

/// 房間狀態。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoomStatus {
    Waiting,
    Playing,
    RoundOver,
    GameOver,
}

/// 座位佔用者。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum Occupant {
    Empty,
    Human { user: UserPublic },
    Bot { kind: String },
}

/// 單一座位資訊。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeatInfo {
    pub seat: Seat,
    pub occupant: Occupant,
    pub ready: bool,
    pub connected: bool,
}

/// 房間完整資訊（任何變動時整份送出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RoomInfo {
    pub code: String,
    #[ts(type = "number")]
    pub host_user_id: i64,
    pub public: bool,
    pub config: RuleConfig,
    pub status: RoomStatus,
    pub seats: Vec<SeatInfo>,
    pub spectators: Vec<UserPublic>,
    pub join_url: String,
}

/// Client → Server 訊息（INTERFACES §7.1）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum ClientMsg {
    /// 連線後第一則；`protocol` 應為 [`PROTOCOL_VERSION`]。
    Hello {
        token: String,
        protocol: u32,
    },
    CreateRoom {
        mode_id: String,
        config: Option<RuleConfig>,
        public: bool,
    },
    JoinRoom {
        code: String,
        as_spectator: bool,
    },
    LeaveRoom,
    TakeSeat {
        seat: Seat,
    },
    /// 房主限定；`bot`: random|heuristic|hard|onnx:<model>。
    AddBot {
        seat: Seat,
        bot: String,
    },
    /// 房主限定：踢人/移除 bot。
    RemoveSeat {
        seat: Seat,
    },
    /// 房主限定，等待中才可。
    UpdateConfig {
        config: RuleConfig,
    },
    SetReady {
        ready: bool,
    },
    /// 房主限定，所有真人 ready 才可。
    StartGame,
    /// `seq` 每個客戶端遞增。
    Action {
        seq: u32,
        action: Action,
    },
    /// 房主限定（RoundOver 時；也會在 8 秒後自動）。
    NextRound,
    /// 房主限定，GameOver 後。
    Rematch,
    /// 文字長度上限 200 字（伺服器端驗證）。
    Chat {
        text: String,
    },
    /// "thumbs_up" | "laugh" | "angry" | "cry" | "fire" | "uno"
    Emote {
        emote: String,
    },
    Ping {
        #[ts(type = "number")]
        t: u64,
    },
}

/// Server → Client 訊息（INTERFACES §7.2）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum ServerMsg {
    Welcome {
        user: UserPublic,
        server_version: String,
    },
    Error {
        code: ErrorCode,
        message: String,
        seq: Option<u32>,
    },
    RoomState {
        room: RoomInfo,
    },
    /// 加入/重連/每次動作後送出。
    GameSnapshot {
        view: PlayerView,
        timer: Option<TurnTimer>,
    },
    /// 事件已依接收者 filter。
    GameEvents {
        events: Vec<Event>,
        view: PlayerView,
        timer: Option<TurnTimer>,
    },
    ChatMsg {
        from: String,
        seat: Option<Seat>,
        text: String,
        #[ts(type = "number")]
        t: u64,
    },
    EmoteMsg {
        seat: Seat,
        emote: String,
    },
    MatchSaved {
        #[ts(type = "number")]
        match_id: i64,
        elo_changes: Vec<EloChange>,
    },
    Pong {
        #[ts(type = "number")]
        t: u64,
    },
}

#[cfg(test)]
mod tests;
