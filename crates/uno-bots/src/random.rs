use uno_core::{Action, PlayerView, Rng};

use crate::Bot;

/// 隨機挑選合法動作的基準 Bot；若可喊 UNO 會優先喊出以免被無謂抓罰。
#[derive(Debug, Clone)]
pub struct RandomBot {
    rng: Rng,
}

impl RandomBot {
    /// 以指定決定性種子建立 `RandomBot`。
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
        }
    }
}

impl Bot for RandomBot {
    fn name(&self) -> &'static str {
        "random"
    }

    fn act(&mut self, state_view: &PlayerView) -> Action {
        if state_view.legal.contains(&Action::CallUno) {
            return Action::CallUno;
        }
        let len = state_view.legal.len();
        if len == 0 {
            return Action::Draw;
        }
        let idx = self.rng.below(len as u64) as usize;
        state_view.legal[idx]
    }
}
