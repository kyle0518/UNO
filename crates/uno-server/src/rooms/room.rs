//! 房間 actor：單一 tokio task 持有全部房間狀態，依序處理命令。

use std::collections::HashMap;
use std::time::Duration;

use rand::RngExt;
use tokio::sync::{mpsc, oneshot};
use uno_bots::Bot;
use uno_core::{
    filter_events, Action, Event, GameState, Phase, ReplayPlayer, ReplayPlayerKind, RuleConfig,
    Seat,
};
use uno_protocol::{
    ClientMsg, EloChange, ErrorCode, Occupant, RoomInfo, RoomStatus, SeatInfo, ServerMsg,
    TurnTimer, UserPublic,
};

use super::bots;
use super::timers::{now_epoch_ms, schedule_after};
use super::{OutTx, RoomCommand};
use crate::db::{matches, ratings, Db};

/// 建立房間所需的參數。
pub struct RoomParams {
    pub code: String,
    pub host_user_id: i64,
    pub public: bool,
    pub config: RuleConfig,
    pub join_url: String,
    pub db: Db,
}

/// 房間 handle：外部以此送命令，不直接接觸 actor 狀態。
#[derive(Clone)]
pub struct RoomHandle {
    code: String,
    tx: mpsc::Sender<RoomCommand>,
}

impl std::fmt::Debug for RoomHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomHandle")
            .field("code", &self.code)
            .finish_non_exhaustive()
    }
}

impl RoomHandle {
    pub fn spawn(params: RoomParams) -> Self {
        let (tx, rx) = mpsc::channel(256);
        let code = params.code.clone();
        let room = Room::new(params, tx.clone());
        tokio::spawn(room.run(rx));
        Self { code, tx }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn sender(&self) -> mpsc::Sender<RoomCommand> {
        self.tx.clone()
    }

    pub async fn send(&self, cmd: RoomCommand) -> Result<(), ErrorCode> {
        self.tx.send(cmd).await.map_err(|_| ErrorCode::Internal)
    }

    /// 加入房間；成功回傳 `Ok(())`，訊息由 `out` 通道推送。
    pub async fn join(
        &self,
        user: UserPublic,
        as_spectator: bool,
        out: OutTx,
    ) -> Result<(), ErrorCode> {
        let (reply, rx) = oneshot::channel();
        self.send(RoomCommand::Join {
            user,
            as_spectator,
            out,
            reply,
        })
        .await?;
        rx.await.map_err(|_| ErrorCode::Internal)?
    }

    pub async fn leave(&self, user_id: i64) -> Result<(), ErrorCode> {
        self.send(RoomCommand::Leave { user_id }).await
    }

    pub async fn disconnect(&self, user_id: i64) -> Result<(), ErrorCode> {
        self.send(RoomCommand::Disconnect { user_id }).await
    }

    pub async fn handle_msg(&self, user_id: i64, msg: ClientMsg) -> Result<(), ErrorCode> {
        self.send(RoomCommand::Msg { user_id, msg }).await
    }

    pub async fn room_info(&self) -> Option<RoomInfo> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(RoomCommand::QueryInfo { reply }).await.ok()?;
        rx.await.ok()
    }
}

#[derive(Clone, Debug)]
enum OccupantState {
    Empty,
    Human(UserPublic),
    Bot(String),
}

#[derive(Clone, Debug)]
struct SeatState {
    occupant: OccupantState,
    ready: bool,
    connected: bool,
}

impl SeatState {
    fn empty() -> Self {
        Self {
            occupant: OccupantState::Empty,
            ready: false,
            connected: false,
        }
    }
}

struct ClientConn {
    user: UserPublic,
    out: OutTx,
    spectator: bool,
}

struct Room {
    code: String,
    host_user_id: i64,
    public: bool,
    config: RuleConfig,
    status: RoomStatus,
    seats: Vec<SeatState>,
    clients: HashMap<i64, ClientConn>,
    game: Option<GameState>,
    bots: HashMap<Seat, Box<dyn Bot>>,
    game_seed: u64,
    epoch: u64,
    turn_timer: Option<TurnTimer>,
    match_saved: bool,
    disconnect_epochs: HashMap<i64, u64>,
    cmd_tx: mpsc::Sender<RoomCommand>,
    db: Db,
    join_url: String,
}

impl Room {
    fn new(params: RoomParams, cmd_tx: mpsc::Sender<RoomCommand>) -> Self {
        let n = usize::from(params.config.num_players);
        Self {
            code: params.code,
            host_user_id: params.host_user_id,
            public: params.public,
            config: params.config,
            status: RoomStatus::Waiting,
            seats: (0..n).map(|_| SeatState::empty()).collect(),
            clients: HashMap::new(),
            game: None,
            bots: HashMap::new(),
            game_seed: 0,
            epoch: 0,
            turn_timer: None,
            match_saved: false,
            disconnect_epochs: HashMap::new(),
            cmd_tx,
            db: params.db,
            join_url: params.join_url,
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<RoomCommand>) {
        while let Some(cmd) = rx.recv().await {
            self.handle(cmd).await;
        }
    }

    async fn handle(&mut self, cmd: RoomCommand) {
        match cmd {
            RoomCommand::Join {
                user,
                as_spectator,
                out,
                reply,
            } => {
                let res = self.join(user, as_spectator, out);
                let _ = reply.send(res);
            }
            RoomCommand::Leave { user_id } => self.leave(user_id),
            RoomCommand::Disconnect { user_id } => self.disconnect(user_id),
            RoomCommand::Msg { user_id, msg } => self.handle_msg(user_id, msg).await,
            RoomCommand::BotTurn { epoch, turn, seat } => self.on_bot_turn(epoch, turn, seat).await,
            RoomCommand::BotCatch {
                epoch,
                target,
                seat,
            } => self.on_bot_catch(epoch, target, seat).await,
            RoomCommand::TurnTimeout { epoch, turn, seat } => {
                self.on_turn_timeout(epoch, turn, seat).await
            }
            RoomCommand::GameTimeout { epoch } => self.on_game_timeout(epoch).await,
            RoomCommand::AutoNextRound { epoch, round } => {
                self.on_auto_next_round(epoch, round).await
            }
            RoomCommand::DisconnectExpired { user_id, epoch } => {
                self.on_disconnect_expired(user_id, epoch)
            }
            RoomCommand::QueryInfo { reply } => {
                let _ = reply.send(self.room_info());
            }
        }
    }

    // ---------- 連線管理 ----------

    fn join(&mut self, user: UserPublic, as_spectator: bool, out: OutTx) -> Result<(), ErrorCode> {
        let user_id = user.id;
        let seated = self.seat_of(user_id);
        // 對局中且無座位者只能觀戰。
        let spectator = as_spectator || (self.status == RoomStatus::Playing && seated.is_none());
        self.clients.insert(
            user_id,
            ClientConn {
                user: user.clone(),
                out,
                spectator,
            },
        );
        if let Some(seat) = seated {
            self.seats[usize::from(seat)].connected = true;
            self.disconnect_epochs.insert(
                user_id,
                self.disconnect_epochs.get(&user_id).copied().unwrap_or(0) + 1,
            );
        }
        self.send_room_state_to(user_id);
        if self.status == RoomStatus::Playing || self.status == RoomStatus::RoundOver {
            self.send_snapshot_to(user_id);
        }
        self.broadcast_room_state();
        Ok(())
    }

    fn leave(&mut self, user_id: i64) {
        self.clients.remove(&user_id);
        if let Some(seat) = self.seat_of(user_id) {
            let s = &mut self.seats[usize::from(seat)];
            s.occupant = OccupantState::Empty;
            s.ready = false;
            s.connected = false;
        }
        if user_id == self.host_user_id {
            self.transfer_host();
        }
        self.broadcast_room_state();
    }

    fn disconnect(&mut self, user_id: i64) {
        let Some(seat) = self.seat_of(user_id) else {
            self.clients.remove(&user_id);
            return;
        };
        self.seats[usize::from(seat)].connected = false;
        let epoch = self.disconnect_epochs.get(&user_id).copied().unwrap_or(0) + 1;
        self.disconnect_epochs.insert(user_id, epoch);
        schedule_after(
            self.cmd_tx.clone(),
            Duration::from_secs(60),
            RoomCommand::DisconnectExpired { user_id, epoch },
        );
        self.broadcast_room_state();
        // 若正輪到斷線者，啟動代打排程。
        if self.status == RoomStatus::Playing {
            if let Some(game) = &self.game {
                if game.decision_player() == Some(seat) {
                    self.schedule_takeover(seat);
                }
            }
        }
    }

    fn on_disconnect_expired(&mut self, user_id: i64, epoch: u64) {
        if self.disconnect_epochs.get(&user_id).copied() != Some(epoch) {
            return;
        }
        let Some(seat) = self.seat_of(user_id) else {
            return;
        };
        if self.seats[usize::from(seat)].connected {
            return;
        }
        // 逾時：座位永久轉為 heuristic bot。
        let s = &mut self.seats[usize::from(seat)];
        s.occupant = OccupantState::Bot("heuristic".to_string());
        s.ready = true;
        s.connected = true;
        self.clients.remove(&user_id);
        if self.status == RoomStatus::Playing && self.game.is_some() {
            if let Some(bot) = bots::create_seat_bot("heuristic", self.game_seed, seat) {
                self.bots.insert(seat, bot);
            }
            self.schedule_next();
        }
        self.broadcast_room_state();
    }

    fn transfer_host(&mut self) {
        for s in &self.seats {
            if let OccupantState::Human(u) = &s.occupant {
                self.host_user_id = u.id;
                return;
            }
        }
        for c in self.clients.values() {
            if !c.spectator {
                self.host_user_id = c.user.id;
                return;
            }
        }
    }

    // ---------- 客戶端訊息 ----------

    async fn handle_msg(&mut self, user_id: i64, msg: ClientMsg) {
        match msg {
            ClientMsg::Ping { t } => self.send_to(user_id, ServerMsg::Pong { t }),
            ClientMsg::Chat { text } => {
                if text.chars().count() > 200 {
                    self.send_error(user_id, ErrorCode::BadRequest, "chat too long", None);
                    return;
                }
                let (from, seat) = match self.clients.get(&user_id) {
                    Some(c) => (c.user.name.clone(), self.seat_of(user_id)),
                    None => return,
                };
                self.broadcast(ServerMsg::ChatMsg {
                    from,
                    seat,
                    text,
                    t: now_epoch_ms(),
                });
            }
            ClientMsg::Emote { emote } => {
                if let Some(seat) = self.seat_of(user_id) {
                    self.broadcast(ServerMsg::EmoteMsg { seat, emote });
                }
            }
            ClientMsg::TakeSeat { seat } => self.take_seat(user_id, seat),
            ClientMsg::AddBot { seat, bot } => self.add_bot(user_id, seat, &bot),
            ClientMsg::RemoveSeat { seat } => self.remove_seat(user_id, seat),
            ClientMsg::UpdateConfig { config } => self.update_config(user_id, config),
            ClientMsg::SetReady { ready } => {
                if let Some(seat) = self.seat_of(user_id) {
                    self.seats[usize::from(seat)].ready = ready;
                    self.broadcast_room_state();
                }
            }
            ClientMsg::StartGame => self.start_game(user_id).await,
            ClientMsg::Action { seq, action } => self.player_action(user_id, seq, action).await,
            ClientMsg::NextRound => self.next_round_cmd(user_id).await,
            ClientMsg::Rematch => self.rematch(user_id),
            // Hello/CreateRoom/JoinRoom/LeaveRoom 由 WebSocket 層先行處理後轉成命令。
            ClientMsg::LeaveRoom => self.leave(user_id),
            ClientMsg::Hello { .. } | ClientMsg::CreateRoom { .. } | ClientMsg::JoinRoom { .. } => {
                self.send_error(user_id, ErrorCode::BadRequest, "unexpected message", None)
            }
        }
    }

    fn take_seat(&mut self, user_id: i64, seat: Seat) {
        if self.status != RoomStatus::Waiting {
            self.send_error(user_id, ErrorCode::GameInProgress, "game in progress", None);
            return;
        }
        let idx = usize::from(seat);
        if idx >= self.seats.len() {
            self.send_error(user_id, ErrorCode::BadRequest, "seat out of range", None);
            return;
        }
        if self.seat_of(user_id).is_some() {
            self.send_error(user_id, ErrorCode::BadRequest, "already seated", None);
            return;
        }
        if !matches!(self.seats[idx].occupant, OccupantState::Empty) {
            self.send_error(user_id, ErrorCode::BadRequest, "seat taken", None);
            return;
        }
        let Some(user) = self.clients.get(&user_id).map(|c| c.user.clone()) else {
            return;
        };
        let s = &mut self.seats[idx];
        s.occupant = OccupantState::Human(user);
        s.ready = false;
        s.connected = true;
        if let Some(c) = self.clients.get_mut(&user_id) {
            c.spectator = false;
        }
        self.broadcast_room_state();
    }

    fn add_bot(&mut self, user_id: i64, seat: Seat, bot: &str) {
        if !self.require_host(user_id) {
            return;
        }
        if self.status != RoomStatus::Waiting {
            self.send_error(user_id, ErrorCode::GameInProgress, "game in progress", None);
            return;
        }
        let idx = usize::from(seat);
        if idx >= self.seats.len() {
            self.send_error(user_id, ErrorCode::BadRequest, "seat out of range", None);
            return;
        }
        if bots::validate_bot_kind(bot).is_err() {
            self.send_error(user_id, ErrorCode::BadRequest, "unknown bot kind", None);
            return;
        }
        if !matches!(self.seats[idx].occupant, OccupantState::Empty) {
            self.send_error(user_id, ErrorCode::BadRequest, "seat taken", None);
            return;
        }
        let s = &mut self.seats[idx];
        s.occupant = OccupantState::Bot(bot.to_string());
        s.ready = true;
        s.connected = true;
        self.broadcast_room_state();
    }

    fn remove_seat(&mut self, user_id: i64, seat: Seat) {
        if !self.require_host(user_id) {
            return;
        }
        if self.status != RoomStatus::Waiting {
            self.send_error(user_id, ErrorCode::GameInProgress, "game in progress", None);
            return;
        }
        let idx = usize::from(seat);
        if idx >= self.seats.len() {
            self.send_error(user_id, ErrorCode::BadRequest, "seat out of range", None);
            return;
        }
        if let OccupantState::Human(u) = &self.seats[idx].occupant {
            if u.id == user_id {
                self.send_error(user_id, ErrorCode::BadRequest, "cannot remove self", None);
                return;
            }
            let removed_id = u.id;
            if let Some(c) = self.clients.get_mut(&removed_id) {
                c.spectator = true;
            }
        }
        let s = &mut self.seats[idx];
        s.occupant = OccupantState::Empty;
        s.ready = false;
        s.connected = false;
        self.broadcast_room_state();
    }

    fn update_config(&mut self, user_id: i64, config: RuleConfig) {
        if !self.require_host(user_id) {
            return;
        }
        if self.status != RoomStatus::Waiting {
            self.send_error(user_id, ErrorCode::GameInProgress, "game in progress", None);
            return;
        }
        let mut cfg = config;
        if cfg.validate().is_err() {
            self.send_error(user_id, ErrorCode::BadRequest, "invalid config", None);
            return;
        }
        let n = usize::from(cfg.num_players);
        self.seats.resize_with(n, SeatState::empty);
        // 被裁掉座位上的真人轉為觀戰。
        let seated: Vec<i64> = self
            .seats
            .iter()
            .filter_map(|s| match &s.occupant {
                OccupantState::Human(u) => Some(u.id),
                _ => None,
            })
            .collect();
        for c in self.clients.values_mut() {
            if !c.spectator && !seated.contains(&c.user.id) {
                c.spectator = true;
            }
        }
        self.config = cfg;
        self.broadcast_room_state();
    }

    fn rematch(&mut self, user_id: i64) {
        if !self.require_host(user_id) {
            return;
        }
        if self.status != RoomStatus::GameOver {
            self.send_error(user_id, ErrorCode::BadRequest, "game not over", None);
            return;
        }
        self.game = None;
        self.bots.clear();
        self.status = RoomStatus::Waiting;
        self.turn_timer = None;
        self.match_saved = false;
        for s in &mut self.seats {
            s.ready = matches!(s.occupant, OccupantState::Bot(_));
        }
        self.broadcast_room_state();
    }

    // ---------- 對局流程 ----------

    async fn start_game(&mut self, user_id: i64) {
        if !self.require_host(user_id) {
            return;
        }
        if self.status != RoomStatus::Waiting {
            self.send_error(user_id, ErrorCode::GameInProgress, "game in progress", None);
            return;
        }
        if self
            .seats
            .iter()
            .any(|s| matches!(s.occupant, OccupantState::Empty))
        {
            self.send_error(
                user_id,
                ErrorCode::BadRequest,
                "all seats must be filled",
                None,
            );
            return;
        }
        let humans_ready = self.seats.iter().all(|s| match &s.occupant {
            OccupantState::Human(_) => s.ready,
            _ => true,
        });
        if !humans_ready {
            self.send_error(
                user_id,
                ErrorCode::BadRequest,
                "not all human players are ready",
                None,
            );
            return;
        }
        self.game_seed = rand::rng().random_range(1..u64::MAX);
        let game = match GameState::new(self.config.clone(), self.game_seed) {
            Ok(g) => g,
            Err(_) => {
                self.send_error(user_id, ErrorCode::BadRequest, "invalid config", None);
                return;
            }
        };
        self.bots.clear();
        for (idx, s) in self.seats.iter().enumerate() {
            if let OccupantState::Bot(kind) = &s.occupant {
                let seat = idx as Seat;
                match bots::create_seat_bot(kind, self.game_seed, seat) {
                    Some(bot) => {
                        self.bots.insert(seat, bot);
                    }
                    None => {
                        self.send_error(user_id, ErrorCode::Internal, "bot unavailable", None);
                        return;
                    }
                }
            }
        }
        self.epoch += 1;
        self.game = Some(game);
        self.status = RoomStatus::Playing;
        self.match_saved = false;
        self.broadcast_room_state();
        self.broadcast_snapshot();
        if self.config.game_seconds > 0 {
            schedule_after(
                self.cmd_tx.clone(),
                Duration::from_secs(u64::from(self.config.game_seconds)),
                RoomCommand::GameTimeout { epoch: self.epoch },
            );
        }
        self.schedule_next();
    }

    async fn player_action(&mut self, user_id: i64, seq: u32, action: Action) {
        if self.status != RoomStatus::Playing {
            self.send_error(user_id, ErrorCode::GameInProgress, "not playing", Some(seq));
            return;
        }
        let Some(seat) = self.seat_of(user_id) else {
            self.send_error(user_id, ErrorCode::BadRequest, "not seated", Some(seq));
            return;
        };
        if !self.seats[usize::from(seat)].connected {
            return;
        }
        let legal = match &self.game {
            Some(g) => g.legal_actions(seat),
            None => return,
        };
        if !legal.contains(&action) {
            let decision = self.game.as_ref().and_then(|g| g.decision_player());
            let code = if decision != Some(seat) {
                ErrorCode::NotYourTurn
            } else {
                ErrorCode::IllegalAction
            };
            self.send_error(user_id, code, "illegal action", Some(seq));
            return;
        }
        self.apply_and_broadcast(seat, action).await;
    }

    async fn next_round_cmd(&mut self, user_id: i64) {
        if !self.require_host(user_id) {
            return;
        }
        self.do_next_round().await;
    }

    async fn do_next_round(&mut self) {
        if self.status != RoomStatus::RoundOver {
            return;
        }
        let events = match self.game.as_mut().and_then(|g| g.next_round().ok()) {
            Some(e) => e,
            None => return,
        };
        self.status = RoomStatus::Playing;
        self.after_step(events).await;
    }

    async fn apply_and_broadcast(&mut self, seat: Seat, action: Action) {
        let events = match self.game.as_mut().and_then(|g| g.apply(seat, action).ok()) {
            Some(e) => e,
            None => return,
        };
        self.after_step(events).await;
    }

    async fn after_step(&mut self, events: Vec<Event>) {
        let phase = self.game.as_ref().map(|g| g.phase());
        match phase {
            Some(Phase::RoundOver) => {
                self.status = RoomStatus::RoundOver;
                self.turn_timer = None;
                self.broadcast_events(&events);
                self.broadcast_room_state();
                let round = self.game.as_ref().map(|g| g.round()).unwrap_or(0);
                schedule_after(
                    self.cmd_tx.clone(),
                    Duration::from_secs(8),
                    RoomCommand::AutoNextRound {
                        epoch: self.epoch,
                        round,
                    },
                );
            }
            Some(Phase::GameOver) => {
                self.status = RoomStatus::GameOver;
                self.turn_timer = None;
                self.broadcast_events(&events);
                self.finish_match().await;
                self.broadcast_room_state();
            }
            _ => {
                self.broadcast_events(&events);
                self.schedule_next();
            }
        }
    }

    async fn finish_match(&mut self) {
        if self.match_saved {
            return;
        }
        self.match_saved = true;
        let Some(game) = &self.game else { return };
        let rankings = game.rankings().map(|r| r.to_vec()).unwrap_or_default();
        let scores = game.scores().to_vec();
        let rank_of = |seat: Seat| -> u32 {
            rankings
                .iter()
                .position(|&s| s == seat)
                .map(|p| p as u32)
                .unwrap_or(seat as u32)
        };
        let mut players = Vec::new();
        let mut outcomes = Vec::new();
        let mut replay_players = Vec::new();
        for (idx, s) in self.seats.iter().enumerate() {
            let seat = idx as Seat;
            let rank = rank_of(seat);
            let score = scores.get(idx).copied().unwrap_or(0);
            let (user_id, bot_kind, name, kind) = match &s.occupant {
                OccupantState::Human(u) => {
                    (Some(u.id), None, u.name.clone(), ReplayPlayerKind::Human)
                }
                OccupantState::Bot(k) => (
                    None,
                    Some(k.clone()),
                    format!("Bot {seat}"),
                    ReplayPlayerKind::Bot,
                ),
                OccupantState::Empty => continue,
            };
            players.push(matches::NewMatchPlayer {
                seat: u32::from(seat),
                user_id,
                bot_kind: bot_kind.clone(),
                rank,
                score,
            });
            outcomes.push(ratings::PlayerMatchOutcome {
                user_id,
                seat: u32::from(seat),
                rank,
                team: game.team_of_seat(seat),
            });
            replay_players.push(ReplayPlayer {
                seat,
                name,
                user_id,
                kind,
                bot: bot_kind,
            });
        }
        let replay = game.to_replay(replay_players, now_epoch_ms().to_string());
        let replay_json = match serde_json::to_string(&replay) {
            Ok(j) => j,
            Err(_) => return,
        };
        let new_match = matches::NewMatch {
            mode_id: self.config.mode_id.clone(),
            num_players: self.config.num_players as u32,
            is_solo: false,
            seed: self.game_seed,
            replay_json,
        };
        let is_team = self.config.teams.is_some();
        let mode_id = self.config.mode_id.clone();
        let result = self
            .db
            .run(move |conn| {
                let match_id = matches::insert_match(conn, &new_match, &players)?;
                let deltas = ratings::apply_match_ratings(
                    conn, match_id, &mode_id, false, is_team, &outcomes,
                )?;
                Ok((match_id, deltas))
            })
            .await;
        if let Err(e) = &result {
            tracing::error!(room = %self.code, error = %e, "failed to persist match result");
        }
        if let Ok((match_id, deltas)) = result {
            let elo_changes = deltas
                .iter()
                .map(|d| EloChange {
                    user_id: d.user_id,
                    before: d.old_elo,
                    after: d.new_elo,
                })
                .collect();
            self.broadcast(ServerMsg::MatchSaved {
                match_id,
                elo_changes,
            });
        }
    }

    // ---------- 排程 ----------

    fn schedule_next(&mut self) {
        let Some(game) = &self.game else { return };
        if self.status != RoomStatus::Playing {
            return;
        }
        let Some(decision) = game.decision_player() else {
            self.turn_timer = None;
            return;
        };
        let turn = game.view(decision).turn;
        // 回合計時
        if self.config.turn_seconds > 0 {
            let total_ms = u32::from(self.config.turn_seconds) * 1000;
            self.turn_timer = Some(TurnTimer {
                seat: decision,
                deadline_ms: now_epoch_ms() + u64::from(total_ms),
                total_ms,
            });
            schedule_after(
                self.cmd_tx.clone(),
                Duration::from_millis(u64::from(total_ms)),
                RoomCommand::TurnTimeout {
                    epoch: self.epoch,
                    turn,
                    seat: decision,
                },
            );
        } else {
            self.turn_timer = None;
        }
        // Bot / 代打
        match &self.seats[usize::from(decision)].occupant {
            OccupantState::Bot(_) => schedule_after(
                self.cmd_tx.clone(),
                Duration::from_millis(bots::bot_think_delay_ms()),
                RoomCommand::BotTurn {
                    epoch: self.epoch,
                    turn,
                    seat: decision,
                },
            ),
            OccupantState::Human(_) if !self.seats[usize::from(decision)].connected => {
                self.schedule_takeover(decision)
            }
            _ => {}
        }
        self.schedule_catch();
    }

    fn schedule_takeover(&self, seat: Seat) {
        let Some(game) = &self.game else { return };
        let turn = game.view(seat).turn;
        schedule_after(
            self.cmd_tx.clone(),
            Duration::from_millis(bots::TAKEOVER_DELAY_MS),
            RoomCommand::BotTurn {
                epoch: self.epoch,
                turn,
                seat,
            },
        );
    }

    fn schedule_catch(&mut self) {
        let Some(game) = &self.game else { return };
        let Some(target) = game.uno_window() else {
            return;
        };
        let turn = game.view(target).turn;
        let _ = turn;
        for (idx, s) in self.seats.iter().enumerate() {
            let seat = idx as Seat;
            if seat == target {
                continue;
            }
            let OccupantState::Bot(kind) = &s.occupant else {
                continue;
            };
            let can_catch = game
                .legal_actions(seat)
                .contains(&Action::CatchUno { target });
            if !can_catch || !bots::roll_catch(kind) {
                continue;
            }
            schedule_after(
                self.cmd_tx.clone(),
                Duration::from_millis(bots::bot_catch_delay_ms()),
                RoomCommand::BotCatch {
                    epoch: self.epoch,
                    target,
                    seat,
                },
            );
            break;
        }
    }

    async fn on_bot_turn(&mut self, epoch: u64, turn: u32, seat: Seat) {
        if epoch != self.epoch || self.status != RoomStatus::Playing {
            return;
        }
        let action = {
            let Some(game) = &self.game else { return };
            if game.decision_player() != Some(seat) {
                return;
            }
            let view = game.view(seat);
            if view.turn != turn {
                return;
            }
            match &self.seats[usize::from(seat)].occupant {
                OccupantState::Bot(_) => match self.bots.get_mut(&seat) {
                    Some(bot) => bots::ensure_legal(&view, bot.act(&view)),
                    None => bots::takeover_action(&view),
                },
                OccupantState::Human(_) if !self.seats[usize::from(seat)].connected => {
                    bots::takeover_action(&view)
                }
                _ => None,
            }
        };
        if let Some(action) = action {
            self.apply_and_broadcast(seat, action).await;
        }
    }

    async fn on_bot_catch(&mut self, epoch: u64, target: Seat, seat: Seat) {
        if epoch != self.epoch || self.status != RoomStatus::Playing {
            return;
        }
        let can = match &self.game {
            Some(g) => {
                g.uno_window() == Some(target)
                    && g.legal_actions(seat).contains(&Action::CatchUno { target })
            }
            None => false,
        };
        if can {
            self.apply_and_broadcast(seat, Action::CatchUno { target })
                .await;
        }
    }

    async fn on_turn_timeout(&mut self, epoch: u64, turn: u32, seat: Seat) {
        if epoch != self.epoch || self.status != RoomStatus::Playing {
            return;
        }
        let valid = match &self.game {
            Some(g) => g.decision_player() == Some(seat) && g.view(seat).turn == turn,
            None => false,
        };
        if valid {
            self.apply_and_broadcast(seat, Action::Timeout).await;
        }
    }

    async fn on_game_timeout(&mut self, epoch: u64) {
        if epoch != self.epoch || self.status != RoomStatus::Playing {
            return;
        }
        let seat = self
            .game
            .as_ref()
            .and_then(|g| g.decision_player())
            .unwrap_or(0);
        self.apply_and_broadcast(seat, Action::EndByTime).await;
    }

    async fn on_auto_next_round(&mut self, epoch: u64, round: u32) {
        if epoch != self.epoch || self.status != RoomStatus::RoundOver {
            return;
        }
        let current = self.game.as_ref().map(|g| g.round()).unwrap_or(0);
        if current == round {
            self.do_next_round().await;
        }
    }

    // ---------- 推送 ----------

    fn seat_of(&self, user_id: i64) -> Option<Seat> {
        self.seats
            .iter()
            .enumerate()
            .find_map(|(idx, s)| match &s.occupant {
                OccupantState::Human(u) if u.id == user_id => Some(idx as Seat),
                _ => None,
            })
    }

    fn require_host(&mut self, user_id: i64) -> bool {
        if user_id == self.host_user_id {
            true
        } else {
            self.send_error(user_id, ErrorCode::NotHost, "not host", None);
            false
        }
    }

    fn room_info(&self) -> RoomInfo {
        let seats = self
            .seats
            .iter()
            .enumerate()
            .map(|(idx, s)| SeatInfo {
                seat: idx as Seat,
                occupant: match &s.occupant {
                    OccupantState::Empty => Occupant::Empty,
                    OccupantState::Human(u) => Occupant::Human { user: u.clone() },
                    OccupantState::Bot(k) => Occupant::Bot { kind: k.clone() },
                },
                ready: s.ready,
                connected: s.connected,
            })
            .collect();
        let spectators = self
            .clients
            .values()
            .filter(|c| c.spectator)
            .map(|c| c.user.clone())
            .collect();
        RoomInfo {
            code: self.code.clone(),
            host_user_id: self.host_user_id,
            public: self.public,
            config: self.config.clone(),
            status: self.status,
            seats,
            spectators,
            join_url: self.join_url.clone(),
        }
    }

    fn view_for(&self, user_id: i64) -> Option<uno_core::PlayerView> {
        let game = self.game.as_ref()?;
        match self.seat_of(user_id) {
            Some(seat) => Some(game.view(seat)),
            None => Some(game.spectator_view()),
        }
    }

    fn send_to(&self, user_id: i64, msg: ServerMsg) {
        if let Some(c) = self.clients.get(&user_id) {
            let _ = c.out.send(msg);
        }
    }

    fn send_error(&self, user_id: i64, code: ErrorCode, message: &str, seq: Option<u32>) {
        self.send_to(
            user_id,
            ServerMsg::Error {
                code,
                message: message.to_string(),
                seq,
            },
        );
    }

    fn send_room_state_to(&self, user_id: i64) {
        let info = self.room_info();
        self.send_to(user_id, ServerMsg::RoomState { room: info });
    }

    fn send_snapshot_to(&self, user_id: i64) {
        if let Some(view) = self.view_for(user_id) {
            let timer = self.turn_timer;
            self.send_to(user_id, ServerMsg::GameSnapshot { view, timer });
        }
    }

    fn broadcast(&self, msg: ServerMsg) {
        for c in self.clients.values() {
            let _ = c.out.send(msg.clone());
        }
    }

    fn broadcast_room_state(&self) {
        let info = self.room_info();
        self.broadcast(ServerMsg::RoomState { room: info });
    }

    fn broadcast_snapshot(&self) {
        let ids: Vec<i64> = self.clients.keys().copied().collect();
        for id in ids {
            self.send_snapshot_to(id);
        }
    }

    fn broadcast_events(&self, events: &[Event]) {
        let Some(game) = &self.game else { return };
        let timer = self.turn_timer_for_current();
        for (user_id, c) in &self.clients {
            let view = match self.seat_of(*user_id) {
                Some(seat) => game.view(seat),
                None => game.spectator_view(),
            };
            let filtered = match self.seat_of(*user_id) {
                Some(seat) => filter_events(events, Some(seat)),
                None => filter_events(events, None),
            };
            let _ = c.out.send(ServerMsg::GameEvents {
                events: filtered,
                view,
                timer,
            });
        }
    }

    fn turn_timer_for_current(&self) -> Option<TurnTimer> {
        self.turn_timer
    }
}
