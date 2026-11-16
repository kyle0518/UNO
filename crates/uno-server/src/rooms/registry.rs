//! 房間註冊表：持有所有進行中的房間 handle，並負責產生不重複房號。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rand::RngExt;
use uno_protocol::{ErrorCode, RoomInfo, UserPublic};

use super::room::{RoomHandle, RoomParams};
use crate::db::Db;

/// 房號字元集：大寫字母＋數字，排除易混淆的 `0 O 1 I L`。
const CODE_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 6;

/// 產生一個 6 碼房號（不檢查是否重複，由 registry 負責避讓）。
pub fn generate_room_code() -> String {
    let mut rng = rand::rng();
    (0..CODE_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CODE_CHARS.len());
            CODE_CHARS[idx] as char
        })
        .collect()
}

/// 所有房間的註冊表（以 `Mutex<HashMap>` 實作，規模為區網對局）。
#[derive(Clone)]
pub struct RoomRegistry {
    rooms: Arc<Mutex<HashMap<String, RoomHandle>>>,
    db: Db,
    join_base_url: String,
}

impl std::fmt::Debug for RoomRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomRegistry").finish_non_exhaustive()
    }
}

impl RoomRegistry {
    /// `join_base_url` 為對外基底網址（如 `http://192.168.1.10:8787`），
    /// 房間的 `join_url` 會是 `{join_base_url}/r/{code}`。
    pub fn new(db: Db, join_base_url: impl Into<String>) -> Self {
        Self {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            db,
            join_base_url: join_base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// 建立房間並回傳 handle；`config` 為 None 時以 `mode_id` 的 4 人 preset 建立。
    pub fn create_room(
        &self,
        host: UserPublic,
        mode_id: &str,
        config: Option<uno_core::RuleConfig>,
        public: bool,
    ) -> Result<RoomHandle, ErrorCode> {
        let mut config = match config {
            Some(c) => c,
            None => uno_core::RuleConfig::preset(mode_id, 4).map_err(|_| ErrorCode::BadRequest)?,
        };
        config.validate().map_err(|_| ErrorCode::BadRequest)?;

        let mut rooms = self.rooms.lock().map_err(|_| ErrorCode::Internal)?;
        let mut code = generate_room_code();
        while rooms.contains_key(&code) {
            code = generate_room_code();
        }
        let handle = RoomHandle::spawn(RoomParams {
            code: code.clone(),
            host_user_id: host.id,
            public,
            config,
            join_url: format!("{}/r/{}", self.join_base_url, code),
            db: self.db.clone(),
        });
        rooms.insert(code, handle.clone());
        Ok(handle)
    }

    /// 依房號取得房間 handle。
    pub fn get(&self, code: &str) -> Option<RoomHandle> {
        self.rooms.lock().ok()?.get(code).cloned()
    }

    /// 移除房間（房間逾時清理時使用）。
    pub fn remove(&self, code: &str) -> Option<RoomHandle> {
        self.rooms.lock().ok()?.remove(code)
    }

    /// 目前房間數。
    pub fn len(&self) -> usize {
        self.rooms.lock().map(|r| r.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 列出公開且等待中的房間（供 `GET /api/rooms` 使用）。
    pub async fn list_public(&self) -> Vec<RoomInfo> {
        let handles: Vec<RoomHandle> = {
            match self.rooms.lock() {
                Ok(r) => r.values().cloned().collect(),
                Err(_) => return Vec::new(),
            }
        };
        let mut out = Vec::new();
        for h in handles {
            if let Some(info) = h.room_info().await {
                if info.public && info.status == uno_protocol::RoomStatus::Waiting {
                    out.push(info);
                }
            }
        }
        out
    }
}
