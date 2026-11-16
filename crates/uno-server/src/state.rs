use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::ServerConfig;
use crate::db::Db;
use crate::rooms::RoomRegistry;

/// 簡易滑動視窗限流器（以 key 區分，例如登入的使用者名稱）。
#[derive(Debug, Default)]
pub struct RateLimiter {
    windows: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// 記錄一次嘗試；視窗內未超限回傳 `true`，超限回傳 `false`。
    pub fn check(&self, key: &str, limit: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut guard = match self.windows.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let entry = guard.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < window);
        if entry.len() >= limit {
            return false;
        }
        entry.push(now);
        true
    }
}

/// 伺服器共享狀態內容。
#[derive(Debug)]
struct AppStateInner {
    config: ServerConfig,
    version: String,
    port: u16,
    lan_urls: Vec<String>,
    db: Db,
    auth_limiter: RateLimiter,
    rooms: RoomRegistry,
}

/// 透過 `Arc` 共享給 Axum handler 與背景工作執行的伺服器狀態。
#[derive(Clone, Debug)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

impl AppState {
    /// 建立新的伺服器狀態實例。
    pub fn new(config: ServerConfig, port: u16, lan_urls: Vec<String>, db: Db) -> Self {
        let join_base = lan_urls
            .first()
            .cloned()
            .unwrap_or_else(|| format!("http://127.0.0.1:{port}"));
        let rooms = RoomRegistry::new(db.clone(), join_base);
        Self {
            inner: Arc::new(AppStateInner {
                config,
                version: env!("CARGO_PKG_VERSION").to_string(),
                port,
                lan_urls,
                db,
                auth_limiter: RateLimiter::default(),
                rooms,
            }),
        }
    }

    pub fn config(&self) -> &ServerConfig {
        &self.inner.config
    }

    pub fn version(&self) -> &str {
        &self.inner.version
    }

    pub fn port(&self) -> u16 {
        self.inner.port
    }

    pub fn lan_urls(&self) -> &[String] {
        &self.inner.lan_urls
    }

    pub fn db(&self) -> &Db {
        &self.inner.db
    }

    pub fn rooms(&self) -> &RoomRegistry {
        &self.inner.rooms
    }

    /// 認證端點限流：同一 key 每分鐘最多 10 次、全部認證請求每分鐘最多 300 次。
    pub fn check_auth_rate_limit(&self, key: &str) -> bool {
        let window = Duration::from_secs(60);
        self.inner.auth_limiter.check(key, 10, window)
            && self
                .inner
                .auth_limiter
                .check("__auth_global__", 300, window)
    }
}
