use super::*;
use uno_core::{GameState, RuleConfig};

fn sample_config() -> RuleConfig {
    RuleConfig::preset("classic", 4).expect("classic preset must succeed")
}

fn sample_user() -> UserPublic {
    UserPublic {
        id: 42,
        name: "Alice".to_string(),
        is_guest: false,
    }
}

fn sample_view() -> PlayerView {
    let game = GameState::new(sample_config(), 42).expect("game must build");
    game.view(0)
}

fn sample_timer() -> TurnTimer {
    TurnTimer {
        seat: 0,
        deadline_ms: 1_700_000_000_000,
        total_ms: 15_000,
    }
}

fn sample_room() -> RoomInfo {
    RoomInfo {
        code: "ABC234".to_string(),
        host_user_id: 42,
        public: true,
        config: sample_config(),
        status: RoomStatus::Waiting,
        seats: vec![
            SeatInfo {
                seat: 0,
                occupant: Occupant::Human {
                    user: sample_user(),
                },
                ready: true,
                connected: true,
            },
            SeatInfo {
                seat: 1,
                occupant: Occupant::Bot {
                    kind: "heuristic".to_string(),
                },
                ready: true,
                connected: true,
            },
            SeatInfo {
                seat: 2,
                occupant: Occupant::Empty,
                ready: false,
                connected: false,
            },
        ],
        spectators: vec![UserPublic {
            id: 7,
            name: "Watcher".to_string(),
            is_guest: true,
        }],
        join_url: "http://192.168.1.10:8787/r/ABC234".to_string(),
    }
}

fn assert_client_round_trip(msg: ClientMsg, expected_type: &str) {
    let json = serde_json::to_string(&msg).expect("client msg must serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("json must parse");
    assert_eq!(
        value["type"], expected_type,
        "unexpected tag for {expected_type}: {json}"
    );
    let back: ClientMsg = serde_json::from_str(&json).expect("client msg must deserialize");
    assert_eq!(back, msg, "round-trip mismatch for {expected_type}");
}

fn assert_server_round_trip(msg: ServerMsg, expected_type: &str) {
    let json = serde_json::to_string(&msg).expect("server msg must serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("json must parse");
    assert_eq!(
        value["type"], expected_type,
        "unexpected tag for {expected_type}: {json}"
    );
    let back: ServerMsg = serde_json::from_str(&json).expect("server msg must deserialize");
    assert_eq!(back, msg, "round-trip mismatch for {expected_type}");
}

#[test]
fn client_msg_round_trip_all_variants() {
    assert_client_round_trip(
        ClientMsg::Hello {
            token: "tok123".to_string(),
            protocol: PROTOCOL_VERSION,
        },
        "hello",
    );
    assert_client_round_trip(
        ClientMsg::CreateRoom {
            mode_id: "classic".to_string(),
            config: None,
            public: true,
        },
        "create_room",
    );
    assert_client_round_trip(
        ClientMsg::CreateRoom {
            mode_id: "custom".to_string(),
            config: Some(sample_config()),
            public: false,
        },
        "create_room",
    );
    assert_client_round_trip(
        ClientMsg::JoinRoom {
            code: "ABC234".to_string(),
            as_spectator: false,
        },
        "join_room",
    );
    assert_client_round_trip(
        ClientMsg::JoinRoom {
            code: "ABC234".to_string(),
            as_spectator: true,
        },
        "join_room",
    );
    assert_client_round_trip(ClientMsg::LeaveRoom, "leave_room");
    assert_client_round_trip(ClientMsg::TakeSeat { seat: 2 }, "take_seat");
    assert_client_round_trip(
        ClientMsg::AddBot {
            seat: 1,
            bot: "heuristic".to_string(),
        },
        "add_bot",
    );
    assert_client_round_trip(ClientMsg::RemoveSeat { seat: 1 }, "remove_seat");
    assert_client_round_trip(
        ClientMsg::UpdateConfig {
            config: sample_config(),
        },
        "update_config",
    );
    assert_client_round_trip(ClientMsg::SetReady { ready: true }, "set_ready");
    assert_client_round_trip(ClientMsg::StartGame, "start_game");
    assert_client_round_trip(
        ClientMsg::Action {
            seq: 17,
            action: Action::Draw,
        },
        "action",
    );
    assert_client_round_trip(
        ClientMsg::Action {
            seq: 18,
            action: Action::CatchUno { target: 3 },
        },
        "action",
    );
    assert_client_round_trip(ClientMsg::NextRound, "next_round");
    assert_client_round_trip(ClientMsg::Rematch, "rematch");
    assert_client_round_trip(
        ClientMsg::Chat {
            text: "hello".to_string(),
        },
        "chat",
    );
    assert_client_round_trip(
        ClientMsg::Emote {
            emote: "uno".to_string(),
        },
        "emote",
    );
    assert_client_round_trip(ClientMsg::Ping { t: 123_456 }, "ping");
}

#[test]
fn server_msg_round_trip_all_variants() {
    assert_server_round_trip(
        ServerMsg::Welcome {
            user: sample_user(),
            server_version: "0.1.0".to_string(),
        },
        "welcome",
    );
    assert_server_round_trip(
        ServerMsg::Error {
            code: ErrorCode::NotYourTurn,
            message: "not your turn".to_string(),
            seq: Some(17),
        },
        "error",
    );
    assert_server_round_trip(
        ServerMsg::Error {
            code: ErrorCode::Internal,
            message: "internal".to_string(),
            seq: None,
        },
        "error",
    );
    assert_server_round_trip(
        ServerMsg::RoomState {
            room: sample_room(),
        },
        "room_state",
    );
    assert_server_round_trip(
        ServerMsg::GameSnapshot {
            view: sample_view(),
            timer: Some(sample_timer()),
        },
        "game_snapshot",
    );
    assert_server_round_trip(
        ServerMsg::GameSnapshot {
            view: sample_view(),
            timer: None,
        },
        "game_snapshot",
    );
    let game = GameState::new(sample_config(), 7).expect("game must build");
    assert_server_round_trip(
        ServerMsg::GameEvents {
            events: game.initial_events().to_vec(),
            view: sample_view(),
            timer: None,
        },
        "game_events",
    );
    assert_server_round_trip(
        ServerMsg::ChatMsg {
            from: "Alice".to_string(),
            seat: Some(0),
            text: "hi".to_string(),
            t: 1_700_000_000_000,
        },
        "chat_msg",
    );
    assert_server_round_trip(
        ServerMsg::ChatMsg {
            from: "Watcher".to_string(),
            seat: None,
            text: "watching".to_string(),
            t: 1_700_000_000_001,
        },
        "chat_msg",
    );
    assert_server_round_trip(
        ServerMsg::EmoteMsg {
            seat: 1,
            emote: "laugh".to_string(),
        },
        "emote_msg",
    );
    assert_server_round_trip(
        ServerMsg::MatchSaved {
            match_id: 99,
            elo_changes: vec![EloChange {
                user_id: 42,
                before: 1200,
                after: 1216,
            }],
        },
        "match_saved",
    );
    assert_server_round_trip(ServerMsg::Pong { t: 123_456 }, "pong");
}

#[test]
fn numeric_ids_serialize_as_json_numbers() {
    // JS 端以 number 接收（gen-types 以 #[ts(type = "number")] 對齊），
    // 序列化必須是 JSON number 而非字串。
    let json = serde_json::to_string(&ServerMsg::Pong { t: 5 }).expect("serialize");
    assert!(json.contains("\"t\":5"), "unexpected pong json: {json}");
    let json = serde_json::to_string(&sample_user()).expect("serialize");
    assert!(json.contains("\"id\":42"), "unexpected user json: {json}");
}

#[test]
fn error_code_round_trip_all_variants() {
    for code in [
        ErrorCode::BadRequest,
        ErrorCode::Unauthorized,
        ErrorCode::RoomNotFound,
        ErrorCode::RoomFull,
        ErrorCode::NotHost,
        ErrorCode::NotYourTurn,
        ErrorCode::IllegalAction,
        ErrorCode::GameInProgress,
        ErrorCode::RateLimited,
        ErrorCode::Internal,
    ] {
        let json = serde_json::to_string(&code).expect("code must serialize");
        let back: ErrorCode = serde_json::from_str(&json).expect("code must deserialize");
        assert_eq!(back, code);
    }
    let json = serde_json::to_string(&ErrorCode::NotYourTurn).expect("serialize");
    assert_eq!(json, "\"not_your_turn\"");
}

#[test]
fn room_status_and_occupant_round_trip() {
    for status in [
        RoomStatus::Waiting,
        RoomStatus::Playing,
        RoomStatus::RoundOver,
        RoomStatus::GameOver,
    ] {
        let json = serde_json::to_string(&status).expect("status must serialize");
        let back: RoomStatus = serde_json::from_str(&json).expect("status must deserialize");
        assert_eq!(back, status);
    }
    assert_eq!(
        serde_json::to_string(&RoomStatus::RoundOver).expect("serialize"),
        "\"round_over\""
    );
    for occupant in [
        Occupant::Empty,
        Occupant::Human {
            user: sample_user(),
        },
        Occupant::Bot {
            kind: "hard".to_string(),
        },
    ] {
        let json = serde_json::to_string(&occupant).expect("occupant must serialize");
        let back: Occupant = serde_json::from_str(&json).expect("occupant must deserialize");
        assert_eq!(back, occupant);
    }
}
