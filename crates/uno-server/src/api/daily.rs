//! 每日挑戰與模型清單 API（INTERFACES §8、PLAN T8.4/T10.5）。

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use uno_core::{Replay, RuleConfig};

use crate::api::matches_api::persist_and_score;
use crate::api::ApiError;
use crate::auth::AuthUser;
use crate::db::matches;
use crate::state::AppState;

/// UTC 日期（YYYY-MM-DD）與每日種子（與前端 getDailySeed 對齊：y*10000+m*100+d）。
pub fn daily_date_seed(now_secs: u64) -> (String, u64) {
    let days = (now_secs / 86_400) as i64;
    // Howard Hinnant civil-from-days 演算法
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    let date = format!("{year:04}-{m:02}-{d:02}");
    let seed = (year * 10_000 + m as i64 * 100 + d as i64) as u64;
    (date, seed)
}

fn today_daily() -> (String, u64, RuleConfig) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (date, seed) = daily_date_seed(secs);
    let config = RuleConfig::preset("daily", 4)
        .unwrap_or_else(|_| RuleConfig::preset("classic", 4).expect("classic preset"));
    (date, seed, config)
}

#[derive(Debug, Serialize)]
pub struct DailyInfoResponse {
    pub date: String,
    pub seed: u64,
    pub config: RuleConfig,
    pub my_best: Option<i32>,
}

/// `GET /api/daily`（登入選填：有 AuthUser 時附 my_best；未登入經 OptionalAuthUser）。
pub async fn get_daily(
    auth: crate::auth::OptionalAuthUser,
    State(state): State<AppState>,
) -> Result<Json<DailyInfoResponse>, ApiError> {
    let (date, seed, config) = today_daily();
    let my_best = match auth.0 {
        Some(user) => {
            let date2 = date.clone();
            state
                .db()
                .run(move |conn| matches::get_daily_score(conn, user.id, &date2))
                .await?
        }
        None => None,
    };
    Ok(Json(DailyInfoResponse {
        date,
        seed,
        config,
        my_best,
    }))
}

#[derive(Debug, Serialize)]
pub struct DailySubmitResponse {
    pub score: i32,
    pub counted: bool,
    pub rank: u32,
}

/// `POST /api/daily/submit`：驗證 seed/config 符合當日後入庫並計入每日榜。
pub async fn submit_daily(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(replay): Json<Replay>,
) -> Result<Json<DailySubmitResponse>, ApiError> {
    let user = auth_user.0;
    let (date, seed, _config) = today_daily();
    if replay.seed != seed || replay.config.mode_id != "daily" {
        return Err(ApiError::BadRequest(
            "replay does not match today's daily challenge".to_string(),
        ));
    }
    let user_id = user.id;
    let (score, counted, rank) = state
        .db()
        .run(move |conn| {
            let (match_id, seat0_score) = persist_and_score(conn, &replay, user_id, false)?;
            let result = matches::record_daily_score(conn, user_id, &date, seat0_score, match_id)?;
            Ok((result.score, result.counted, result.rank))
        })
        .await
        .map_err(|e| match e {
            crate::db::DbError::InvalidData(m) => ApiError::BadRequest(m),
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(DailySubmitResponse {
        score,
        counted,
        rank,
    }))
}

/// `GET /api/models`：models 目錄下所有 `*.json` 的內容陣列。
pub async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let dir = state.config().models_dir.clone();
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(Json(out)),
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    paths.sort();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(obj) = value.as_object_mut() {
            if !obj.contains_key("name") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    obj.insert(
                        "name".to_string(),
                        serde_json::Value::String(stem.to_string()),
                    );
                }
            }
        }
        out.push(value);
    }
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::users;
    use crate::db::Db;
    use std::path::PathBuf;
    use uno_core::{GameState, ReplayPlayer, ReplayPlayerKind};

    fn test_state(db: Db) -> AppState {
        AppState::new(
            crate::config::ServerConfig {
                port: 0,
                host: "127.0.0.1".to_string(),
                data_dir: PathBuf::from("data"),
                web_dir: PathBuf::from("/tmp/web"),
                models_dir: PathBuf::from("/tmp/models"),
            },
            0,
            vec!["http://127.0.0.1".to_string()],
            db,
        )
    }

    fn played_daily_replay(seed: u64) -> Replay {
        let mut config = RuleConfig::preset("daily", 4).expect("daily preset");
        config.scoring = uno_core::Scoring::SingleRound;
        let mut game = GameState::new(config, seed).expect("game");
        let mut rng = uno_core::Rng::new(777);
        let mut guard = 0;
        while !game.is_over() {
            guard += 1;
            assert!(guard < 30000, "daily game should finish");
            if game.phase() == uno_core::Phase::RoundOver {
                game.next_round().expect("next");
                continue;
            }
            let Some(dp) = game.decision_player() else {
                break;
            };
            let legal = game.legal_actions(dp);
            if legal.is_empty() {
                break;
            }
            let idx = (rng.next_u64() % legal.len() as u64) as usize;
            game.apply(dp, legal[idx]).expect("apply");
            if let Some(target) = game.uno_window() {
                for seat in 0..4u8 {
                    if seat != target
                        && game
                            .legal_actions(seat)
                            .contains(&uno_core::Action::CatchUno { target })
                    {
                        let _ = game.apply(seat, uno_core::Action::CatchUno { target });
                        break;
                    }
                }
            }
        }
        assert!(game.is_over());
        let players = (0..4u8)
            .map(|seat| ReplayPlayer {
                seat,
                name: format!("P{seat}"),
                user_id: None,
                kind: if seat == 0 {
                    ReplayPlayerKind::Human
                } else {
                    ReplayPlayerKind::Bot
                },
                bot: if seat == 0 {
                    None
                } else {
                    Some("hard".to_string())
                },
            })
            .collect();
        game.to_replay(players, "2026-11-16T00:00:00Z")
    }

    #[tokio::test]
    async fn daily_submit_counted_once() {
        let db = Db::open_in_memory().expect("db");
        let user = db
            .run(|conn| users::create_user(conn, "daily_user", "Daily", "h"))
            .await
            .expect("user");
        let state = test_state(db);
        let (_date, seed, _cfg) = today_daily();
        let replay = played_daily_replay(seed);

        let first = submit_daily(
            AuthUser(user.clone()),
            State(state.clone()),
            Json(replay.clone()),
        )
        .await
        .expect("first submit");
        assert!(first.0.counted);
        assert_eq!(first.0.rank, 1);

        let second = submit_daily(AuthUser(user), State(state), Json(replay))
            .await
            .expect("second submit");
        assert!(!second.0.counted);
        assert_eq!(second.0.score, first.0.score);
    }

    #[test]
    fn daily_date_seed_known_dates() {
        // 2026-11-16 00:00:00 UTC = 1794441600? 以 civil 演算法驗證已知日期：
        // 1970-01-01
        assert_eq!(daily_date_seed(0), ("1970-01-01".to_string(), 19700101));
        // 2000-02-29（閏日）= 951782400
        assert_eq!(
            daily_date_seed(951_782_400),
            ("2000-02-29".to_string(), 20000229)
        );
        // 2026-01-01 = 1767225600
        assert_eq!(
            daily_date_seed(1_767_225_600),
            ("2026-01-01".to_string(), 20260101)
        );
    }
}
