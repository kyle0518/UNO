//! WebSocket 端點（INTERFACES §7、PLAN T7.3）：路徑 `/ws`。
//! 第一則訊息必須是 `Hello`（5 秒內、protocol = 1、token 有效），
//! 之後訊息轉發至房間 actor；每連線每秒最多 10 則訊息。

use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uno_protocol::{ClientMsg, ErrorCode, ServerMsg, UserPublic, PROTOCOL_VERSION};

use crate::db::users;
use crate::rooms::{OutRx, RoomHandle};
use crate::state::AppState;

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const RATE_LIMIT: usize = 10;
const RATE_WINDOW: Duration = Duration::from_secs(1);

/// `GET /ws` 升級處理。
pub async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

fn send(out: &mpsc::UnboundedSender<ServerMsg>, msg: ServerMsg) {
    let _ = out.send(msg);
}

fn send_error(out: &mpsc::UnboundedSender<ServerMsg>, code: ErrorCode, message: &str) {
    send(
        out,
        ServerMsg::Error {
            code,
            message: message.to_string(),
            seq: None,
        },
    );
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (out_tx, mut out_rx): (mpsc::UnboundedSender<ServerMsg>, OutRx) = mpsc::unbounded_channel();

    // 先等待 Hello（5 秒逾時）。
    let hello = tokio::time::timeout(HELLO_TIMEOUT, stream.next()).await;
    let user: UserPublic = match hello {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientMsg>(&text) {
            Ok(ClientMsg::Hello { token, protocol }) => {
                if protocol != PROTOCOL_VERSION {
                    let _ = sink
                        .send(Message::Text(
                            serde_json::to_string(&ServerMsg::Error {
                                code: ErrorCode::Unauthorized,
                                message: "unsupported protocol".to_string(),
                                seq: None,
                            })
                            .unwrap_or_default()
                            .into(),
                        ))
                        .await;
                    return;
                }
                let db = state.db().clone();
                let found = db
                    .run(move |conn| users::find_user_by_token(conn, &token))
                    .await;
                match found {
                    Ok(Some(u)) => UserPublic {
                        id: u.id,
                        name: u.display_name.clone(),
                        is_guest: u.is_guest,
                    },
                    _ => {
                        let _ = sink
                            .send(Message::Text(
                                serde_json::to_string(&ServerMsg::Error {
                                    code: ErrorCode::Unauthorized,
                                    message: "invalid token".to_string(),
                                    seq: None,
                                })
                                .unwrap_or_default()
                                .into(),
                            ))
                            .await;
                        return;
                    }
                }
            }
            _ => return,
        },
        _ => return,
    };

    send(
        &out_tx,
        ServerMsg::Welcome {
            user: user.clone(),
            server_version: state.version().to_string(),
        },
    );

    // 轉發任務：房間推送 -> WebSocket
    let forward = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let Ok(text) = serde_json::to_string(&msg) else {
                continue;
            };
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let mut current_room: Option<RoomHandle> = None;
    let mut window: Vec<Instant> = Vec::new();

    while let Some(item) = stream.next().await {
        let msg = match item {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Binary(_)) => continue,
        };
        // 速率限制：每秒 10 則
        let now = Instant::now();
        window.retain(|t| now.duration_since(*t) < RATE_WINDOW);
        if window.len() >= RATE_LIMIT {
            send_error(&out_tx, ErrorCode::RateLimited, "rate limited");
            continue;
        }
        window.push(now);

        let parsed: ClientMsg = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(_) => {
                send_error(&out_tx, ErrorCode::BadRequest, "invalid message");
                continue;
            }
        };

        match parsed {
            ClientMsg::Hello { .. } => {
                send_error(&out_tx, ErrorCode::BadRequest, "already greeted");
            }
            ClientMsg::CreateRoom {
                mode_id,
                config,
                public,
            } => {
                match state
                    .rooms()
                    .create_room(user.clone(), &mode_id, config, public)
                {
                    Ok(handle) => {
                        if let Some(old) = current_room.take() {
                            let _ = old.leave(user.id).await;
                        }
                        match handle.join(user.clone(), false, out_tx.clone()).await {
                            Ok(()) => current_room = Some(handle),
                            Err(e) => send_error(&out_tx, e, "join failed"),
                        }
                    }
                    Err(e) => send_error(&out_tx, e, "create room failed"),
                }
            }
            ClientMsg::JoinRoom { code, as_spectator } => match state.rooms().get(&code) {
                Some(handle) => {
                    if let Some(old) = current_room.take() {
                        if old.code() != handle.code() {
                            let _ = old.leave(user.id).await;
                        }
                    }
                    match handle
                        .join(user.clone(), as_spectator, out_tx.clone())
                        .await
                    {
                        Ok(()) => current_room = Some(handle),
                        Err(e) => send_error(&out_tx, e, "join failed"),
                    }
                }
                None => send_error(&out_tx, ErrorCode::RoomNotFound, "room not found"),
            },
            ClientMsg::LeaveRoom => {
                if let Some(room) = current_room.take() {
                    let _ = room.leave(user.id).await;
                }
            }
            ClientMsg::Ping { t } if current_room.is_none() => {
                send(&out_tx, ServerMsg::Pong { t });
            }
            other => match &current_room {
                Some(room) => {
                    let _ = room.handle_msg(user.id, other).await;
                }
                None => send_error(&out_tx, ErrorCode::BadRequest, "not in a room"),
            },
        }
    }

    // 連線結束：通知房間斷線（保留座位代打）。
    if let Some(room) = current_room.take() {
        let _ = room.disconnect(user.id).await;
    }
    drop(out_tx);
    let _ = forward.await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::db::Db;
    use std::path::PathBuf;
    use tokio_tungstenite::tungstenite::Message as TMessage;

    async fn spawn_server() -> (String, Db) {
        let db = Db::open_in_memory().expect("db");
        let cfg = ServerConfig {
            port: 0,
            host: "127.0.0.1".to_string(),
            data_dir: PathBuf::from("data"),
            web_dir: PathBuf::from("/tmp/uno-ws-test-web"),
            models_dir: PathBuf::from("/tmp/uno-ws-test-models"),
        };
        let state = AppState::new(cfg, 0, vec!["http://127.0.0.1".to_string()], db.clone());
        let app = crate::build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (format!("ws://{addr}/ws"), db)
    }

    fn make_user(db: &Db, name: &str) -> (i64, String) {
        let token = crate::auth::generate_token();
        let id = db
            .with_conn(|conn| {
                let u = users::create_guest(conn, name)?;
                users::create_session_30_days(conn, &token, u.id)?;
                Ok(u.id)
            })
            .expect("make user");
        (id, token)
    }

    async fn recv_json(
        ws: &mut (impl StreamExt<Item = Result<TMessage, tokio_tungstenite::tungstenite::Error>>
                  + Unpin),
    ) -> ServerMsg {
        let deadline = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _ = &mut deadline => panic!("timeout waiting server msg"),
                item = ws.next() => {
                    let msg = item.expect("stream end").expect("ws error");
                    if let TMessage::Text(t) = msg {
                        return serde_json::from_str(&t).expect("parse server msg");
                    }
                }
            }
        }
    }

    async fn recv_until<F: Fn(&ServerMsg) -> bool>(
        ws: &mut (impl StreamExt<Item = Result<TMessage, tokio_tungstenite::tungstenite::Error>>
                  + Unpin),
        pred: F,
    ) -> ServerMsg {
        loop {
            let m = recv_json(ws).await;
            if pred(&m) {
                return m;
            }
        }
    }

    #[tokio::test]
    async fn ws_two_clients_create_join_and_start() {
        let (url, db) = spawn_server().await;
        let (alice_id, alice_token) = make_user(&db, "Alice");
        let (bob_id, bob_token) = make_user(&db, "Bob");

        let (mut a, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("connect a");
        a.send(TMessage::Text(
            serde_json::to_string(&ClientMsg::Hello {
                token: alice_token,
                protocol: PROTOCOL_VERSION,
            })
            .expect("json")
            .into(),
        ))
        .await
        .expect("hello a");
        let welcome = recv_json(&mut a).await;
        assert!(matches!(welcome, ServerMsg::Welcome { ref user, .. } if user.id == alice_id));

        a.send(TMessage::Text(
            serde_json::to_string(&ClientMsg::CreateRoom {
                mode_id: "classic".to_string(),
                config: None,
                public: true,
            })
            .expect("json")
            .into(),
        ))
        .await
        .expect("create");
        let room_state = recv_until(&mut a, |m| matches!(m, ServerMsg::RoomState { .. })).await;
        let ServerMsg::RoomState { room } = room_state else {
            unreachable!()
        };
        let code = room.code.clone();
        assert_eq!(code.len(), 6);

        let (mut b, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("connect b");
        b.send(TMessage::Text(
            serde_json::to_string(&ClientMsg::Hello {
                token: bob_token,
                protocol: PROTOCOL_VERSION,
            })
            .expect("json")
            .into(),
        ))
        .await
        .expect("hello b");
        let welcome_b = recv_json(&mut b).await;
        assert!(matches!(welcome_b, ServerMsg::Welcome { ref user, .. } if user.id == bob_id));
        b.send(TMessage::Text(
            serde_json::to_string(&ClientMsg::JoinRoom {
                code: code.clone(),
                as_spectator: false,
            })
            .expect("json")
            .into(),
        ))
        .await
        .expect("join b");
        let joined = recv_until(&mut b, |m| matches!(m, ServerMsg::RoomState { .. })).await;
        assert!(matches!(joined, ServerMsg::RoomState { ref room } if room.code == code));

        // 兩人入座 + 補 2 bot + ready + 開始，雙方都應收到 GameSnapshot
        for (ws, seat) in [(&mut a, 0u8), (&mut b, 1u8)] {
            ws.send(TMessage::Text(
                serde_json::to_string(&ClientMsg::TakeSeat { seat })
                    .expect("json")
                    .into(),
            ))
            .await
            .expect("seat");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        for seat in [2u8, 3u8] {
            a.send(TMessage::Text(
                serde_json::to_string(&ClientMsg::AddBot {
                    seat,
                    bot: "random".to_string(),
                })
                .expect("json")
                .into(),
            ))
            .await
            .expect("add bot");
            tokio::time::sleep(Duration::from_millis(1100)).await; // 避開速率限制
        }
        for ws in [&mut a, &mut b] {
            ws.send(TMessage::Text(
                serde_json::to_string(&ClientMsg::SetReady { ready: true })
                    .expect("json")
                    .into(),
            ))
            .await
            .expect("ready");
            tokio::time::sleep(Duration::from_millis(1100)).await;
        }
        a.send(TMessage::Text(
            serde_json::to_string(&ClientMsg::StartGame)
                .expect("json")
                .into(),
        ))
        .await
        .expect("start");
        let snap_a = recv_until(&mut a, |m| matches!(m, ServerMsg::GameSnapshot { .. })).await;
        let snap_b = recv_until(&mut b, |m| matches!(m, ServerMsg::GameSnapshot { .. })).await;
        assert!(matches!(snap_a, ServerMsg::GameSnapshot { ref view, .. } if view.me == Some(0)));
        assert!(matches!(snap_b, ServerMsg::GameSnapshot { ref view, .. } if view.me == Some(1)));
    }

    #[tokio::test]
    async fn ws_rejects_bad_token() {
        let (url, _db) = spawn_server().await;
        let (mut a, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("connect");
        a.send(TMessage::Text(
            serde_json::to_string(&ClientMsg::Hello {
                token: "invalid-token".to_string(),
                protocol: PROTOCOL_VERSION,
            })
            .expect("json")
            .into(),
        ))
        .await
        .expect("hello");
        let err = recv_json(&mut a).await;
        assert!(matches!(
            err,
            ServerMsg::Error {
                code: ErrorCode::Unauthorized,
                ..
            }
        ));
    }
}
