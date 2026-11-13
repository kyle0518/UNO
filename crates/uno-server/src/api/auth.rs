use axum::extract::State;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::ApiError;
use crate::auth::{
    self, generate_token, hash_password, validate_nickname, validate_password, validate_username,
    verify_password, OptionalAuthUser,
};
use crate::db::users::{self, PublicUser};
use crate::state::AppState;

/// `POST /api/guest` 請求主體。
#[derive(Debug, Deserialize)]
pub struct GuestRequest {
    pub nickname: String,
}

/// `POST /api/register` 請求主體。
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

/// `POST /api/login` 請求主體。
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 認證成功回應。
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthResponse {
    pub token: String,
    pub user: PublicUser,
}

/// 訪客快速登入。
pub async fn guest(
    State(state): State<AppState>,
    Json(req): Json<GuestRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    validate_nickname(&req.nickname)?;
    let nickname = req.nickname.trim().to_string();
    let token = generate_token();
    let token_clone = token.clone();

    let user = state
        .db()
        .run(move |conn| {
            let user = users::create_guest(conn, &nickname)?;
            users::create_session_30_days(conn, &token_clone, user.id)?;
            Ok(user)
        })
        .await?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// 正式會員註冊（若攜帶訪客 Bearer Token 則平滑升級訪客帳號）。
pub async fn register(
    State(state): State<AppState>,
    opt_user: OptionalAuthUser,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    validate_username(&req.username)?;
    validate_password(&req.password)?;

    let username = req.username.trim().to_string();
    let password_hash = hash_password(&req.password)?;
    let token = generate_token();
    let token_clone = token.clone();

    let user = match opt_user.0 {
        Some(guest_user) if guest_user.is_guest => {
            // 升級訪客帳號
            let uid = guest_user.id;
            state
                .db()
                .run(move |conn| {
                    let user = users::upgrade_guest(conn, uid, &username, &password_hash, None)?;
                    users::create_session_30_days(conn, &token_clone, user.id)?;
                    Ok(user)
                })
                .await?
        }
        Some(_) => {
            return Err(ApiError::BadRequest("該帳號已經是正式會員".to_string()));
        }
        None => {
            // 全新註冊
            let display_name = username.clone();
            state
                .db()
                .run(move |conn| {
                    let user = users::create_user(conn, &username, &display_name, &password_hash)?;
                    users::create_session_30_days(conn, &token_clone, user.id)?;
                    Ok(user)
                })
                .await?
        }
    };

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// 會員帳密登入。
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    validate_username(&req.username)?;
    validate_password(&req.password)?;

    let username = req.username.trim().to_string();
    let user_opt = state
        .db()
        .run(move |conn| users::find_by_username(conn, &username))
        .await?;

    let user = user_opt.ok_or_else(|| ApiError::Unauthorized("帳號或密碼錯誤".to_string()))?;
    let hash = user
        .password_hash
        .as_deref()
        .ok_or_else(|| ApiError::Unauthorized("帳號或密碼錯誤".to_string()))?;

    let is_valid = verify_password(&req.password, hash)?;
    if !is_valid {
        return Err(ApiError::Unauthorized("帳號或密碼錯誤".to_string()));
    }

    let token = generate_token();
    let token_clone = token.clone();
    let uid = user.id;

    state
        .db()
        .run(move |conn| users::create_session_30_days(conn, &token_clone, uid))
        .await?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// 登出（作廢當前 Session Token）。
pub async fn logout(State(state): State<AppState>, parts: Parts) -> StatusCode {
    if let Some(token) = auth::extract_bearer_token(&parts) {
        let _ = state
            .db()
            .run(move |conn| users::delete_session(conn, &token))
            .await;
    }
    StatusCode::NO_CONTENT
}
