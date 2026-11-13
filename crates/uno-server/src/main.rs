pub mod api;
pub mod auth;
mod config;
pub mod db;
mod lan;
mod state;
mod static_files;

use std::process;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::ServerConfig;
use crate::state::AppState;

/// `GET /api/server-info` 回應格式。
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerInfoResponse {
    pub version: String,
    pub lan_urls: Vec<String>,
    pub port: u16,
}

async fn server_info(State(state): State<AppState>) -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse {
        version: state.version().to_string(),
        lan_urls: state.lan_urls().to_vec(),
        port: state.port(),
    })
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "Not found" })),
    )
}

/// 建立 Axum 應用程式路由器（含 `/api`、`/models` 與 SPA fallback）。
pub fn build_router(state: AppState) -> Router {
    let api_router = Router::new()
        .route("/server-info", get(server_info))
        .merge(api::create_api_router())
        .fallback(api_not_found);

    let models_dir = state.config().models_dir.clone();
    let web_dir = state.config().web_dir.clone();

    Router::new()
        .nest("/api", api_router)
        .nest_service("/models", static_files::models_service(&models_dir))
        .fallback_service(static_files::spa_service(&web_dir))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::warn!(error = %err, "無法註冊 Ctrl+C 中斷處理");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => {
                tracing::warn!(error = %err, "無法註冊 SIGTERM 中斷處理");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("收到關閉訊號，正在優雅結束伺服器...");
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("uno_server=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = ServerConfig::parse();
    let bind_addr = config.bind_addr();
    let listener = TcpListener::bind(&bind_addr).await?;
    let bound_port = listener.local_addr()?.port();
    let lan_urls = lan::discover_lan_urls(&config.host, bound_port);

    if !config.web_dir.join("index.html").exists() {
        tracing::warn!(
            web_dir = %config.web_dir.display(),
            "找不到 index.html；請先執行 pnpm -C apps/web build"
        );
    }

    lan::print_startup_banner(bound_port, &lan_urls);
    tracing::info!(
        host = %config.host,
        port = bound_port,
        data_dir = %config.data_dir.display(),
        web_dir = %config.web_dir.display(),
        models_dir = %config.models_dir.display(),
        "UNO server listening"
    );

    let db_path = config.data_dir.join("uno.db");
    let db = db::Db::open(&db_path)?;
    let state = AppState::new(config, bound_port, lan_urls, db);
    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("uno-server 啟動失敗：{err}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "uno-server-{prefix}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn test_state(web_dir: PathBuf, models_dir: PathBuf) -> AppState {
        let cfg = ServerConfig {
            port: 8787,
            host: "0.0.0.0".to_string(),
            data_dir: PathBuf::from("data"),
            web_dir,
            models_dir,
        };
        let db = db::Db::open_in_memory().expect("test db");
        AppState::new(cfg, 8787, vec!["http://192.168.1.23:8787".to_string()], db)
    }

    #[tokio::test]
    async fn server_info_returns_expected_json() {
        let tmp = TempDir::new("info");
        let app = build_router(test_state(tmp.path.join("web"), tmp.path.join("models")));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/server-info")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), 1024 * 64)
            .await
            .expect("read body");
        let info: ServerInfoResponse = serde_json::from_slice(&bytes).expect("parse json");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.port, 8787);
        assert_eq!(info.lan_urls, vec!["http://192.168.1.23:8787"]);
    }

    #[tokio::test]
    async fn unknown_api_route_returns_404_json_instead_of_spa_fallback() {
        let tmp = TempDir::new("api404");
        let web_dir = tmp.path.join("web");
        fs::create_dir_all(&web_dir).expect("mkdir web");
        fs::write(web_dir.join("index.html"), "<html>SPA</html>").expect("write index");

        let app = build_router(test_state(web_dir, tmp.path.join("models")));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/nonexistent")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(resp.into_body(), 1024 * 64)
            .await
            .expect("read body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json error");
        assert_eq!(body["error"], "Not found");
    }

    #[tokio::test]
    async fn static_files_and_spa_fallback_work() {
        let tmp = TempDir::new("spa");
        let web_dir = tmp.path.join("web");
        let models_dir = tmp.path.join("models");
        fs::create_dir_all(web_dir.join("assets")).expect("mkdir assets");
        fs::create_dir_all(&models_dir).expect("mkdir models");
        fs::write(
            web_dir.join("index.html"),
            "<!doctype html><html>UNO SPA</html>",
        )
        .expect("write index");
        fs::write(web_dir.join("assets/app.js"), "console.log('uno');").expect("write js");
        fs::write(models_dir.join("demo.json"), "{\"name\":\"demo\"}").expect("write model meta");

        let app = build_router(test_state(web_dir, models_dir));

        // 靜態資源直接回傳該檔案
        let asset_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("asset response");
        assert_eq!(asset_resp.status(), StatusCode::OK);
        let asset_body = to_bytes(asset_resp.into_body(), 1024 * 64)
            .await
            .expect("read asset");
        assert_eq!(asset_body, "console.log('uno');");

        // 未知前端路由 fallback 回傳 index.html
        let spa_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/some/route")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("spa response");
        assert_eq!(spa_resp.status(), StatusCode::OK);
        let spa_body = to_bytes(spa_resp.into_body(), 1024 * 64)
            .await
            .expect("read spa");
        assert_eq!(spa_body, "<!doctype html><html>UNO SPA</html>");

        // /models 存在檔案回 200，不存在回 404（不走 SPA fallback）
        let model_ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/models/demo.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("model response");
        assert_eq!(model_ok.status(), StatusCode::OK);

        let model_missing = app
            .oneshot(
                Request::builder()
                    .uri("/models/missing.onnx")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("missing model response");
        assert_eq!(model_missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_auth_full_lifecycle_and_validation() {
        let tmp = TempDir::new("auth-api");
        let app = build_router(test_state(tmp.path.join("web"), tmp.path.join("models")));

        // 1. 訪客快速登入
        let guest_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/guest")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"nickname":"訪客007"}"#))
                    .expect("req"),
            )
            .await
            .expect("guest resp");
        assert_eq!(guest_resp.status(), StatusCode::OK);
        let guest_bytes = to_bytes(guest_resp.into_body(), 1024 * 64)
            .await
            .expect("bytes");
        let guest_data: api::auth::AuthResponse =
            serde_json::from_slice(&guest_bytes).expect("parse auth resp");
        assert!(guest_data.user.is_guest);
        assert_eq!(guest_data.user.display_name, "訪客007");

        // 2. 訪客升級為正式帳號 (帶訪客 Token)
        let upgrade_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/register")
                    .header("authorization", format!("Bearer {}", guest_data.token))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"ming_pro","password":"password123"}"#))
                    .expect("req"),
            )
            .await
            .expect("upgrade resp");
        assert_eq!(upgrade_resp.status(), StatusCode::OK);
        let upgrade_bytes = to_bytes(upgrade_resp.into_body(), 1024 * 64)
            .await
            .expect("bytes");
        let upgrade_data: api::auth::AuthResponse =
            serde_json::from_slice(&upgrade_bytes).expect("parse upgrade resp");
        assert!(!upgrade_data.user.is_guest);
        assert_eq!(upgrade_data.user.id, guest_data.user.id);
        assert_eq!(upgrade_data.user.username.as_deref(), Some("ming_pro"));

        // 3. 呼叫 /api/me 驗證個人資料
        let me_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/me")
                    .header("authorization", format!("Bearer {}", upgrade_data.token))
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("me resp");
        assert_eq!(me_resp.status(), StatusCode::OK);
        let me_bytes = to_bytes(me_resp.into_body(), 1024 * 64).await.expect("bytes");
        let me_data: api::users::MeResponse =
            serde_json::from_slice(&me_bytes).expect("parse me resp");
        assert_eq!(me_data.user.id, guest_data.user.id);
        assert_eq!(me_data.user.username.as_deref(), Some("ming_pro"));

        // 4. 重複帳號註冊回傳 409 Conflict
        let conflict_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/register")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"ming_pro","password":"password456"}"#))
                    .expect("req"),
            )
            .await
            .expect("conflict resp");
        assert_eq!(conflict_resp.status(), StatusCode::CONFLICT);

        // 5. 錯誤密碼登入回傳 401 Unauthorized
        let wrong_pwd_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"ming_pro","password":"wrong_password"}"#))
                    .expect("req"),
            )
            .await
            .expect("wrong pwd resp");
        assert_eq!(wrong_pwd_resp.status(), StatusCode::UNAUTHORIZED);

        // 6. 正確登入
        let login_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"ming_pro","password":"password123"}"#))
                    .expect("req"),
            )
            .await
            .expect("login resp");
        assert_eq!(login_resp.status(), StatusCode::OK);
        let login_bytes = to_bytes(login_resp.into_body(), 1024 * 64).await.expect("bytes");
        let login_data: api::auth::AuthResponse =
            serde_json::from_slice(&login_bytes).expect("parse login resp");
        assert_eq!(login_data.user.username.as_deref(), Some("ming_pro"));

        // 7. 登出
        let logout_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/logout")
                    .header("authorization", format!("Bearer {}", login_data.token))
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("logout resp");
        assert_eq!(logout_resp.status(), StatusCode::NO_CONTENT);

        // 8. 登出後使用該 Token 呼叫 /api/me 應回傳 401
        let after_logout_me = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/me")
                    .header("authorization", format!("Bearer {}", login_data.token))
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("after logout resp");
        assert_eq!(after_logout_me.status(), StatusCode::UNAUTHORIZED);
    }
}
