use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::card::Seat;
use crate::config::RuleConfig;
use crate::events::{Action, EngineError, Event};
use crate::state::GameState;

pub const REPLAY_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ReplayPlayerKind {
    Human,
    Bot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReplayPlayer {
    pub seat: Seat,
    pub name: String,
    #[ts(type = "number | null")]
    pub user_id: Option<i64>,
    pub kind: ReplayPlayerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bot: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NextRoundMarker {
    NextRound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum ReplayAction {
    Action(Action),
    NextRound(NextRoundMarker),
}

impl ReplayAction {
    pub const fn next_round() -> Self {
        Self::NextRound(NextRoundMarker::NextRound)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReplayStep {
    pub seat: Option<Seat>,
    pub action: ReplayAction,
    #[ts(type = "number")]
    pub t: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Replay {
    pub replay_version: u32,
    pub engine_version: String,
    pub config: RuleConfig,
    #[ts(type = "number")]
    pub seed: u64,
    pub players: Vec<ReplayPlayer>,
    pub steps: Vec<ReplayStep>,
    pub created_at: String,
}

impl Replay {
    pub fn replay(&self) -> Result<(GameState, Vec<Vec<Event>>), EngineError> {
        replay(self)
    }
}

/// 依 `Replay` 從頭重建對局並驗證每一步均合法。
pub fn replay(r: &Replay) -> Result<(GameState, Vec<Vec<Event>>), EngineError> {
    if r.replay_version != REPLAY_VERSION {
        return Err(EngineError::InvalidConfig(format!(
            "unsupported replay_version: {}",
            r.replay_version
        )));
    }
    let mut state = GameState::new(r.config.clone(), r.seed)?;
    let mut all_events = Vec::with_capacity(r.steps.len());

    for step in &r.steps {
        let events = match (step.seat, step.action) {
            (None, ReplayAction::NextRound(_)) => state.next_round()?,
            (Some(seat), ReplayAction::Action(action)) => state.apply(seat, action)?,
            _ => return Err(EngineError::IllegalAction),
        };
        state.set_last_step_time(step.t);
        all_events.push(events);
    }

    Ok((state, all_events))
}

impl GameState {
    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn steps(&self) -> &[ReplayStep] {
        &self.steps
    }

    /// 供宿主設定最後一步的時間戳（毫秒，相對於開局）。
    pub fn set_last_step_time(&mut self, t: u64) {
        if let Some(last) = self.steps.last_mut() {
            last.t = t;
        }
    }

    pub fn to_replay(&self, players: Vec<ReplayPlayer>, created_at: impl Into<String>) -> Replay {
        Replay {
            replay_version: REPLAY_VERSION,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            config: self.config.clone(),
            seed: self.seed,
            players,
            steps: self.steps.clone(),
            created_at: created_at.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Color;
    use crate::config::{RuleConfig, Scoring};
    use crate::events::Phase;
    use crate::rng::Rng;

    #[test]
    fn replay_round_trip_preserves_json_format_and_state_hash() {
        let mut cfg = RuleConfig::preset("classic", 4).unwrap();
        cfg.scoring = Scoring::Points { target: 60 };
        let mut game = GameState::new(cfg, 123_456_789).unwrap();
        let mut rng = Rng::new(987);

        let mut step_idx = 0u64;
        while !game.is_over() && step_idx < 3000 {
            if game.phase() == Phase::RoundOver {
                game.next_round().unwrap();
                step_idx += 1;
                game.set_last_step_time(step_idx * 100);
                continue;
            }
            if step_idx == 1500 {
                game.apply(0, Action::EndByTime).unwrap();
                step_idx += 1;
                game.set_last_step_time(step_idx * 100);
                break;
            }
            let dp = game.decision_player().unwrap();
            let actions = game.legal_actions(dp);
            let act = actions[(rng.next_u64() as usize) % actions.len()];
            game.apply(dp, act).unwrap();
            step_idx += 1;
            game.set_last_step_time(step_idx * 100);
        }

        if !game.is_over() {
            game.apply(0, Action::EndByTime).unwrap();
            game.set_last_step_time(999_999);
        }

        let players = vec![
            ReplayPlayer {
                seat: 0,
                name: "Alice".to_string(),
                user_id: Some(12),
                kind: ReplayPlayerKind::Human,
                bot: None,
            },
            ReplayPlayer {
                seat: 1,
                name: "Bot (困難)".to_string(),
                user_id: None,
                kind: ReplayPlayerKind::Bot,
                bot: Some("hard".to_string()),
            },
        ];

        let rep = game.to_replay(players, "2026-11-13T05:00:00Z");
        let json = serde_json::to_string(&rep).unwrap();
        assert!(json.contains("\"replay_version\":1"));
        assert!(json.contains("\"kind\":\"human\""));
        assert!(!json.contains("\"bot\":null"));
        assert!(json.contains("\"bot\":\"hard\""));

        let parsed: Replay = serde_json::from_str(&json).unwrap();
        let (replayed, events) = parsed.replay().unwrap();
        assert_eq!(events.len(), parsed.steps.len());
        assert_eq!(replayed.state_hash(), game.state_hash());
        assert_eq!(replayed.steps(), game.steps());

        // 驗證 ReplayAction::NextRound("next_round") 與 ReplayAction::Action JSON 格式
        let step_next = ReplayStep {
            seat: None,
            action: ReplayAction::next_round(),
            t: 90_000,
        };
        let step_next_json = serde_json::to_string(&step_next).unwrap();
        assert_eq!(
            step_next_json,
            "{\"seat\":null,\"action\":\"next_round\",\"t\":90000}"
        );
        let decoded_step: ReplayStep = serde_json::from_str(&step_next_json).unwrap();
        assert_eq!(decoded_step, step_next);

        let step_wild = ReplayStep {
            seat: Some(0),
            action: ReplayAction::Action(Action::ChooseColor { color: Color::Red }),
            t: 1500,
        };
        let decoded_wild: ReplayStep =
            serde_json::from_str(&serde_json::to_string(&step_wild).unwrap()).unwrap();
        assert_eq!(decoded_wild, step_wild);
    }
}
