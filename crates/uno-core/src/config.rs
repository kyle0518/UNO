use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::events::EngineError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ModeKind {
    Classic,
    Flip,
    NoMercy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StackDraw {
    None,
    Same,
    Progressive,
    NoMercy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Wild4Rule {
    Challenge,
    Strict,
    Free,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum UnoRule {
    Manual,
    Auto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum Scoring {
    Points { target: u32 },
    SingleRound,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RuleConfig {
    pub mode_id: String,
    pub mode: ModeKind,
    pub num_players: u8,
    pub hand_size: u8,
    pub deck_count: u8,
    pub stack_draw: StackDraw,
    pub seven_zero: bool,
    pub jump_in: bool,
    pub draw_to_match: bool,
    pub force_play: bool,
    pub wild4_rule: Wild4Rule,
    pub uno_rule: UnoRule,
    pub uno_penalty: u8,
    pub scoring: Scoring,
    pub teams: Option<u8>,
    pub elimination: bool,
    pub turn_seconds: u16,
    pub game_seconds: u16,
}

impl RuleConfig {
    fn base(mode_id: &str, mode: ModeKind, num_players: u8) -> Self {
        Self {
            mode_id: mode_id.to_string(),
            mode,
            num_players,
            hand_size: 7,
            deck_count: 1,
            stack_draw: StackDraw::None,
            seven_zero: false,
            jump_in: false,
            draw_to_match: false,
            force_play: false,
            wild4_rule: Wild4Rule::Challenge,
            uno_rule: UnoRule::Manual,
            uno_penalty: 2,
            scoring: Scoring::Points { target: 500 },
            teams: None,
            elimination: false,
            turn_seconds: 30,
            game_seconds: 0,
        }
    }

    /// 依 RULES §4 回傳模式預設值。未知 mode_id 或人數不合法回傳 Err。
    pub fn preset(mode_id: &str, num_players: u8) -> Result<Self, EngineError> {
        let mut cfg = match mode_id {
            "classic" | "custom" => Self::base(mode_id, ModeKind::Classic, num_players),
            "party" => {
                let mut c = Self::base(mode_id, ModeKind::Classic, num_players);
                c.stack_draw = StackDraw::Progressive;
                c.seven_zero = true;
                c.jump_in = true;
                c
            }
            "flip" => Self::base(mode_id, ModeKind::Flip, num_players),
            "no_mercy" => Self::base(mode_id, ModeKind::NoMercy, num_players),
            "teams" => {
                let mut c = Self::base(mode_id, ModeKind::Classic, num_players);
                c.teams = Some(2);
                c
            }
            "elimination" => {
                let mut c = Self::base(mode_id, ModeKind::Classic, num_players);
                c.elimination = true;
                c
            }
            "blitz" => {
                let mut c = Self::base(mode_id, ModeKind::Classic, num_players);
                c.turn_seconds = 5;
                c.game_seconds = 180;
                c.scoring = Scoring::SingleRound;
                c.uno_rule = UnoRule::Auto;
                c
            }
            "daily" => {
                let mut c = Self::base(mode_id, ModeKind::Classic, num_players);
                c.scoring = Scoring::SingleRound;
                c
            }
            other => {
                return Err(EngineError::InvalidConfig(format!(
                    "unknown mode_id: {other}"
                )))
            }
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// 依 RULES 檢查並修正設定；人數或其他限制不合法時回傳錯誤。
    pub fn validate(&mut self) -> Result<(), EngineError> {
        if !(2..=10).contains(&self.num_players) {
            return Err(EngineError::InvalidConfig(format!(
                "num_players must be in 2..=10, got {}",
                self.num_players
            )));
        }
        if self.mode == ModeKind::NoMercy && !(2..=6).contains(&self.num_players) {
            return Err(EngineError::InvalidConfig(format!(
                "no_mercy num_players must be in 2..=6, got {}",
                self.num_players
            )));
        }
        if self.mode_id == "daily" && self.num_players != 4 {
            return Err(EngineError::InvalidConfig(format!(
                "daily mode requires 4 players, got {}",
                self.num_players
            )));
        }
        if let Some(t) = self.teams {
            if t != 2 || (self.num_players != 4 && self.num_players != 6) {
                return Err(EngineError::InvalidConfig(format!(
                    "teams mode requires teams=Some(2) and 4 or 6 players, got teams={t}, players={}",
                    self.num_players
                )));
            }
        }
        if self.elimination && self.num_players < 3 {
            return Err(EngineError::InvalidConfig(format!(
                "elimination mode requires at least 3 players, got {}",
                self.num_players
            )));
        }
        if !(1..=15).contains(&self.hand_size) {
            return Err(EngineError::InvalidConfig(format!(
                "hand_size must be in 1..=15, got {}",
                self.hand_size
            )));
        }
        if !(1..=3).contains(&self.deck_count) {
            return Err(EngineError::InvalidConfig(format!(
                "deck_count must be in 1..=3, got {}",
                self.deck_count
            )));
        }
        if self.uno_penalty > 6 {
            return Err(EngineError::InvalidConfig(format!(
                "uno_penalty must be in 0..=6, got {}",
                self.uno_penalty
            )));
        }
        if let Scoring::Points { target } = self.scoring {
            if target == 0 {
                return Err(EngineError::InvalidConfig(
                    "target score must be > 0".to_string(),
                ));
            }
        }

        let single_deck_size: usize = match self.mode {
            ModeKind::Classic => 108,
            ModeKind::Flip => 112,
            ModeKind::NoMercy => 168,
        };
        let total_cards = single_deck_size * usize::from(self.deck_count);
        let needed = usize::from(self.num_players) * usize::from(self.hand_size) + 1;
        if needed > total_cards {
            return Err(EngineError::InvalidConfig(format!(
                "not enough cards ({total_cards}) to deal {needed} cards"
            )));
        }

        // 模式強制修正
        match self.mode {
            ModeKind::Flip => {
                self.stack_draw = StackDraw::None;
                self.seven_zero = false;
            }
            ModeKind::NoMercy => {
                self.stack_draw = StackDraw::NoMercy;
                self.seven_zero = true;
                self.draw_to_match = true;
                self.wild4_rule = Wild4Rule::Free;
                self.scoring = Scoring::SingleRound;
            }
            ModeKind::Classic => {}
        }

        if self.mode_id == "daily" {
            self.scoring = Scoring::SingleRound;
        }

        // R3.1.6 開啟疊加時 +4 不能質疑
        if self.stack_draw != StackDraw::None && self.wild4_rule == Wild4Rule::Challenge {
            self.wild4_rule = Wild4Rule::Free;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_pass_validate() {
        let cases = [
            ("classic", 4),
            ("party", 4),
            ("flip", 4),
            ("no_mercy", 4),
            ("teams", 4),
            ("teams", 6),
            ("elimination", 3),
            ("blitz", 4),
            ("daily", 4),
            ("custom", 2),
        ];
        for (mode_id, n) in cases {
            let mut cfg = RuleConfig::preset(mode_id, n).expect("preset must succeed");
            assert_eq!(cfg.num_players, n);
            cfg.validate().expect("re-validate must succeed");
        }
    }

    #[test]
    fn preset_validation_rejects_invalid_player_counts_and_unknown_mode() {
        assert!(RuleConfig::preset("unknown", 4).is_err());
        assert!(RuleConfig::preset("classic", 1).is_err());
        assert!(RuleConfig::preset("classic", 11).is_err());
        assert!(RuleConfig::preset("no_mercy", 7).is_err());
        assert!(RuleConfig::preset("teams", 2).is_err());
        assert!(RuleConfig::preset("teams", 5).is_err());
        assert!(RuleConfig::preset("elimination", 2).is_err());
        assert!(RuleConfig::preset("daily", 2).is_err());
    }

    #[test]
    fn validate_enforces_mode_rules() {
        let mut flip = RuleConfig::preset("flip", 4).unwrap();
        flip.stack_draw = StackDraw::Progressive;
        flip.seven_zero = true;
        flip.validate().unwrap();
        assert_eq!(flip.stack_draw, StackDraw::None);
        assert!(!flip.seven_zero);

        let mut nm = RuleConfig::preset("no_mercy", 4).unwrap();
        nm.stack_draw = StackDraw::None;
        nm.seven_zero = false;
        nm.draw_to_match = false;
        nm.wild4_rule = Wild4Rule::Challenge;
        nm.scoring = Scoring::Points { target: 500 };
        nm.validate().unwrap();
        assert_eq!(nm.stack_draw, StackDraw::NoMercy);
        assert!(nm.seven_zero);
        assert!(nm.draw_to_match);
        assert_eq!(nm.wild4_rule, Wild4Rule::Free);
        assert_eq!(nm.scoring, Scoring::SingleRound);

        let mut party = RuleConfig::preset("party", 4).unwrap();
        assert_eq!(party.wild4_rule, Wild4Rule::Free);
        party.uno_rule = UnoRule::Auto;
        party.uno_penalty = 5;
        party.validate().unwrap();
        assert_eq!(party.uno_penalty, 5);
    }
}
