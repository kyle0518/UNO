use std::sync::Arc;

use crate::config::ServerConfig;
use crate::db::Db;

/// 伺服器共享狀態內容。
#[derive(Debug)]
struct AppStateInner {
    config: ServerConfig,
    version: String,
    port: u16,
    lan_urls: Vec<String>,
    db: Db,
}

/// 透過 `Arc` 共享給 Axum handler 與背景工作執行的伺服器狀態。
#[derive(Clone, Debug)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

impl AppState {
    /// 建立新的伺服器狀態實例。
    pub fn new(config: ServerConfig, port: u16, lan_urls: Vec<String>, db: Db) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config,
                version: env!("CARGO_PKG_VERSION").to_string(),
                port,
                lan_urls,
                db,
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
}
