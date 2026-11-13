pub mod auth;
pub mod users;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::state::AppState;

/// 統一的 API 錯誤類型。
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

impl From<crate::db::DbError> for ApiError {
    fn from(err: crate::db::DbError) -> Self {
        match err {
            crate::db::DbError::UsernameAlreadyExists(name) => {
                ApiError::Conflict(format!("使用者名稱已被使用: {name}"))
            }
            crate::db::DbError::UserNotFound(id) => {
                ApiError::NotFound(format!("使用者不存在: {id}"))
            }
            crate::db::DbError::InvalidData(msg) => ApiError::BadRequest(msg),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

/// 建立 `/api` 路由表。
pub fn create_api_router() -> Router<AppState> {
    Router::new()
        .route("/guest", post(auth::guest))
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/me", get(users::me))
}
