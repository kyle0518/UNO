use rusqlite::Connection;

use crate::db::DbError;

/// 建立所有 UNO 資料庫表格與索引的 DDL。
pub const MIGRATIONS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  username TEXT UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT,
  is_guest INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sessions (
  token TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS matches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mode_id TEXT NOT NULL,
  num_players INTEGER NOT NULL,
  is_solo INTEGER NOT NULL DEFAULT 0,
  seed INTEGER NOT NULL,
  replay_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS match_players (
  match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
  seat INTEGER NOT NULL,
  user_id INTEGER REFERENCES users(id),
  bot_kind TEXT,
  rank INTEGER NOT NULL,
  score INTEGER NOT NULL,
  PRIMARY KEY (match_id, seat)
);

CREATE TABLE IF NOT EXISTS ratings (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  mode_id TEXT NOT NULL,
  elo INTEGER NOT NULL DEFAULT 1200,
  games INTEGER NOT NULL DEFAULT 0,
  wins INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, mode_id)
);

CREATE TABLE IF NOT EXISTS rating_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL,
  mode_id TEXT NOT NULL,
  elo INTEGER NOT NULL,
  match_id INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS daily_scores (
  user_id INTEGER NOT NULL REFERENCES users(id),
  date TEXT NOT NULL,
  score INTEGER NOT NULL,
  match_id INTEGER NOT NULL,
  PRIMARY KEY (user_id, date)
);

CREATE INDEX IF NOT EXISTS idx_mp_user ON match_players(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_rh_user_mode ON rating_history(user_id, mode_id);
CREATE INDEX IF NOT EXISTS idx_ratings_mode_elo ON ratings(mode_id, elo DESC);
"#;

/// 在連線上套用資料庫綱要與外鍵設定。
pub fn run_migrations(conn: &mut Connection) -> Result<(), DbError> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let tx = conn.transaction()?;
    tx.execute_batch(MIGRATIONS_SQL)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_create_expected_tables() {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        run_migrations(&mut conn).expect("run migrations");

        // 再次執行驗證等冪性 (IF NOT EXISTS)
        run_migrations(&mut conn).expect("idempotent rerun");

        let tables: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;",
                )
                .expect("prepare");
            let rows = stmt
                .query_map([], |row| row.get(0))
                .expect("query map")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect");
            rows
        };

        let expected = vec![
            "daily_scores",
            "match_players",
            "matches",
            "rating_history",
            "ratings",
            "sessions",
            "users",
        ];
        assert_eq!(tables, expected);

        // 驗證外鍵有效
        let fk_enabled: i32 = conn
            .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
            .expect("pragma fk");
        assert_eq!(fk_enabled, 1);
    }
}
