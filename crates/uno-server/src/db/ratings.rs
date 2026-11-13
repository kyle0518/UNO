use std::collections::{BTreeMap, HashMap};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::users::PublicUser;
use crate::db::DbError;

/// 評分紀錄（單一模式）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RatingRecord {
    pub user_id: i64,
    pub mode_id: String,
    pub elo: i32,
    pub games: i32,
    pub wins: i32,
}

/// 評分變更歷史紀錄點（符合 INTERFACES §8）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RatingHistoryEntry {
    pub t: String,
    pub mode_id: String,
    pub elo: i32,
}

/// 排行榜項目（符合 INTERFACES §8）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaderboardEntry {
    pub user: PublicUser,
    pub elo: i32,
    pub games: i32,
    pub wins: i32,
}

/// 模式統計摘要。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ModeStats {
    pub games: i32,
    pub wins: i32,
    pub elo: i32,
}

/// 使用者整體統計摘要。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct UserStats {
    pub games: i32,
    pub wins: i32,
    pub by_mode: BTreeMap<String, ModeStats>,
}

/// 對局結算時單一玩家的輸入結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMatchOutcome {
    pub user_id: Option<i64>,
    pub seat: u32,
    pub rank: u32,
    pub team: Option<u8>,
}

/// 單一使用者的 Elo 變動。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EloDelta {
    pub user_id: i64,
    pub seat: u32,
    pub old_elo: i32,
    pub new_elo: i32,
    pub delta: i32,
}

/// 依 SPEC §6 與 INTERFACES §9 計算 Elo 變動：
/// - 單機對局 (is_solo) 或真人玩家數 < 2 時不計算 Elo。
/// - 個人賽：對每對真人 (i, j)：
///   `S = 1 if rank_i < rank_j, 0.5 if 相等, else 0`
///   `E = 1 / (1 + 10^((R_j - R_i) / 400))`
///   `ΔR_i = 32 × Σ(S - E) / (真人數 - 1)`，四捨五入。
/// - 團隊賽：以隊伍平均 Elo 計算，同隊成員變動相同。
pub fn calculate_elo_deltas(
    is_solo: bool,
    is_team: bool,
    outcomes: &[PlayerMatchOutcome],
    current_elos: &HashMap<i64, i32>,
) -> Vec<EloDelta> {
    if is_solo {
        return Vec::new();
    }

    let humans: Vec<&PlayerMatchOutcome> =
        outcomes.iter().filter(|o| o.user_id.is_some()).collect();
    if humans.len() < 2 {
        return Vec::new();
    }

    if is_team {
        // 團隊模式：以隊伍平均 Elo 計算
        let mut team0_humans: Vec<&PlayerMatchOutcome> = Vec::new();
        let mut team1_humans: Vec<&PlayerMatchOutcome> = Vec::new();

        for h in &humans {
            match h.team {
                Some(0) => team0_humans.push(h),
                Some(1) => team1_humans.push(h),
                _ => {}
            }
        }

        // 必須兩隊都有真人參與才計算隊伍 Elo
        if team0_humans.is_empty() || team1_humans.is_empty() {
            return Vec::new();
        }

        let team0_elo_sum: i64 = team0_humans
            .iter()
            .map(|h| *current_elos.get(&h.user_id.unwrap()).unwrap_or(&1200) as i64)
            .sum();
        let avg_r0 = team0_elo_sum as f64 / team0_humans.len() as f64;

        let team1_elo_sum: i64 = team1_humans
            .iter()
            .map(|h| *current_elos.get(&h.user_id.unwrap()).unwrap_or(&1200) as i64)
            .sum();
        let avg_r1 = team1_elo_sum as f64 / team1_humans.len() as f64;

        let rank0 = team0_humans.iter().map(|h| h.rank).min().unwrap_or(0);
        let rank1 = team1_humans.iter().map(|h| h.rank).min().unwrap_or(0);

        let s0 = if rank0 < rank1 {
            1.0
        } else if rank0 == rank1 {
            0.5
        } else {
            0.0
        };
        let s1 = 1.0 - s0;

        let e0 = 1.0 / (1.0 + 10.0_f64.powf((avg_r1 - avg_r0) / 400.0));
        let e1 = 1.0 - e0;

        let delta0 = (32.0 * (s0 - e0)).round() as i32;
        let delta1 = (32.0 * (s1 - e1)).round() as i32;

        let mut deltas = Vec::new();
        for h in team0_humans {
            let uid = h.user_id.unwrap();
            let old_elo = *current_elos.get(&uid).unwrap_or(&1200);
            let new_elo = (old_elo + delta0).max(100);
            deltas.push(EloDelta {
                user_id: uid,
                seat: h.seat,
                old_elo,
                new_elo,
                delta: new_elo - old_elo,
            });
        }
        for h in team1_humans {
            let uid = h.user_id.unwrap();
            let old_elo = *current_elos.get(&uid).unwrap_or(&1200);
            let new_elo = (old_elo + delta1).max(100);
            deltas.push(EloDelta {
                user_id: uid,
                seat: h.seat,
                old_elo,
                new_elo,
                delta: new_elo - old_elo,
            });
        }
        deltas
    } else {
        // 個人模式
        let num_humans = humans.len();
        let denom = (num_humans - 1) as f64;
        let mut deltas = Vec::new();

        for (i, p_i) in humans.iter().enumerate() {
            let uid_i = p_i.user_id.unwrap();
            let r_i = *current_elos.get(&uid_i).unwrap_or(&1200) as f64;
            let mut sum_diff = 0.0;

            for (j, p_j) in humans.iter().enumerate() {
                if i == j {
                    continue;
                }
                let uid_j = p_j.user_id.unwrap();
                let r_j = *current_elos.get(&uid_j).unwrap_or(&1200) as f64;

                let s = if p_i.rank < p_j.rank {
                    1.0
                } else if p_i.rank == p_j.rank {
                    0.5
                } else {
                    0.0
                };
                let e = 1.0 / (1.0 + 10.0_f64.powf((r_j - r_i) / 400.0));
                sum_diff += s - e;
            }

            let delta = (32.0 * sum_diff / denom).round() as i32;
            let old_elo = r_i as i32;
            let new_elo = (old_elo + delta).max(100);
            deltas.push(EloDelta {
                user_id: uid_i,
                seat: p_i.seat,
                old_elo,
                new_elo,
                delta: new_elo - old_elo,
            });
        }
        deltas
    }
}

/// 查詢使用者各模式的 Elo 數值映射。
pub fn get_user_ratings(conn: &Connection, user_id: i64) -> Result<BTreeMap<String, i32>, DbError> {
    let mut stmt = conn.prepare("SELECT mode_id, elo FROM ratings WHERE user_id = ?;")?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;
    let mut map = BTreeMap::new();
    for r in rows {
        let (mode, elo) = r?;
        map.insert(mode, elo);
    }
    Ok(map)
}

/// 查詢使用者在單一模式的評分紀錄（若不存在則給予預設值 1200, 0, 0）。
pub fn get_user_rating(
    conn: &Connection,
    user_id: i64,
    mode_id: &str,
) -> Result<RatingRecord, DbError> {
    let mut stmt =
        conn.prepare("SELECT elo, games, wins FROM ratings WHERE user_id = ? AND mode_id = ?;")?;
    let record = stmt
        .query_row(params![user_id, mode_id], |row| {
            Ok(RatingRecord {
                user_id,
                mode_id: mode_id.to_string(),
                elo: row.get(0)?,
                games: row.get(1)?,
                wins: row.get(2)?,
            })
        })
        .optional()?;

    Ok(record.unwrap_or(RatingRecord {
        user_id,
        mode_id: mode_id.to_string(),
        elo: 1200,
        games: 0,
        wins: 0,
    }))
}

/// 查詢使用者的 Elo 變動歷史。
pub fn get_rating_history(
    conn: &Connection,
    user_id: i64,
    limit: Option<u32>,
) -> Result<Vec<RatingHistoryEntry>, DbError> {
    let limit_val = limit.unwrap_or(500);
    let mut stmt = conn.prepare(
        "SELECT created_at, mode_id, elo FROM rating_history
         WHERE user_id = ? ORDER BY id ASC LIMIT ?;",
    )?;
    let rows = stmt.query_map(params![user_id, limit_val], |row| {
        Ok(RatingHistoryEntry {
            t: row.get(0)?,
            mode_id: row.get(1)?,
            elo: row.get(2)?,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// 查詢排行榜（限有參與局數 games > 0 者）。
pub fn get_leaderboard(
    conn: &Connection,
    mode_id: &str,
    limit: u32,
) -> Result<Vec<LeaderboardEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.username, u.display_name, u.is_guest, r.elo, r.games, r.wins
         FROM ratings r
         JOIN users u ON r.user_id = u.id
         WHERE r.mode_id = ? AND r.games > 0
         ORDER BY r.elo DESC, r.wins DESC, r.games DESC
         LIMIT ?;",
    )?;
    let rows = stmt.query_map(params![mode_id, limit], |row| {
        let is_guest_int: i32 = row.get(3)?;
        Ok(LeaderboardEntry {
            user: PublicUser {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                is_guest: is_guest_int != 0,
            },
            elo: row.get(4)?,
            games: row.get(5)?,
            wins: row.get(6)?,
        })
    })?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// 查詢使用者的整體統計（包含各模式戰績與累積勝場）。
pub fn get_user_stats(conn: &Connection, user_id: i64) -> Result<UserStats, DbError> {
    let mut stmt =
        conn.prepare("SELECT mode_id, games, wins, elo FROM ratings WHERE user_id = ?;")?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;

    let mut by_mode = BTreeMap::new();
    let mut total_games = 0;
    let mut total_wins = 0;

    for r in rows {
        let (mode, games, wins, elo) = r?;
        total_games += games;
        total_wins += wins;
        by_mode.insert(mode, ModeStats { games, wins, elo });
    }

    Ok(UserStats {
        games: total_games,
        wins: total_wins,
        by_mode,
    })
}

/// 將對局結果結算至評分系統（更新 ratings 表與寫入 rating_history）。
pub fn apply_match_ratings(
    conn: &mut Connection,
    match_id: i64,
    mode_id: &str,
    is_solo: bool,
    is_team: bool,
    outcomes: &[PlayerMatchOutcome],
) -> Result<Vec<EloDelta>, DbError> {
    let tx = conn.transaction()?;

    // 1. 取得所有真人當前的 Elo
    let mut current_elos = HashMap::new();
    for o in outcomes {
        if let Some(uid) = o.user_id {
            if let std::collections::hash_map::Entry::Vacant(e) = current_elos.entry(uid) {
                let mut stmt =
                    tx.prepare("SELECT elo FROM ratings WHERE user_id = ? AND mode_id = ?;")?;
                let elo: i32 = stmt
                    .query_row(params![uid, mode_id], |row| row.get(0))
                    .optional()?
                    .unwrap_or(1200);
                e.insert(elo);
            }
        }
    }

    // 2. 計算 Elo 變動
    let deltas = calculate_elo_deltas(is_solo, is_team, outcomes, &current_elos);
    let delta_map: HashMap<i64, &EloDelta> = deltas.iter().map(|d| (d.user_id, d)).collect();

    // 3. 更新每一位真人玩家的 ratings 與 rating_history
    for o in outcomes {
        if let Some(uid) = o.user_id {
            let is_win = o.rank == 0;
            let win_inc = if is_win { 1 } else { 0 };
            let current_elo = *current_elos.get(&uid).unwrap_or(&1200);
            let final_elo = delta_map
                .get(&uid)
                .map(|d| d.new_elo)
                .unwrap_or(current_elo);

            tx.execute(
                "INSERT INTO ratings (user_id, mode_id, elo, games, wins)
                 VALUES (?, ?, ?, 1, ?)
                 ON CONFLICT(user_id, mode_id) DO UPDATE SET
                   elo = excluded.elo,
                   games = ratings.games + 1,
                   wins = ratings.wins + excluded.wins;",
                params![uid, mode_id, final_elo, win_inc],
            )?;

            // 僅在 Elo 實際計算 (非 solo 且真人數 >= 2) 時寫入 rating_history
            if delta_map.contains_key(&uid) {
                tx.execute(
                    "INSERT INTO rating_history (user_id, mode_id, elo, match_id)
                     VALUES (?, ?, ?, ?);",
                    params![uid, mode_id, final_elo, match_id],
                )?;
            }
        }
    }

    tx.commit()?;
    Ok(deltas)
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
    fn test_calculate_elo_4_players_symmetric() {
        let outcomes = vec![
            PlayerMatchOutcome {
                user_id: Some(1),
                seat: 0,
                rank: 0,
                team: None,
            },
            PlayerMatchOutcome {
                user_id: Some(2),
                seat: 1,
                rank: 1,
                team: None,
            },
            PlayerMatchOutcome {
                user_id: Some(3),
                seat: 2,
                rank: 2,
                team: None,
            },
            PlayerMatchOutcome {
                user_id: Some(4),
                seat: 3,
                rank: 3,
                team: None,
            },
        ];

        let mut current_elos = HashMap::new();
        current_elos.insert(1, 1200);
        current_elos.insert(2, 1200);
        current_elos.insert(3, 1200);
        current_elos.insert(4, 1200);

        let deltas = calculate_elo_deltas(false, false, &outcomes, &current_elos);
        assert_eq!(deltas.len(), 4);

        let d1 = deltas.iter().find(|d| d.user_id == 1).unwrap();
        let d2 = deltas.iter().find(|d| d.user_id == 2).unwrap();
        let d3 = deltas.iter().find(|d| d.user_id == 3).unwrap();
        let d4 = deltas.iter().find(|d| d.user_id == 4).unwrap();

        assert_eq!(d1.delta, 16);
        assert_eq!(d1.new_elo, 1216);
        assert_eq!(d2.delta, 5);
        assert_eq!(d2.new_elo, 1205);
        assert_eq!(d3.delta, -5);
        assert_eq!(d3.new_elo, 1195);
        assert_eq!(d4.delta, -16);
        assert_eq!(d4.new_elo, 1184);

        let sum_deltas: i32 = deltas.iter().map(|d| d.delta).sum();
        assert_eq!(sum_deltas, 0);
    }

    #[test]
    fn test_calculate_elo_team_2v2() {
        let outcomes = vec![
            PlayerMatchOutcome {
                user_id: Some(1),
                seat: 0,
                rank: 0,
                team: Some(0),
            },
            PlayerMatchOutcome {
                user_id: Some(2),
                seat: 1,
                rank: 1,
                team: Some(1),
            },
            PlayerMatchOutcome {
                user_id: Some(3),
                seat: 2,
                rank: 0,
                team: Some(0),
            },
            PlayerMatchOutcome {
                user_id: Some(4),
                seat: 3,
                rank: 1,
                team: Some(1),
            },
        ];

        let mut current_elos = HashMap::new();
        current_elos.insert(1, 1200);
        current_elos.insert(2, 1200);
        current_elos.insert(3, 1200);
        current_elos.insert(4, 1200);

        let deltas = calculate_elo_deltas(false, true, &outcomes, &current_elos);
        assert_eq!(deltas.len(), 4);

        // 同隊變動相同
        let d1 = deltas.iter().find(|d| d.user_id == 1).unwrap();
        let d3 = deltas.iter().find(|d| d.user_id == 3).unwrap();
        let d2 = deltas.iter().find(|d| d.user_id == 2).unwrap();
        let d4 = deltas.iter().find(|d| d.user_id == 4).unwrap();

        assert_eq!(d1.delta, 16);
        assert_eq!(d3.delta, 16);
        assert_eq!(d2.delta, -16);
        assert_eq!(d4.delta, -16);
    }

    #[test]
    fn test_solo_and_rating_application() {
        let mut conn = setup_test_db();
        let u1 = create_user(&conn, "u1", "User 1", "h").expect("u1");

        // 單人局 vs 3 bot
        let outcomes = vec![
            PlayerMatchOutcome {
                user_id: Some(u1.id),
                seat: 0,
                rank: 0,
                team: None,
            },
            PlayerMatchOutcome {
                user_id: None,
                seat: 1,
                rank: 1,
                team: None,
            },
            PlayerMatchOutcome {
                user_id: None,
                seat: 2,
                rank: 2,
                team: None,
            },
            PlayerMatchOutcome {
                user_id: None,
                seat: 3,
                rank: 3,
                team: None,
            },
        ];

        let deltas = apply_match_ratings(&mut conn, 50, "classic", true, false, &outcomes)
            .expect("apply solo");
        assert!(deltas.is_empty(), "Solo 對局不計 Elo");

        // 雖然不計 Elo，但場次與勝場必須累加
        let stats = get_user_stats(&conn, u1.id).expect("stats");
        assert_eq!(stats.games, 1);
        assert_eq!(stats.wins, 1);
        let classic_stat = stats.by_mode.get("classic").unwrap();
        assert_eq!(classic_stat.elo, 1200);
        assert_eq!(classic_stat.games, 1);
        assert_eq!(classic_stat.wins, 1);

        // 排行榜必須能查到該使用者
        let lb = get_leaderboard(&conn, "classic", 10).expect("lb");
        assert_eq!(lb.len(), 1);
        assert_eq!(lb[0].user.id, u1.id);
        assert_eq!(lb[0].games, 1);
    }
}
