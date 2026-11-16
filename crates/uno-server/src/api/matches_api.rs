//! 單機上傳與每日挑戰 API（INTERFACES §8、PLAN T8.2/T8.4）。
//! 兩者皆以 `uno_core::Replay::replay()` 在伺服器重跑驗證，禁止竄改。

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use uno_core::{Replay, ReplayPlayerKind};

use crate::api::ApiError;
use crate::auth::AuthUser;
use crate::db::matches::{self, NewMatch, NewMatchPlayer};
use crate::db::ratings;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct SoloMatchResponse {
    pub match_id: i64,
}

/// 將已驗證的重播寫入 DB（共用於 solo 與 daily）。
fn persist_verified_match(
    conn: &mut rusqlite::Connection,
    replay: &Replay,
    user_id: i64,
    is_solo: bool,
) -> Result<(i64, Vec<u32>, Vec<i32>), crate::db::DbError> {
    let (game, _events) = replay
        .replay()
        .map_err(|e| crate::db::DbError::InvalidData(format!("replay verification failed: {e}")))?;
    if !game.is_over() {
        return Err(crate::db::DbError::InvalidData(
            "replay does not reach game over".to_string(),
        ));
    }
    let rankings = game.rankings().map(|r| r.to_vec()).unwrap_or_default();
    let scores = game.scores().to_vec();
    let rank_of = |seat: uno_core::Seat| -> u32 {
        rankings
            .iter()
            .position(|&s| s == seat)
            .map(|p| p as u32)
            .unwrap_or(u32::from(seat))
    };
    let mut players = Vec::new();
    let mut outcomes = Vec::new();
    for p in &replay.players {
        let seat = p.seat;
        let is_self = seat == 0;
        let uid = if is_self { Some(user_id) } else { p.user_id };
        players.push(NewMatchPlayer {
            seat: u32::from(seat),
            user_id: uid,
            bot_kind: p.bot.clone(),
            rank: rank_of(seat),
            score: scores.get(usize::from(seat)).copied().unwrap_or(0),
        });
        outcomes.push(ratings::PlayerMatchOutcome {
            user_id: uid,
            seat: u32::from(seat),
            rank: rank_of(seat),
            team: game.team_of_seat(seat),
        });
    }
    let replay_json = serde_json::to_string(replay)
        .map_err(|e| crate::db::DbError::InvalidData(e.to_string()))?;
    let match_id = matches::insert_match(
        conn,
        &NewMatch {
            mode_id: replay.config.mode_id.clone(),
            num_players: replay.config.num_players as u32,
            is_solo,
            seed: replay.seed,
            replay_json,
        },
        &players,
    )?;
    let is_team = replay.config.teams.is_some();
    ratings::apply_match_ratings(
        conn,
        match_id,
        &replay.config.mode_id,
        is_solo,
        is_team,
        &outcomes,
    )?;
    let ranks: Vec<u32> = players.iter().map(|p| p.rank).collect();
    Ok((match_id, ranks, scores))
}

/// 驗證並入庫，回傳 (match_id, seat0 最終分數)。供 daily submit 共用。
pub(crate) fn persist_and_score(
    conn: &mut rusqlite::Connection,
    replay: &Replay,
    user_id: i64,
    is_solo: bool,
) -> Result<(i64, i32), crate::db::DbError> {
    validate_seat_zero(replay, user_id)
        .map_err(|e| crate::db::DbError::InvalidData(e.to_string()))?;
    let (match_id, _ranks, scores) = persist_verified_match(conn, replay, user_id, is_solo)?;
    Ok((match_id, scores.first().copied().unwrap_or(0)))
}

pub(crate) fn validate_seat_zero(replay: &Replay, user_id: i64) -> Result<(), ApiError> {
    let p0 = replay
        .players
        .iter()
        .find(|p| p.seat == 0)
        .ok_or_else(|| ApiError::BadRequest("replay missing seat 0".to_string()))?;
    if p0.kind != ReplayPlayerKind::Human {
        return Err(ApiError::BadRequest("seat 0 must be human".to_string()));
    }
    if let Some(uid) = p0.user_id {
        if uid != user_id {
            return Err(ApiError::BadRequest(
                "seat 0 does not belong to the requesting user".to_string(),
            ));
        }
    }
    Ok(())
}

/// `POST /api/matches/solo`：驗證單機重播後入庫（不計 Elo，計 games/wins）。
pub async fn submit_solo_match(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(replay): Json<Replay>,
) -> Result<Json<SoloMatchResponse>, ApiError> {
    let user = auth_user.0;
    validate_seat_zero(&replay, user.id)?;
    let user_id = user.id;
    let (match_id, _ranks, _scores) = state
        .db()
        .run(move |conn| persist_verified_match(conn, &replay, user_id, true))
        .await?;
    Ok(Json(SoloMatchResponse { match_id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::db::users;
    use crate::db::Db;
    use std::path::PathBuf;
    use uno_core::{GameState, ReplayPlayer, RuleConfig};

    fn played_replay() -> Replay {
        let mut config = RuleConfig::preset("classic", 2).expect("preset");
        config.scoring = uno_core::Scoring::SingleRound;
        let mut game = GameState::new(config, 99).expect("game");
        let mut rng = uno_core::Rng::new(12345);
        let mut guard = 0;
        while !game.is_over() {
            guard += 1;
            assert!(guard < 20000, "game should finish");
            if game.phase() == uno_core::Phase::RoundOver {
                game.next_round().expect("next round");
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
            let action = legal[idx];
            // 非決策者的 catch/call 由 legal_actions 提供，decision 迴圈外另行處理最小化：
            game.apply(dp, action).expect("apply");
            // 處理 UNO 視窗：隨機一位可 catch 就 catch（讓對局能推進）
            if let Some(target) = game.uno_window() {
                for seat in 0..2u8 {
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
        let players = vec![
            ReplayPlayer {
                seat: 0,
                name: "Me".to_string(),
                user_id: None,
                kind: ReplayPlayerKind::Human,
                bot: None,
            },
            ReplayPlayer {
                seat: 1,
                name: "Bot".to_string(),
                user_id: None,
                kind: ReplayPlayerKind::Bot,
                bot: Some("random".to_string()),
            },
        ];
        game.to_replay(players, "2026-11-16T00:00:00Z")
    }

    fn test_state(db: Db) -> AppState {
        AppState::new(
            ServerConfig {
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

    #[tokio::test]
    async fn solo_upload_valid_replay_200_and_tampered_400() {
        let db = Db::open_in_memory().expect("db");
        let user = db
            .run(|conn| users::create_user(conn, "solo_user", "Solo", "h"))
            .await
            .expect("user");
        let state = test_state(db.clone());
        let replay = played_replay();

        let resp = submit_solo_match(
            AuthUser(user.clone()),
            State(state.clone()),
            Json(replay.clone()),
        )
        .await
        .expect("valid upload");
        assert!(resp.0.match_id > 0);

        // solo 不計 Elo：ratings games=1 但 elo 維持 1200
        let stats = db
            .run(move |conn| ratings::get_user_stats(conn, user.id))
            .await
            .expect("stats");
        assert_eq!(stats.games, 1);
        assert_eq!(stats.by_mode["classic"].elo, 1200);

        // 竄改一步動作應被重播驗證拒絕
        let mut tampered = replay.clone();
        if let Some(step) = tampered.steps.iter_mut().find(|s| s.seat == Some(0)) {
            step.action = uno_core::ReplayAction::Action(uno_core::Action::Draw);
            // Draw 在多數情境可能仍合法，改為直接破壞：把第一步換成不存在的出牌
            step.action = uno_core::ReplayAction::Action(uno_core::Action::Play { card: 9999 });
        }
        let err = submit_solo_match(AuthUser(user), State(state), Json(tampered)).await;
        assert!(
            matches!(err, Err(ApiError::BadRequest(_))),
            "tampered replay must be 400"
        );
    }
}
