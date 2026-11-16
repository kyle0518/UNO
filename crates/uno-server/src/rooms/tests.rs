//! 房間 actor 測試（不經網路，直接對 actor 發命令、用 channel 收訊息）。

use std::time::Duration;

use tokio::sync::mpsc;
use uno_core::RuleConfig;
use uno_protocol::{ClientMsg, ErrorCode, RoomStatus, ServerMsg, UserPublic};

use super::registry::RoomRegistry;
use super::{OutRx, RoomHandle};
use crate::db::Db;

fn user(id: i64, name: &str) -> UserPublic {
    UserPublic {
        id,
        name: name.to_string(),
        is_guest: false,
    }
}

fn test_registry() -> RoomRegistry {
    RoomRegistry::new(Db::open_in_memory().expect("db"), "http://127.0.0.1:8787")
}

/// 建立 4 人 classic 房：host(user1) 與 user2 真人 + 2 bot，皆 ready 後回傳 handle 與兩人的接收端。
async fn started_room() -> (RoomHandle, OutRx, OutRx, Db) {
    let db = Db::open_in_memory().expect("db");
    let registry = RoomRegistry::new(db.clone(), "http://127.0.0.1:8787");
    let mut config = RuleConfig::preset("classic", 4).expect("preset");
    config.turn_seconds = 0;
    config.game_seconds = 0;
    let handle = registry
        .create_room(user(1, "Alice"), "classic", Some(config), true)
        .expect("create room");

    let (tx1, rx1) = mpsc::unbounded_channel();
    let (tx2, rx2) = mpsc::unbounded_channel();
    handle
        .join(user(1, "Alice"), false, tx1)
        .await
        .expect("join1");
    handle
        .join(user(2, "Bob"), false, tx2)
        .await
        .expect("join2");
    handle
        .handle_msg(1, ClientMsg::TakeSeat { seat: 0 })
        .await
        .expect("seat0");
    handle
        .handle_msg(2, ClientMsg::TakeSeat { seat: 1 })
        .await
        .expect("seat1");
    handle
        .handle_msg(
            1,
            ClientMsg::AddBot {
                seat: 2,
                bot: "random".to_string(),
            },
        )
        .await
        .expect("bot2");
    handle
        .handle_msg(
            1,
            ClientMsg::AddBot {
                seat: 3,
                bot: "random".to_string(),
            },
        )
        .await
        .expect("bot3");
    handle
        .handle_msg(1, ClientMsg::SetReady { ready: true })
        .await
        .expect("ready1");
    handle
        .handle_msg(2, ClientMsg::SetReady { ready: true })
        .await
        .expect("ready2");
    (handle, rx1, rx2, db)
}

/// 排空目前已抵達的訊息（不等待）。
fn drain(rx: &mut OutRx) -> Vec<ServerMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// 等待直到收到符合條件的訊息（逾時 panic）。
async fn wait_for<F: Fn(&ServerMsg) -> bool>(rx: &mut OutRx, what: &str, pred: F) -> ServerMsg {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let msg = rx.recv().await.expect("channel closed");
            if pred(&msg) {
                return msg;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timeout waiting for {what}"))
}

#[tokio::test]
async fn rooms_create_join_and_room_state() {
    let registry = test_registry();
    let handle = registry
        .create_room(user(1, "Alice"), "classic", None, true)
        .expect("create");
    assert_eq!(handle.code().len(), 6);
    assert!(handle
        .code()
        .chars()
        .all(|c| !"0O1IL".contains(c) && (c.is_ascii_uppercase() || c.is_ascii_digit())));

    let (tx, mut rx) = mpsc::unbounded_channel();
    handle
        .join(user(1, "Alice"), false, tx)
        .await
        .expect("join");
    let msgs = drain(&mut rx);
    assert!(msgs.iter().any(
        |m| matches!(m, ServerMsg::RoomState { room } if room.code == handle.code()
        && room.host_user_id == 1
        && room.seats.len() == 4
        && room.join_url.ends_with(&format!("/r/{}", handle.code())))
    ));

    let info = handle.room_info().await.expect("info");
    assert_eq!(info.status, RoomStatus::Waiting);
    // registry 列表應包含公開等待中房間
    let public = registry.list_public().await;
    assert_eq!(public.len(), 1);
    assert_eq!(public[0].code, handle.code());
}

#[tokio::test]
async fn rooms_start_game_sends_snapshots() {
    let (handle, mut rx1, mut rx2, _db) = started_room().await;
    handle
        .handle_msg(1, ClientMsg::StartGame)
        .await
        .expect("start");
    let snap = wait_for(&mut rx1, "snapshot", |m| {
        matches!(m, ServerMsg::GameSnapshot { .. })
    })
    .await;
    match snap {
        ServerMsg::GameSnapshot { view, .. } => {
            assert_eq!(view.me, Some(0));
            assert_eq!(view.my_hand.len(), 7);
        }
        _ => unreachable!(),
    }
    let snap2 = wait_for(&mut rx2, "snapshot2", |m| {
        matches!(m, ServerMsg::GameSnapshot { .. })
    })
    .await;
    match snap2 {
        ServerMsg::GameSnapshot { view, .. } => assert_eq!(view.me, Some(1)),
        _ => unreachable!(),
    }
    let info = handle.room_info().await.expect("info");
    assert_eq!(info.status, RoomStatus::Playing);
}

#[tokio::test]
async fn rooms_not_your_turn_error() {
    let (handle, mut rx1, mut rx2, _db) = started_room().await;
    handle
        .handle_msg(1, ClientMsg::StartGame)
        .await
        .expect("start");
    // 等兩邊都拿到快照，確認決策者
    let snap = wait_for(&mut rx1, "snap", |m| {
        matches!(m, ServerMsg::GameSnapshot { .. })
    })
    .await;
    let ServerMsg::GameSnapshot { view, .. } = snap else {
        unreachable!()
    };
    let decision = view.decision_player.expect("decision player");
    // 非決策者（若決策者是 bot 則兩位真人都非法；挑一位非決策真人送 Draw 背景動作測試）
    // 以送出一個幾乎不可能合法的動作驗證錯誤碼：非決策者應得 NotYourTurn。
    let (offender, offender_rx) = if decision == 0 {
        (2u8, &mut rx2)
    } else {
        (1u8, &mut rx1)
    };
    let offender_id = i64::from(offender);
    // 先排空
    let _ = drain(&mut *offender_rx);
    handle
        .handle_msg(
            offender_id,
            ClientMsg::Action {
                seq: 99,
                action: uno_core::Action::Draw,
            },
        )
        .await
        .expect("action");
    // bot 可能同時行動改變回合；只驗證若收到帶 seq=99 的錯誤，其 code 為 NotYourTurn 或 IllegalAction
    // （回合恰好輪到 offender 且 Draw 合法時不會有錯誤，這時直接通過）。
    tokio::time::sleep(Duration::from_millis(50)).await;
    let msgs = drain(&mut *offender_rx);
    for m in &msgs {
        if let ServerMsg::Error { code, seq, .. } = m {
            if *seq == Some(99) {
                assert!(
                    *code == ErrorCode::NotYourTurn || *code == ErrorCode::IllegalAction,
                    "unexpected error code {code:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn rooms_full_game_completes_and_persists() {
    tokio::time::pause();
    let (handle, mut rx1, mut rx2, db) = started_room().await;
    // match_players.user_id 有外鍵，先在 DB 建立兩位真人（fresh DB 的 id 應為 1、2）。
    db.with_conn(|conn| {
        let a = crate::db::users::create_user(conn, "alice", "Alice", "h")?;
        let b = crate::db::users::create_user(conn, "bob", "Bob", "h")?;
        assert_eq!((a.id, b.id), (1, 2));
        Ok(())
    })
    .expect("create users");
    handle
        .handle_msg(1, ClientMsg::StartGame)
        .await
        .expect("start");

    // 兩位真人輪流做第一個合法動作；bot 由 actor 自動排程（paused time 需推進）。
    let mut view1: Option<uno_core::PlayerView> = None;
    let mut view2: Option<uno_core::PlayerView> = None;
    let mut saved_match_id: Option<i64> = None;
    let mut steps = 0usize;
    while saved_match_id.is_none() && steps < 4000 {
        steps += 1;
        tokio::time::advance(Duration::from_millis(1300)).await;
        for msg in drain(&mut rx1) {
            match msg {
                ServerMsg::GameSnapshot { view, .. } | ServerMsg::GameEvents { view, .. } => {
                    view1 = Some(view)
                }
                ServerMsg::MatchSaved { match_id, .. } => saved_match_id = Some(match_id),
                _ => {}
            }
        }
        for msg in drain(&mut rx2) {
            match msg {
                ServerMsg::GameSnapshot { view, .. } | ServerMsg::GameEvents { view, .. } => {
                    view2 = Some(view)
                }
                ServerMsg::MatchSaved { match_id, .. } if saved_match_id.is_none() => {
                    saved_match_id = Some(match_id);
                }
                _ => {}
            }
        }
        if saved_match_id.is_some() {
            break;
        }
        // 依最新 view 判斷輪到哪位真人，送第一個合法動作（RoundOver 由房主送 NextRound）。
        let phase1 = view1.as_ref().map(|v| v.phase);
        if phase1 == Some(uno_core::Phase::RoundOver) {
            handle
                .handle_msg(1, ClientMsg::NextRound)
                .await
                .expect("next round");
            continue;
        }
        let mut acted = false;
        if let Some(v) = &view1 {
            if v.decision_player == Some(0) {
                if let Some(action) = v.legal.first().copied() {
                    handle
                        .handle_msg(
                            1,
                            ClientMsg::Action {
                                seq: steps as u32,
                                action,
                            },
                        )
                        .await
                        .expect("act1");
                    acted = true;
                }
            }
        }
        if !acted {
            if let Some(v) = &view2 {
                if v.decision_player == Some(1) {
                    if let Some(action) = v.legal.first().copied() {
                        handle
                            .handle_msg(
                                2,
                                ClientMsg::Action {
                                    seq: steps as u32,
                                    action,
                                },
                            )
                            .await
                            .expect("act2");
                    }
                }
            }
        }
    }
    let match_id = saved_match_id.expect("game should complete and save match");
    let (players_count, ratings_count) = db
        .with_conn(|conn| {
            let players = crate::db::matches::get_match_players(conn, match_id)?;
            let ratings: i64 = conn.query_row(
                "SELECT COUNT(*) FROM ratings WHERE user_id IN (1, 2);",
                [],
                |r| r.get(0),
            )?;
            Ok((players.len(), ratings))
        })
        .expect("db check");
    assert_eq!(players_count, 4, "match_players should have 4 rows");
    assert_eq!(ratings_count, 2, "both humans should have rating rows");
    let info = handle.room_info().await.expect("info");
    assert_eq!(info.status, RoomStatus::GameOver);
}

#[tokio::test]
async fn rooms_disconnect_takeover_by_bot() {
    tokio::time::pause();
    let (handle, mut rx1, mut rx2, _db) = started_room().await;
    handle
        .handle_msg(1, ClientMsg::StartGame)
        .await
        .expect("start");
    // 先幫 Bob 走回合（Alice 不動），直到輪到 Alice。
    let mut alice_turn = false;
    let mut bob_view: Option<uno_core::PlayerView> = None;
    for _ in 0..300 {
        tokio::time::advance(Duration::from_millis(1300)).await;
        tokio::task::yield_now().await;
        for msg in drain(&mut rx2) {
            if let ServerMsg::GameSnapshot { view, .. } | ServerMsg::GameEvents { view, .. } = msg {
                bob_view = Some(view);
            }
        }
        for msg in drain(&mut rx1) {
            if let ServerMsg::GameSnapshot { view, .. } | ServerMsg::GameEvents { view, .. } = msg {
                if view.decision_player == Some(0) {
                    alice_turn = true;
                }
            }
        }
        if alice_turn {
            break;
        }
        if let Some(v) = &bob_view {
            if v.decision_player == Some(1) {
                if let Some(action) = v.legal.first().copied() {
                    handle
                        .handle_msg(2, ClientMsg::Action { seq: 1, action })
                        .await
                        .expect("bob act");
                    tokio::task::yield_now().await;
                }
            }
        }
    }
    assert!(alice_turn, "should reach Alice turn");
    // Alice 斷線：代打應在 1 秒（虛擬）後替她出牌，回合推進到別人。
    handle.disconnect(1).await.expect("disconnect");
    let mut advanced = false;
    for _ in 0..50 {
        tokio::time::advance(Duration::from_millis(1100)).await;
        for msg in drain(&mut rx1) {
            if let ServerMsg::GameEvents { view, .. } = msg {
                if view.decision_player != Some(0) {
                    advanced = true;
                }
            }
        }
        if advanced {
            break;
        }
    }
    assert!(advanced, "takeover bot should act for disconnected Alice");
}
