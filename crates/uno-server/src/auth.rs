use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use base64::prelude::*;
use rand::RngExt;

use crate::api::ApiError;
use crate::db::users::{self, User};
use crate::state::AppState;

/// 產生 32-byte 隨機 base64url Token（不含 padding）。
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

/// 使用 Argon2id 對密碼進行安全雜湊。
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(format!("密碼雜湊失敗: {e}")))
}

/// 驗證明文密碼是否與 Argon2 雜湊比對成功。
pub fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| ApiError::Internal(format!("解析密碼雜湊失敗: {e}")))?;
    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

/// 驗證使用者名稱格式（3–20 字元，允許英數字、底線、中文字元）。
pub fn validate_username(username: &str) -> Result<(), ApiError> {
    let trimmed = username.trim();
    let char_count = trimmed.chars().count();
    if !(3..=20).contains(&char_count) {
        return Err(ApiError::BadRequest("使用者名稱長度需介於 3 至 20 字".to_string()));
    }

    for c in trimmed.chars() {
        let is_ascii_valid = c.is_ascii_alphanumeric() || c == '_';
        let is_cjk = ('\u{4e00}'..='\u{9fff}').contains(&c);
        if !is_ascii_valid && !is_cjk {
            return Err(ApiError::BadRequest(
                "使用者名稱僅允許英數字、底線與中文字元".to_string(),
            ));
        }
    }
    Ok(())
}

/// 驗證密碼格式（至少 6 字元）。
pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.chars().count() < 6 {
        return Err(ApiError::BadRequest("密碼長度至少需 6 個字元".to_string()));
    }
    Ok(())
}

/// 驗證暱稱格式（1–20 字元）。
pub fn validate_nickname(nickname: &str) -> Result<(), ApiError> {
    let count = nickname.trim().chars().count();
    if count == 0 || count > 20 {
        return Err(ApiError::BadRequest("暱稱長度需介於 1 至 20 字".to_string()));
    }
    Ok(())
}

/// 從請求標頭解析 Bearer Token。
pub fn extract_bearer_token(parts: &Parts) -> Option<String> {
    let header_val = parts.headers.get(axum::http::header::AUTHORIZATION)?;
    let header_str = header_val.to_str().ok()?;
    header_str
        .strip_prefix("Bearer ")
        .or_else(|| header_str.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 強制需要登入的 Axum Extractor。
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_token(parts).ok_or_else(|| {
            ApiError::Unauthorized("缺少或無效的 Authorization 標頭".to_string())
        })?;

        let db = state.db().clone();
        let user = db
            .run(move |conn| users::find_user_by_token(conn, &token))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .ok_or_else(|| ApiError::Unauthorized("Token 無效或已過期".to_string()))?;

        Ok(AuthUser(user))
    }
}

/// 選填登入身份的 Axum Extractor（用於訪客升級等）。
pub struct OptionalAuthUser(pub Option<User>);

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = match extract_bearer_token(parts) {
            Some(t) => t,
            None => return Ok(OptionalAuthUser(None)),
        };

        let db = state.db().clone();
        let user = db
            .run(move |conn| users::find_user_by_token(conn, &token))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(OptionalAuthUser(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_format() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
        assert_eq!(t1.len(), 43); // 32 bytes in base64 without padding = 43 chars
    }

    #[test]
    fn test_password_hash_and_verify() {
        let pwd = "correct_password_123";
        let hash = hash_password(pwd).expect("hash");
        assert!(verify_password(pwd, &hash).expect("verify true"));
        assert!(!verify_password("wrong_password", &hash).expect("verify false"));
    }

    #[test]
    fn test_validations() {
        assert!(validate_username("alice_01").is_ok());
        assert!(validate_username("小明_99").is_ok());
        assert!(validate_username("ab").is_err()); // 太短
        assert!(validate_username("alice@home").is_err()); // 特殊符號不允許
        assert!(validate_password("123456").is_ok());
        assert!(validate_password("12345").is_err()); // 小於 6
        assert!(validate_nickname("UNO大師").is_ok());
        assert!(validate_nickname("").is_err());
    }
}
