//! WebAssembly 綁定，將 `uno-core` 與 `uno-bots` 提供給前端單機對局與 AI 實驗室使用。

use std::str::FromStr;

use serde::{de::DeserializeOwned, Serialize};
use serde_wasm_bindgen::Serializer;
use uno_bots::{make_bot, Bot, BotKind, HardBot, MOBILE_SIMS};
use uno_core::{
    action_mask, filter_events as core_filter_events, index_to_action as core_index_to_action,
    observe, replay as core_replay, Action, Event, GameState, PlayerView, Replay, ReplayPlayer,
    ReplayPlayerKind, RuleConfig, Seat,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

fn from_js<T: DeserializeOwned>(value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))
}

fn default_replay_players(num_players: u8) -> Vec<ReplayPlayer> {
    (0..num_players)
        .map(|seat| ReplayPlayer {
            seat,
            name: if seat == 0 {
                "You".to_string()
            } else {
                format!("AI {seat}")
            },
            user_id: None,
            kind: if seat == 0 {
                ReplayPlayerKind::Human
            } else {
                ReplayPlayerKind::Bot
            },
            bot: if seat == 0 {
                None
            } else {
                Some("heuristic".to_string())
            },
        })
        .collect()
}

fn validate_and_extract_rankings(json: &str) -> Result<Option<Vec<Seat>>, String> {
    let parsed: Replay = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let (state, _) = core_replay(&parsed).map_err(|e| e.to_string())?;
    Ok(state.rankings().map(|r| r.to_vec()))
}

fn build_wasm_bot(kind: &str, seed: u64) -> Result<Box<dyn Bot>, String> {
    let parsed = BotKind::from_str(kind).map_err(|e| e.to_string())?;
    match parsed {
        BotKind::Hard => Ok(Box::new(HardBot::new(seed, MOBILE_SIMS))),
        other => make_bot(&other, seed).map_err(|e| e.to_string()),
    }
}

#[wasm_bindgen]
pub struct WasmGame {
    state: GameState,
    replay_players: Vec<ReplayPlayer>,
    created_at: String,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue, seed: u64) -> Result<WasmGame, JsError> {
        let cfg: RuleConfig = from_js(config)?;
        let num_players = cfg.num_players;
        let state = GameState::new(cfg, seed).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self {
            state,
            replay_players: default_replay_players(num_players),
            created_at: String::new(),
        })
    }

    pub fn set_replay_meta(&mut self, players: JsValue, created_at: &str) -> Result<(), JsError> {
        let parsed: Vec<ReplayPlayer> = from_js(players)?;
        self.replay_players = parsed;
        self.created_at = created_at.to_string();
        Ok(())
    }

    pub fn initial_events(&self) -> Result<JsValue, JsError> {
        to_js(self.state.initial_events())
    }

    pub fn view(&self, seat: u8) -> Result<JsValue, JsError> {
        to_js(&self.state.view(seat))
    }

    pub fn spectator_view(&self) -> Result<JsValue, JsError> {
        to_js(&self.state.spectator_view())
    }

    pub fn god_view(&self) -> Result<JsValue, JsError> {
        to_js(&self.state.god_view())
    }

    pub fn decision_player(&self) -> Option<u8> {
        self.state.decision_player()
    }

    pub fn legal(&self, seat: u8) -> Result<JsValue, JsError> {
        to_js(&self.state.legal_actions(seat))
    }

    pub fn apply(&mut self, seat: u8, action: JsValue) -> Result<JsValue, JsError> {
        let act: Action = from_js(action)?;
        let events = self
            .state
            .apply(seat, act)
            .map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&events)
    }

    pub fn apply_with_time(
        &mut self,
        seat: u8,
        action: JsValue,
        t_ms: u64,
    ) -> Result<JsValue, JsError> {
        let act: Action = from_js(action)?;
        let events = self
            .state
            .apply(seat, act)
            .map_err(|e| JsError::new(&e.to_string()))?;
        self.state.set_last_step_time(t_ms);
        to_js(&events)
    }

    pub fn next_round(&mut self) -> Result<JsValue, JsError> {
        let events = self
            .state
            .next_round()
            .map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&events)
    }

    pub fn is_over(&self) -> bool {
        self.state.is_over()
    }

    pub fn observe(&self, seat: u8) -> Vec<f32> {
        observe(&self.state, seat)
    }

    pub fn action_mask(&self, seat: u8) -> Vec<u8> {
        action_mask(&self.state, seat)
            .into_iter()
            .map(u8::from)
            .collect()
    }

    pub fn index_to_action(&self, seat: u8, idx: u32) -> Result<JsValue, JsError> {
        let act = core_index_to_action(&self.state, seat, idx as usize)
            .map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&act)
    }

    pub fn replay_json(&self) -> Result<String, JsError> {
        let replay = self
            .state
            .to_replay(self.replay_players.clone(), self.created_at.clone());
        serde_json::to_string(&replay).map_err(|e| JsError::new(&e.to_string()))
    }
}

#[wasm_bindgen]
pub fn filter_events(events: JsValue, seat: Option<u8>) -> Result<JsValue, JsError> {
    let parsed: Vec<Event> = from_js(events)?;
    to_js(&core_filter_events(&parsed, seat))
}

#[wasm_bindgen]
pub fn preset(mode_id: &str, num_players: u8) -> Result<JsValue, JsError> {
    let cfg = RuleConfig::preset(mode_id, num_players).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&cfg)
}

#[wasm_bindgen]
pub fn validate_config(config: JsValue) -> Result<JsValue, JsError> {
    let mut cfg: RuleConfig = from_js(config)?;
    cfg.validate().map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&cfg)
}

#[wasm_bindgen]
pub struct WasmBot {
    inner: Box<dyn Bot>,
}

#[wasm_bindgen]
impl WasmBot {
    #[wasm_bindgen(constructor)]
    pub fn new(kind: &str, seed: u64) -> Result<WasmBot, JsError> {
        let inner = build_wasm_bot(kind, seed).map_err(|e| JsError::new(&e))?;
        Ok(Self { inner })
    }

    pub fn act(&mut self, view: JsValue) -> Result<JsValue, JsError> {
        let parsed: PlayerView = from_js(view)?;
        let action = self.inner.act(&parsed);
        to_js(&action)
    }
}

#[wasm_bindgen]
pub fn replay_validate(json: &str) -> Result<JsValue, JsError> {
    let rankings = validate_and_extract_rankings(json).map_err(|e| JsError::new(&e))?;
    to_js(&rankings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_builder_supports_random_heuristic_and_hard() {
        assert!(build_wasm_bot("random", 1).is_ok());
        assert!(build_wasm_bot("heuristic", 1).is_ok());
        assert!(build_wasm_bot("hard", 1).is_ok());
        assert!(build_wasm_bot("onnx:foo.onnx", 1).is_err());
        assert!(build_wasm_bot("invalid", 1).is_err());
    }

    #[test]
    fn replay_validate_replays_complete_match() {
        let cfg = RuleConfig::preset("classic", 4).expect("preset valid");
        let mut state = GameState::new(cfg, 77).expect("game init");
        let mut bots: Vec<Box<dyn Bot>> = (0..4u64)
            .map(|i| build_wasm_bot("heuristic", 10 + i).expect("bot"))
            .collect();

        while !state.is_over() {
            if let Some(seat) = state.decision_player() {
                let view = state.view(seat);
                let act = bots[usize::from(seat)].act(&view);
                state.apply(seat, act).expect("legal move");
            } else {
                state.next_round().expect("next round");
            }
        }

        let replay = state.to_replay(default_replay_players(4), "");
        let json = serde_json::to_string(&replay).expect("serialize");
        let rankings = validate_and_extract_rankings(&json).expect("valid replay");
        assert!(rankings.is_some());
        assert_eq!(rankings.expect("some").len(), 4);
    }
}
