//! 戰績、排行榜、公開房間與重播 API（INTERFACES §8、PLAN T8.1）。

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::api::ApiError;
use crate::db::matches::{self, UserMatchSummary};
use crate::db::ratings::{self, LeaderboardEntry, RatingHistoryEntry};
use crate::db::users::PublicUser;
use crate::state::AppState;
use uno_protocol::RoomInfo;

/// `GET /api/users/:id/stats` 回應。
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UserStatsResponse {
    pub games: i32,
    pub wins: i32,
    pub by_mode: BTreeMap<String, ratings::ModeStats>,
    pub elo_history: Vec<RatingHistoryEntry>,
}

pub async fn user_stats(
    Path(user_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<UserStatsResponse>, ApiError> {
    let (stats, history) = state
        .db()
        .run(move |conn| {
            let s = ratings::get_user_stats(conn, user_id)?;
            let h = ratings::get_rating_history(conn, user_id, None)?;
            Ok((s, h))
        })
        .await?;
    Ok(Json(UserStatsResponse {
        games: stats.games,
        wins: stats.wins,
        by_mode: stats.by_mode,
        elo_history: history,
    }))
}

#[derive(Debug, Deserialize)]
pub struct MatchesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub async fn user_matches(
    Path(user_id): Path<i64>,
    Query(q): Query<MatchesQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<UserMatchSummary>>, ApiError> {
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0);
    let list = state
        .db()
        .run(move |conn| matches::list_user_matches(conn, user_id, limit, offset))
        .await?;
    Ok(Json(list))
}

/// `GET /api/matches/:id/replay`：回傳原始 Replay JSON。
pub async fn match_replay(
    Path(match_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let replay = state
        .db()
        .run(move |conn| matches::get_match_replay(conn, match_id))
        .await?
        .ok_or_else(|| ApiError::NotFound("match not found".to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&replay).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(value))
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub mode: Option<String>,
    pub limit: Option<u32>,
}

/// 排行榜回應項目（與 §8 的 `[{user, elo, games}]` 對齊，額外附 wins）。
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct LeaderboardResponseEntry {
    pub user: PublicUser,
    pub elo: i32,
    pub games: i32,
    pub wins: i32,
}

pub async fn leaderboard(
    Query(q): Query<LeaderboardQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<LeaderboardResponseEntry>>, ApiError> {
    let mode = q.mode.unwrap_or_else(|| "classic".to_string());
    let limit = q.limit.unwrap_or(50).min(200);
    let entries: Vec<LeaderboardEntry> = state
        .db()
        .run(move |conn| ratings::get_leaderboard(conn, &mode, limit))
        .await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|e| LeaderboardResponseEntry {
                user: e.user,
                elo: e.elo,
                games: e.games,
                wins: e.wins,
            })
            .collect(),
    ))
}

/// `GET /api/rooms`：公開且等待中的房間。
pub async fn public_rooms(State(state): State<AppState>) -> Result<Json<Vec<RoomInfo>>, ApiError> {
    Ok(Json(state.rooms().list_public().await))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::matches::{NewMatch, NewMatchPlayer};
    use crate::db::users;
    use crate::db::Db;
    use std::path::PathBuf;

    fn test_state(db: Db) -> AppState {
        let cfg = crate::config::ServerConfig {
            port: 0,
            host: "127.0.0.1".to_string(),
            data_dir: PathBuf::from("data"),
            web_dir: PathBuf::from("/tmp/web"),
            models_dir: PathBuf::from("/tmp/models"),
        };
        AppState::new(cfg, 0, vec!["http://127.0.0.1".to_string()], db)
    }

    async fn seed_match(db: &Db) -> (i64, i64, i64) {
        db.run(|conn| {
            let u1 = users::create_user(conn, "stats_a", "Stats A", "h")?;
            let u2 = users::create_user(conn, "stats_b", "Stats B", "h")?;
            let mid = matches::insert_match(
                conn,
                &NewMatch {
                    mode_id: "classic".to_string(),
                    num_players: 2,
                    is_solo: false,
                    seed: 7,
                    replay_json: "{\"replay_version\":1}".to_string(),
                },
                &[
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
                        score: 42,
                    },
                ],
            )?;
            let outcomes = vec![
                ratings::PlayerMatchOutcome {
                    user_id: Some(u1.id),
                    seat: 0,
                    rank: 0,
                    team: None,
                },
                ratings::PlayerMatchOutcome {
                    user_id: Some(u2.id),
                    seat: 1,
                    rank: 1,
                    team: None,
                },
            ];
            ratings::apply_match_ratings(conn, mid, "classic", false, false, &outcomes)?;
            Ok((mid, u1.id, u2.id))
        })
        .await
        .expect("seed")
    }

    #[tokio::test]
    async fn stats_api_user_stats_matches_leaderboard_replay() {
        let db = Db::open_in_memory().expect("db");
        let (match_id, u1, u2) = seed_match(&db).await;
        let state = test_state(db);

        let stats = user_stats(Path(u1), State(state.clone()))
            .await
            .expect("stats")
            .0;
        assert_eq!(stats.games, 1);
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.by_mode["classic"].elo, 1216);
        assert_eq!(stats.elo_history.len(), 1);
        assert_eq!(stats.elo_history[0].elo, 1216);

        let list = user_matches(
            Path(u1),
            Query(MatchesQuery {
                limit: None,
                offset: None,
            }),
            State(state.clone()),
        )
        .await
        .expect("matches")
        .0;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].match_id, match_id);
        assert_eq!(list[0].rank, 0);

        let lb = leaderboard(
            Query(LeaderboardQuery {
                mode: None,
                limit: None,
            }),
            State(state.clone()),
        )
        .await
        .expect("lb")
        .0;
        assert_eq!(lb.len(), 2);
        assert_eq!(lb[0].user.id, u1);
        assert_eq!(lb[1].user.id, u2);
        assert!(lb[0].elo > lb[1].elo);

        let replay = match_replay(Path(match_id), State(state.clone()))
            .await
            .expect("replay")
            .0;
        assert_eq!(replay["replay_version"], 1);

        let missing = match_replay(Path(9999), State(state)).await;
        assert!(matches!(missing, Err(ApiError::NotFound(_))));
    }
}
