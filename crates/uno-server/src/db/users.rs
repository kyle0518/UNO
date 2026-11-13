use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::DbError;

/// 使用者資料庫模型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub display_name: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub is_guest: bool,
    pub created_at: String,
}

/// 公開的使用者資訊（不含密碼雜湊）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicUser {
    pub id: i64,
    pub username: Option<String>,
    pub display_name: String,
    pub is_guest: bool,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            is_guest: user.is_guest,
        }
    }
}

/// 建立訪客使用者。
pub fn create_guest(conn: &Connection, display_name: &str) -> Result<User, DbError> {
    let trimmed = display_name.trim();
    if trimmed.is_empty() {
        return Err(DbError::InvalidData("暱稱不得為空".to_string()));
    }
    conn.execute(
        "INSERT INTO users (username, display_name, password_hash, is_guest) VALUES (NULL, ?, NULL, 1);",
        params![trimmed],
    )?;
    let id = conn.last_insert_rowid();
    find_by_id(conn, id)?.ok_or(DbError::UserNotFound(id))
}

/// 建立正式使用者帳號。
pub fn create_user(
    conn: &Connection,
    username: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<User, DbError> {
    let u = username.trim();
    let d = display_name.trim();
    if u.is_empty() {
        return Err(DbError::InvalidData("使用者名稱不得為空".to_string()));
    }
    if d.is_empty() {
        return Err(DbError::InvalidData("顯示名稱不得為空".to_string()));
    }

    let res = conn.execute(
        "INSERT INTO users (username, display_name, password_hash, is_guest) VALUES (?, ?, ?, 0);",
        params![u, d, password_hash],
    );

    match res {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            find_by_id(conn, id)?.ok_or(DbError::UserNotFound(id))
        }
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(DbError::UsernameAlreadyExists(u.to_string()))
        }
        Err(err) => Err(DbError::Sqlite(err)),
    }
}

/// 將現有訪客帳號升級為正式帳號（保留所有歷史戰績與 ID）。
pub fn upgrade_guest(
    conn: &Connection,
    guest_user_id: i64,
    username: &str,
    password_hash: &str,
    display_name: Option<&str>,
) -> Result<User, DbError> {
    let existing = find_by_id(conn, guest_user_id)?.ok_or(DbError::UserNotFound(guest_user_id))?;
    if !existing.is_guest {
        return Err(DbError::InvalidData("該帳號已經是正式會員".to_string()));
    }

    let u = username.trim();
    if u.is_empty() {
        return Err(DbError::InvalidData("使用者名稱不得為空".to_string()));
    }

    let final_display = display_name
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(&existing.display_name);

    let res = conn.execute(
        "UPDATE users SET username = ?, password_hash = ?, is_guest = 0, display_name = ? WHERE id = ? AND is_guest = 1;",
        params![u, password_hash, final_display, guest_user_id],
    );

    match res {
        Ok(0) => Err(DbError::UserNotFound(guest_user_id)),
        Ok(_) => find_by_id(conn, guest_user_id)?.ok_or(DbError::UserNotFound(guest_user_id)),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(DbError::UsernameAlreadyExists(u.to_string()))
        }
        Err(err) => Err(DbError::Sqlite(err)),
    }
}

/// 依 ID 查詢使用者。
pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<User>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, username, display_name, password_hash, is_guest, created_at FROM users WHERE id = ?;",
    )?;
    let user = stmt
        .query_row(params![id], |row| {
            let is_guest_int: i32 = row.get(4)?;
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                password_hash: row.get(3)?,
                is_guest: is_guest_int != 0,
                created_at: row.get(5)?,
            })
        })
        .optional()?;
    Ok(user)
}

/// 依使用者名稱查詢使用者。
pub fn find_by_username(conn: &Connection, username: &str) -> Result<Option<User>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, username, display_name, password_hash, is_guest, created_at FROM users WHERE username = ?;",
    )?;
    let user = stmt
        .query_row(params![username], |row| {
            let is_guest_int: i32 = row.get(4)?;
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                password_hash: row.get(3)?,
                is_guest: is_guest_int != 0,
                created_at: row.get(5)?,
            })
        })
        .optional()?;
    Ok(user)
}

/// 建立使用者 Session Token（預設過期時間 ISO8601 或 SQLite datetime 字串）。
pub fn create_session(
    conn: &Connection,
    token: &str,
    user_id: i64,
    expires_at: &str,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, ?);",
        params![token, user_id, expires_at],
    )?;
    Ok(())
}

/// 建立 30 天有效期的 Session Token（使用 SQLite datetime('now', '+30 days')）。
pub fn create_session_30_days(
    conn: &Connection,
    token: &str,
    user_id: i64,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO sessions (token, user_id, expires_at)
         VALUES (?, ?, datetime('now', '+30 days'));",
        params![token, user_id],
    )?;
    Ok(())
}

/// 依 Token 查詢未過期 Session 的使用者。
pub fn find_user_by_token(conn: &Connection, token: &str) -> Result<Option<User>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.username, u.display_name, u.password_hash, u.is_guest, u.created_at
         FROM sessions s
         JOIN users u ON s.user_id = u.id
         WHERE s.token = ? AND datetime(s.expires_at) > datetime('now');",
    )?;
    let user = stmt
        .query_row(params![token], |row| {
            let is_guest_int: i32 = row.get(4)?;
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                password_hash: row.get(3)?,
                is_guest: is_guest_int != 0,
                created_at: row.get(5)?,
            })
        })
        .optional()?;
    Ok(user)
}

/// 刪除指定 Token 的 Session（登出）。
pub fn delete_session(conn: &Connection, token: &str) -> Result<bool, DbError> {
    let count = conn.execute("DELETE FROM sessions WHERE token = ?;", params![token])?;
    Ok(count > 0)
}

/// 清理所有已過期的 Session。
pub fn delete_expired_sessions(conn: &Connection) -> Result<usize, DbError> {
    let count = conn.execute(
        "DELETE FROM sessions WHERE datetime(expires_at) <= datetime('now');",
        [],
    )?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_migrations;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open memory");
        run_migrations(&mut conn).expect("migrate");
        conn
    }

    #[test]
    fn test_guest_lifecycle_and_upgrade() {
        let conn = setup_test_db();

        let guest = create_guest(&conn, "小明").expect("create guest");
        assert!(guest.is_guest);
        assert_eq!(guest.display_name, "小明");
        assert!(guest.username.is_none());
        assert!(guest.password_hash.is_none());

        // 升級成正式帳號
        let upgraded = upgrade_guest(&conn, guest.id, "ming_123", "argon2_hash_val", Some("大明"))
            .expect("upgrade guest");

        assert_eq!(upgraded.id, guest.id);
        assert!(!upgraded.is_guest);
        assert_eq!(upgraded.username.as_deref(), Some("ming_123"));
        assert_eq!(upgraded.display_name, "大明");

        // 再次以相同 username 建立正式帳號應衝突
        let conflict = create_user(&conn, "ming_123", "另一人", "hash");
        assert!(matches!(conflict, Err(DbError::UsernameAlreadyExists(_))));
    }

    #[test]
    fn test_session_lifecycle_and_expiration() {
        let conn = setup_test_db();
        let user = create_user(&conn, "alice", "Alice", "hash").expect("user");

        // 有效 session (1 小時後)
        let future_time = "2099-01-01 00:00:00";
        create_session(&conn, "token_valid", user.id, future_time).expect("create session");

        let found = find_user_by_token(&conn, "token_valid").expect("find");
        assert_eq!(found.map(|u| u.id), Some(user.id));

        // 過期 session (過去時間)
        let past_time = "2000-01-01 00:00:00";
        create_session(&conn, "token_expired", user.id, past_time).expect("expired session");

        let not_found = find_user_by_token(&conn, "token_expired").expect("find expired");
        assert!(not_found.is_none());

        // 清理過期 session
        let cleaned = delete_expired_sessions(&conn).expect("clean");
        assert_eq!(cleaned, 1);

        // 刪除有效 session (登出)
        let deleted = delete_session(&conn, "token_valid").expect("delete");
        assert!(deleted);
        assert!(find_user_by_token(&conn, "token_valid").unwrap().is_none());
    }
}
