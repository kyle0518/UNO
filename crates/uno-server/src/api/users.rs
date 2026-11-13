use std::collections::BTreeMap;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::ApiError;
use crate::auth::AuthUser;
use crate::db::ratings;
use crate::db::users::PublicUser;
use crate::state::AppState;

/// `GET /api/me` 回應格式。
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeResponse {
    pub user: PublicUser,
    pub ratings: BTreeMap<String, i32>,
}

/// 取得當前已登入使用者的個人資料與評分。
pub async fn me(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MeResponse>, ApiError> {
    let user = auth_user.0;
    let uid = user.id;

    let user_ratings = state
        .db()
        .run(move |conn| ratings::get_user_ratings(conn, uid))
        .await?;

    Ok(Json(MeResponse {
        user: user.into(),
        ratings: user_ratings,
    }))
}
