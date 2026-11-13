use std::path::PathBuf;

use clap::Parser;

/// `uno-server` 命令列與執行環境設定。
#[derive(Parser, Debug, Clone)]
#[command(name = "uno-server", version, about = "UNO 區網多人連線與戰績伺服器")]
pub struct ServerConfig {
    /// HTTP / WebSocket 監聽連接埠
    #[arg(long, default_value_t = 8787, env = "UNO_PORT")]
    pub port: u16,

    /// 監聽位址（預設 0.0.0.0 以允許區網手機連線）
    #[arg(long, default_value = "0.0.0.0", env = "UNO_HOST")]
    pub host: String,

    /// SQLite 資料庫與伺服器資料目錄
    #[arg(long, default_value = "data", env = "UNO_DATA_DIR")]
    pub data_dir: PathBuf,

    /// 前端建置產物目錄（SPA 靜態檔）
    #[arg(long, default_value = "apps/web/dist", env = "UNO_WEB_DIR")]
    pub web_dir: PathBuf,

    /// ONNX 模型與 metadata 目錄
    #[arg(long, default_value = "models", env = "UNO_MODELS_DIR")]
    pub models_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8787,
            host: "0.0.0.0".to_string(),
            data_dir: PathBuf::from("data"),
            web_dir: PathBuf::from("apps/web/dist"),
            models_dir: PathBuf::from("models"),
        }
    }
}

impl ServerConfig {
    /// 回傳提供給 `TcpListener::bind` 的 `host:port` 字串。
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_spec() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.port, 8787);
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.data_dir, PathBuf::from("data"));
        assert_eq!(cfg.web_dir, PathBuf::from("apps/web/dist"));
        assert_eq!(cfg.models_dir, PathBuf::from("models"));
        assert_eq!(cfg.bind_addr(), "0.0.0.0:8787");
    }

    #[test]
    fn clap_parses_defaults_and_overrides() {
        let default_cfg = ServerConfig::try_parse_from(["uno-server"]).expect("parse defaults");
        assert_eq!(default_cfg.port, 8787);
        assert_eq!(default_cfg.host, "0.0.0.0");

        let custom_cfg = ServerConfig::try_parse_from([
            "uno-server",
            "--port",
            "9000",
            "--host",
            "127.0.0.1",
            "--data-dir",
            "/tmp/uno-data",
            "--web-dir",
            "/tmp/uno-web",
            "--models-dir",
            "/tmp/uno-models",
        ])
        .expect("parse custom flags");

        assert_eq!(custom_cfg.port, 9000);
        assert_eq!(custom_cfg.host, "127.0.0.1");
        assert_eq!(custom_cfg.data_dir, PathBuf::from("/tmp/uno-data"));
        assert_eq!(custom_cfg.web_dir, PathBuf::from("/tmp/uno-web"));
        assert_eq!(custom_cfg.models_dir, PathBuf::from("/tmp/uno-models"));
    }
}
