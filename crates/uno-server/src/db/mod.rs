pub mod matches;
pub mod migrations;
pub mod ratings;
pub mod users;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// 資料庫操作錯誤類型。
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite 錯誤: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("資料庫互斥鎖 poisoned")]
    LockPoisoned,

    #[error("非同步執行錯誤: {0}")]
    JoinError(String),

    #[error("使用者不存在 (id={0})")]
    UserNotFound(i64),

    #[error("使用者名稱已存在: {0}")]
    UsernameAlreadyExists(String),

    #[error("無效資料: {0}")]
    InvalidData(String),
}

/// 線程安全且可非同步呼叫的資料庫管理物件。
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db").finish()
    }
}

impl Db {
    /// 開啟本機 SQLite 資料庫檔案並自動套用 migrations 與 WAL 模式。
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| DbError::InvalidData(e.to_string()))?;
            }
        }

        let mut conn = Connection::open(p)?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;",
        )?;
        migrations::run_migrations(&mut conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 開啟記憶體暫存資料庫（適合測試）。
    pub fn open_in_memory() -> Result<Self, DbError> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::run_migrations(&mut conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 在 tokio 阻擋工作執行緒集 (`spawn_blocking`) 中非同步執行資料庫操作。
    pub async fn run<F, R>(&self, f: F) -> Result<R, DbError>
    where
        F: FnOnce(&mut Connection) -> Result<R, DbError> + Send + 'static,
        R: Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = conn.lock().map_err(|_| DbError::LockPoisoned)?;
            f(&mut guard)
        })
        .await
        .map_err(|e| DbError::JoinError(e.to_string()))?
    }

    /// 同步取得資料庫連線並執行閉包（供單元測試或同步環境使用）。
    pub fn with_conn<F, R>(&self, f: F) -> Result<R, DbError>
    where
        F: FnOnce(&mut Connection) -> Result<R, DbError>,
    {
        let mut guard = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_open_in_memory_and_async_run() {
        let db = Db::open_in_memory().expect("open db");

        let user = db
            .run(|conn| users::create_guest(conn, "AsyncGuest"))
            .await
            .expect("create guest");

        assert!(user.is_guest);
        assert_eq!(user.display_name, "AsyncGuest");

        let found = db
            .run(move |conn| users::find_by_id(conn, user.id))
            .await
            .expect("find user");

        assert_eq!(found.unwrap().display_name, "AsyncGuest");
    }

    #[test]
    fn test_db_open_file_and_wal_mode() {
        let tmp_dir = std::env::temp_dir().join(format!("uno-db-test-{}", std::process::id()));
        let db_path = tmp_dir.join("test.db");

        let db = Db::open(&db_path).expect("open file db");

        let journal_mode: String = db
            .with_conn(|conn| {
                conn.query_row("PRAGMA journal_mode;", [], |row| row.get(0))
                    .map_err(DbError::from)
            })
            .expect("pragma");

        assert_eq!(journal_mode.to_uppercase(), "WAL");

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
