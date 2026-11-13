use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::users::PublicUser;
use crate::db::DbError;

/// 新建對局紀錄輸入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMatch {
    pub mode_id: String,
    pub num_players: u32,
    pub is_solo: bool,
    pub seed: u64,
    pub replay_json: String,
}

/// 新建對局玩家資訊輸入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMatchPlayer {
    pub seat: u32,
    pub user_id: Option<i64>,
    pub bot_kind: Option<String>,
    pub rank: u32,
    pub score: i32,
}

/// 對局基本資訊。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchRecord {
    pub id: i64,
    pub mode_id: String,
    pub num_players: u32,
    pub is_solo: bool,
    pub seed: u64,
    pub created_at: String,
}

/// 對局玩家資訊。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchPlayerRecord {
    pub match_id: i64,
    pub seat: u32,
    pub user_id: Option<i64>,
    pub bot_kind: Option<String>,
    pub rank: u32,
    pub score: i32,
}

/// 戰績列表中單場對局摘要（符合 SPEC §8）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserMatchSummary {
    pub match_id: i64,
    pub mode_id: String,
    pub created_at: String,
    pub rank: u32,
    pub num_players: u32,
    pub score: i32,
}

/// 每日挑戰分數提交結果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyScoreResult {
    pub date: String,
    pub score: i32,
    pub counted: bool,
    pub rank: u32,
}

/// 每日挑戰排行榜項目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyLeaderboardEntry {
    pub user: PublicUser,
    pub score: i32,
    pub match_id: i64,
}

/// 寫入完整對局紀錄及其參賽玩家（交易中執行）。
pub fn insert_match(
    conn: &mut Connection,
    new_match: &NewMatch,
    players: &[NewMatchPlayer],
) -> Result<i64, DbError> {
    if players.is_empty() {
        return Err(DbError::InvalidData("對局必須至少有一位玩家".to_string()));
    }

    let tx = conn.transaction()?;

    let is_solo_int = if new_match.is_solo { 1 } else { 0 };
    let seed_i64 = new_match.seed as i64;

    tx.execute(
        "INSERT INTO matches (mode_id, num_players, is_solo, seed, replay_json)
         VALUES (?, ?, ?, ?, ?);",
        params![
            new_match.mode_id,
            new_match.num_players,
            is_solo_int,
            seed_i64,
            new_match.replay_json
        ],
    )?;

    let match_id = tx.last_insert_rowid();

    for p in players {
        tx.execute(
            "INSERT INTO match_players (match_id, seat, user_id, bot_kind, rank, score)
             VALUES (?, ?, ?, ?, ?, ?);",
            params![match_id, p.seat, p.user_id, p.bot_kind, p.rank, p.score],
        )?;
    }

    tx.commit()?;
    Ok(match_id)
}

/// 依對局 ID 查詢對局基本資料（不含完整重播字串）。
pub fn get_match(conn: &Connection, match_id: i64) -> Result<Option<MatchRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, mode_id, num_players, is_solo, seed, created_at FROM matches WHERE id = ?;",
    )?;
    let record = stmt
        .query_row(params![match_id], |row| {
            let is_solo_int: i32 = row.get(3)?;
            let seed_i64: i64 = row.get(4)?;
            Ok(MatchRecord {
                id: row.get(0)?,
                mode_id: row.get(1)?,
                num_players: row.get(2)?,
                is_solo: is_solo_int != 0,
                seed: seed_i64 as u64,
                created_at: row.get(5)?,
            })
        })
        .optional()?;
    Ok(record)
}

/// 依對局 ID 查詢重播 JSON 字串。
pub fn get_match_replay(conn: &Connection, match_id: i64) -> Result<Option<String>, DbError> {
    let mut stmt = conn.prepare("SELECT replay_json FROM matches WHERE id = ?;")?;
    let replay = stmt
        .query_row(params![match_id], |row| row.get(0))
        .optional()?;
    Ok(replay)
}

/// 查詢對局的所有參賽玩家紀錄。
pub fn get_match_players(
    conn: &Connection,
    match_id: i64,
) -> Result<Vec<MatchPlayerRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT match_id, seat, user_id, bot_kind, rank, score
         FROM match_players WHERE match_id = ? ORDER BY seat ASC;",
    )?;
    let rows = stmt.query_map(params![match_id], |row| {
        Ok(MatchPlayerRecord {
            match_id: row.get(0)?,
            seat: row.get(1)?,
            user_id: row.get(2)?,
            bot_kind: row.get(3)?,
            rank: row.get(4)?,
            score: row.get(5)?,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// 查詢特定使用者的歷史戰績摘要（最新優先，支援分頁）。
pub fn list_user_matches(
    conn: &Connection,
    user_id: i64,
    limit: u32,
    offset: u32,
) -> Result<Vec<UserMatchSummary>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.mode_id, m.created_at, mp.rank, m.num_players, mp.score
         FROM matches m
         JOIN match_players mp ON m.id = mp.match_id
         WHERE mp.user_id = ?
         ORDER BY m.id DESC
         LIMIT ? OFFSET ?;",
    )?;
    let rows = stmt.query_map(params![user_id, limit, offset], |row| {
        Ok(UserMatchSummary {
            match_id: row.get(0)?,
            mode_id: row.get(1)?,
            created_at: row.get(2)?,
            rank: row.get(3)?,
            num_players: row.get(4)?,
            score: row.get(5)?,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// 記錄每日挑戰分數：同一使用者每日僅首次提交計入榜單 (counted=true)，重複提交 counted=false。
pub fn record_daily_score(
    conn: &Connection,
    user_id: i64,
    date: &str,
    score: i32,
    match_id: i64,
) -> Result<DailyScoreResult, DbError> {
    let existing = get_daily_score(conn, user_id, date)?;
    let (counted, final_score) = match existing {
        Some(old_score) => (false, old_score),
        None => {
            conn.execute(
                "INSERT INTO daily_scores (user_id, date, score, match_id) VALUES (?, ?, ?, ?);",
                params![user_id, date, score, match_id],
            )?;
            (true, score)
        }
    };

    // 計算今日名次：分數愈低愈佳 (UNO 扣分制)，同分依 match_id ASC
    let rank: u32 = conn.query_row(
        "SELECT COUNT(*) + 1 FROM daily_scores
         WHERE date = ? AND (score < ? OR (score = ? AND user_id < ?));",
        params![date, final_score, final_score, user_id],
        |row| row.get(0),
    )?;

    Ok(DailyScoreResult {
        date: date.to_string(),
        score: final_score,
        counted,
        rank,
    })
}

/// 取得特定使用者在某日的挑戰分數。
pub fn get_daily_score(
    conn: &Connection,
    user_id: i64,
    date: &str,
) -> Result<Option<i32>, DbError> {
    let mut stmt =
        conn.prepare("SELECT score FROM daily_scores WHERE user_id = ? AND date = ?;")?;
    let score = stmt
        .query_row(params![user_id, date], |row| row.get(0))
        .optional()?;
    Ok(score)
}

/// 取得指定日期的每日挑戰排行榜（分數升序）。
pub fn get_daily_leaderboard(
    conn: &Connection,
    date: &str,
    limit: u32,
) -> Result<Vec<DailyLeaderboardEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.username, u.display_name, u.is_guest, ds.score, ds.match_id
         FROM daily_scores ds
         JOIN users u ON ds.user_id = u.id
         WHERE ds.date = ?
         ORDER BY ds.score ASC, ds.match_id ASC
         LIMIT ?;",
    )?;
    let rows = stmt.query_map(params![date, limit], |row| {
        let is_guest_int: i32 = row.get(3)?;
        Ok(DailyLeaderboardEntry {
            user: PublicUser {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                is_guest: is_guest_int != 0,
            },
            score: row.get(4)?,
            match_id: row.get(5)?,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_migrations;
    use crate::db::users::create_user;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open memory");
        run_migrations(&mut conn).expect("migrate");
        conn
    }

    #[test]
    fn test_matches_insert_and_queries() {
        let mut conn = setup_test_db();
        let u1 = create_user(&conn, "p1", "Player 1", "h1").expect("u1");
        let u2 = create_user(&conn, "p2", "Player 2", "h2").expect("u2");

        let new_match = NewMatch {
            mode_id: "classic".to_string(),
            num_players: 4,
            is_solo: false,
            seed: 0xDEADBEEF_CAFE1234,
            replay_json: "{\"version\":1}".to_string(),
        };

        let players = vec![
            NewMatchPlayer {
                seat: 0,
                user_id: Some(u1.id),
                bot_kind: None,
                rank: 0,
                score: 0,
            },
            NewMatchPlayer {
                seat: 1,
                user_id: Some(u2.id),
                bot_kind: None,
                rank: 1,
                score: 35,
            },
            NewMatchPlayer {
                seat: 2,
                user_id: None,
                bot_kind: Some("Hard".to_string()),
                rank: 2,
                score: 60,
            },
            NewMatchPlayer {
                seat: 3,
                user_id: None,
                bot_kind: Some("Heuristic".to_string()),
                rank: 3,
                score: 110,
            },
        ];

        let match_id = insert_match(&mut conn, &new_match, &players).expect("insert match");
        assert!(match_id > 0);

        let record = get_match(&conn, match_id).expect("get match").unwrap();
        assert_eq!(record.mode_id, "classic");
        assert_eq!(record.num_players, 4);
        assert!(!record.is_solo);
        assert_eq!(record.seed, 0xDEADBEEF_CAFE1234);

        let replay = get_match_replay(&conn, match_id).expect("get replay");
        assert_eq!(replay.as_deref(), Some("{\"version\":1}"));

        let match_players = get_match_players(&conn, match_id).expect("players");
        assert_eq!(match_players.len(), 4);
        assert_eq!(match_players[0].user_id, Some(u1.id));
        assert_eq!(match_players[2].bot_kind.as_deref(), Some("Hard"));

        // 查詢 u1 的戰績列表
        let summaries = list_user_matches(&conn, u1.id, 10, 0).expect("list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].match_id, match_id);
        assert_eq!(summaries[0].rank, 0);
        assert_eq!(summaries[0].score, 0);
    }

    #[test]
    fn test_daily_challenge_submission_and_counted() {
        let conn = setup_test_db();
        let u1 = create_user(&conn, "u1", "User 1", "h").expect("u1");
        let u2 = create_user(&conn, "u2", "User 2", "h").expect("u2");

        let today = "2026-11-13";

        // u1 首次提交
        let res1 = record_daily_score(&conn, u1.id, today, 45, 101).expect("u1 submit");
        assert!(res1.counted);
        assert_eq!(res1.score, 45);
        assert_eq!(res1.rank, 1);

        // u1 第二次提交 -> counted 應為 false，保留原分數 45
        let res1_repeat =
            record_daily_score(&conn, u1.id, today, 20, 102).expect("u1 repeat submit");
        assert!(!res1_repeat.counted);
        assert_eq!(res1_repeat.score, 45);

        // u2 提交更佳分數 15 -> u2 應成為 rank 1
        let res2 = record_daily_score(&conn, u2.id, today, 15, 103).expect("u2 submit");
        assert!(res2.counted);
        assert_eq!(res2.rank, 1);

        // 排行榜檢查
        let lb = get_daily_leaderboard(&conn, today, 10).expect("leaderboard");
        assert_eq!(lb.len(), 2);
        assert_eq!(lb[0].user.id, u2.id);
        assert_eq!(lb[0].score, 15);
        assert_eq!(lb[1].user.id, u1.id);
        assert_eq!(lb[1].score, 45);
    }
}
