use crate::card::{CardId, Color, Face, Seat};
use crate::config::ModeKind;
use crate::events::{Action, DrawReason, EngineError, Event, Phase};
use crate::replay::{ReplayAction, ReplayStep};
use crate::state::GameState;

fn is_turn_action(action: &Action) -> bool {
    matches!(
        action,
        Action::Play { .. }
            | Action::Draw
            | Action::Pass
            | Action::ChooseColor { .. }
            | Action::ChooseTarget { .. }
            | Action::Challenge
            | Action::Accept
    )
}

impl GameState {
    /// 套用玩家動作並回傳產生之事件序列；狀態維持決定性。
    pub fn apply(&mut self, seat: Seat, action: Action) -> Result<Vec<Event>, EngineError> {
        if self.phase == Phase::GameOver {
            return Err(EngineError::GameOver);
        }

        if matches!(action, Action::EndByTime) {
            let mut events = Vec::new();
            let rec_seat = self.decision_player().unwrap_or(seat);
            self.finish_by_time(&mut events);
            self.steps.push(ReplayStep {
                seat: Some(rec_seat),
                action: ReplayAction::Action(Action::EndByTime),
                t: 0,
            });
            return Ok(events);
        }

        if self.phase == Phase::RoundOver {
            return Err(EngineError::IllegalAction);
        }

        if usize::from(seat) >= usize::from(self.config.num_players)
            || self.eliminated[usize::from(seat)]
        {
            return Err(EngineError::IllegalAction);
        }

        if matches!(action, Action::Timeout) {
            if self.decision_player() != Some(seat) {
                return Err(EngineError::NotYourTurn);
            }
            let mut events = Vec::new();
            self.resolve_timeout(seat, &mut events);
            self.steps.push(ReplayStep {
                seat: Some(seat),
                action: ReplayAction::Action(Action::Timeout),
                t: 0,
            });
            return Ok(events);
        }

        if is_turn_action(&action) && self.decision_player() != Some(seat) {
            return Err(EngineError::NotYourTurn);
        }

        if !self.legal_actions(seat).contains(&action) {
            return Err(EngineError::IllegalAction);
        }

        let mut events = Vec::new();
        self.apply_legal_action(seat, action, &mut events);
        self.steps.push(ReplayStep {
            seat: Some(seat),
            action: ReplayAction::Action(action),
            t: 0,
        });
        Ok(events)
    }

    fn apply_legal_action(&mut self, seat: Seat, action: Action, events: &mut Vec<Event>) {
        // R2.7.1：任何會推進回合的動作都會關閉既有 UNO 視窗，
        // 唯獨 target 剛出 Wild / +4 / 7 後緊接 ChooseColor / ChooseTarget 不關閉。
        let is_turn_advancing = !matches!(action, Action::CallUno | Action::CatchUno { .. });
        if is_turn_advancing {
            let keep = self.uno_window == Some(seat)
                && matches!(
                    action,
                    Action::ChooseColor { .. } | Action::ChooseTarget { .. }
                );
            if !keep {
                self.uno_window = None;
            }
        }

        match action {
            Action::Play { card } => {
                self.drawn_card = None;
                self.play_card(seat, card, false, events);
            }
            Action::Draw => {
                self.resolve_draw_action(seat, events);
            }
            Action::Pass => {
                self.drawn_card = None;
                if self.hands[usize::from(seat)].len() >= 2 {
                    self.uno_called[usize::from(seat)] = false;
                }
                self.advance_turn(seat, 1, events);
            }
            Action::ChooseColor { color } => {
                self.resolve_choose_color(seat, color, events);
            }
            Action::ChooseTarget { target } => {
                self.resolve_choose_target(seat, target, events);
            }
            Action::CallUno => {
                let idx = usize::from(seat);
                self.uno_called[idx] = true;
                if self.uno_window == Some(seat) {
                    self.uno_window = None;
                }
                events.push(Event::UnoCalled { seat });
            }
            Action::CatchUno { target } => {
                self.uno_window = None;
                let actual_penalty =
                    self.available_draw_count()
                        .min(usize::from(self.config.uno_penalty)) as u8;
                events.push(Event::UnoCaught {
                    catcher: seat,
                    target,
                    penalty: actual_penalty,
                });
                let next_from_current = self.next_seat(self.current, 1);
                self.draw_cards(target, self.config.uno_penalty, DrawReason::Uno, events);
                if !self.check_mercy_elimination(events)
                    && self.eliminated[usize::from(self.current)]
                {
                    self.advance_to(next_from_current, events);
                }
            }
            Action::Accept => {
                let next_after = self.next_seat(seat, 1);
                let top_face = self.top_card().face(self.side).face;
                match top_face {
                    Face::WildDrawTwo => self.draw_many_cards(seat, 2, DrawReason::Penalty, events),
                    Face::WildDrawColor => {
                        let color = self
                            .active_color
                            .expect("active_color must be set for WildDrawColor");
                        self.draw_until_color(seat, color, DrawReason::DrawColor, events);
                    }
                    _ => self.draw_many_cards(seat, 4, DrawReason::Penalty, events),
                }
                events.push(Event::PlayerSkipped { seat });
                self.challenge_offender = None;
                self.wild4_was_legal = false;
                if !self.check_mercy_elimination(events) {
                    self.advance_to(next_after, events);
                }
            }
            Action::Challenge => {
                let offender = self
                    .challenge_offender
                    .expect("challenge_offender must be set in Phase::Challenge");
                let success = !self.wild4_was_legal;
                events.push(Event::ChallengeResolved {
                    challenger: seat,
                    offender,
                    success,
                });
                let top_face = self.top_card().face(self.side).face;
                let chosen_color = self.active_color;
                self.challenge_offender = None;
                self.wild4_was_legal = false;
                if success {
                    match top_face {
                        Face::WildDrawTwo => {
                            self.draw_many_cards(offender, 2, DrawReason::Challenge, events);
                        }
                        Face::WildDrawColor => {
                            let color =
                                chosen_color.expect("active_color must be set for WildDrawColor");
                            self.draw_until_color(offender, color, DrawReason::DrawColor, events);
                        }
                        _ => self.draw_many_cards(offender, 4, DrawReason::Challenge, events),
                    }
                    if !self.check_mercy_elimination(events) {
                        self.phase = Phase::Play;
                        self.current = seat;
                    }
                } else {
                    let next_after = self.next_seat(seat, 1);
                    match top_face {
                        Face::WildDrawTwo => {
                            self.draw_many_cards(seat, 4, DrawReason::Challenge, events);
                        }
                        Face::WildDrawColor => {
                            let color =
                                chosen_color.expect("active_color must be set for WildDrawColor");
                            self.draw_until_color(seat, color, DrawReason::DrawColor, events);
                            self.draw_many_cards(seat, 2, DrawReason::Challenge, events);
                        }
                        _ => self.draw_many_cards(seat, 6, DrawReason::Challenge, events),
                    }
                    events.push(Event::PlayerSkipped { seat });
                    if !self.check_mercy_elimination(events) {
                        self.advance_to(next_after, events);
                    }
                }
            }
            Action::JumpIn { card } => {
                self.drawn_card = None;
                self.play_card(seat, card, true, events);
            }
            Action::Timeout | Action::EndByTime => {
                unreachable!("handled prior to apply_legal_action")
            }
        }
    }

    fn resolve_draw_action(&mut self, seat: Seat, events: &mut Vec<Event>) {
        let next_after = self.next_seat(seat, 1);

        if self.pending_draw > 0 {
            let total = self.pending_draw;
            self.pending_draw = 0;
            events.push(Event::PendingDrawChanged { amount: 0 });
            self.draw_many_cards(seat, total, DrawReason::Stack, events);
            events.push(Event::PlayerSkipped { seat });
            if !self.check_mercy_elimination(events) {
                self.advance_to(next_after, events);
            }
            return;
        }

        if !self.config.draw_to_match {
            let drawn = self.draw_cards(seat, 1, DrawReason::Voluntary, events);
            if self.check_mercy_elimination(events) {
                return;
            }
            if self.eliminated[usize::from(seat)] {
                self.drawn_card = None;
                self.advance_to(next_after, events);
                return;
            }
            if let Some(&cid) = drawn.first() {
                let face = self.card(cid).face(self.side);
                if self.can_play(seat, &face) {
                    self.drawn_card = Some(cid);
                    self.phase = Phase::DrawnCardDecision;
                } else {
                    self.drawn_card = None;
                    self.advance_to(next_after, events);
                }
            } else {
                self.drawn_card = None;
                self.advance_to(next_after, events);
            }
            return;
        }

        // R2.5.6 draw_to_match：持續抽牌直到抽到可出之牌或牌堆與棄牌堆皆耗盡（No Mercy 達 25 張立即淘汰）
        let mut drawn_chunk: Vec<CardId> = Vec::new();
        let mut matched_playable: Option<CardId> = None;

        while let Some(cid) =
            self.pop_draw_one(seat, &mut drawn_chunk, DrawReason::Voluntary, events)
        {
            if self.config.mode == ModeKind::NoMercy && self.hands[usize::from(seat)].len() >= 25 {
                matched_playable = None;
                break;
            }
            let face = self.card(cid).face(self.side);
            if self.can_play(seat, &face) {
                matched_playable = Some(cid);
                break;
            }
        }

        if self.hands[usize::from(seat)].len() >= 2 {
            self.uno_called[usize::from(seat)] = false;
        }

        self.flush_drawn_chunk(seat, &mut drawn_chunk, DrawReason::Voluntary, events);
        if self.check_mercy_elimination(events) {
            return;
        }
        if self.eliminated[usize::from(seat)] {
            self.drawn_card = None;
            self.advance_to(next_after, events);
            return;
        }

        if let Some(cid) = matched_playable {
            self.drawn_card = Some(cid);
            self.phase = Phase::DrawnCardDecision;
        } else {
            self.drawn_card = None;
            self.advance_to(next_after, events);
        }
    }

    fn resolve_timeout(&mut self, seat: Seat, events: &mut Vec<Event>) {
        match self.phase {
            Phase::Play => {
                if self.pending_draw > 0 || self.available_draw_count() > 0 {
                    self.apply_legal_action(seat, Action::Draw, events);
                    if self.phase == Phase::DrawnCardDecision {
                        self.timeout_drawn_decision(seat, events);
                    }
                } else {
                    let first_play = self
                        .legal_actions(seat)
                        .into_iter()
                        .find(|a| matches!(a, Action::Play { .. }));
                    if let Some(play_action) = first_play {
                        self.apply_legal_action(seat, play_action, events);
                        self.timeout_followups(seat, events);
                    } else {
                        self.apply_legal_action(seat, Action::Pass, events);
                    }
                }
            }
            Phase::DrawnCardDecision => {
                self.timeout_drawn_decision(seat, events);
            }
            Phase::ChooseColor => {
                let color = self.timeout_best_color(seat);
                self.apply_legal_action(seat, Action::ChooseColor { color }, events);
            }
            Phase::ChooseTarget => {
                let target = self.next_seat(seat, 1);
                self.apply_legal_action(seat, Action::ChooseTarget { target }, events);
            }
            Phase::Challenge => {
                self.apply_legal_action(seat, Action::Accept, events);
            }
            Phase::RoundOver | Phase::GameOver => {}
        }
    }

    fn timeout_drawn_decision(&mut self, seat: Seat, events: &mut Vec<Event>) {
        if self.config.force_play {
            let cid = self
                .drawn_card
                .expect("drawn_card must be set in DrawnCardDecision");
            self.apply_legal_action(seat, Action::Play { card: cid }, events);
            self.timeout_followups(seat, events);
        } else {
            self.apply_legal_action(seat, Action::Pass, events);
        }
    }

    fn timeout_followups(&mut self, seat: Seat, events: &mut Vec<Event>) {
        if self.phase == Phase::ChooseColor && self.decision_player() == Some(seat) {
            let color = self.timeout_best_color(seat);
            self.apply_legal_action(seat, Action::ChooseColor { color }, events);
        }
        if self.phase == Phase::ChooseTarget && self.decision_player() == Some(seat) {
            let target = self.next_seat(seat, 1);
            self.apply_legal_action(seat, Action::ChooseTarget { target }, events);
        }
    }

    fn timeout_best_color(&self, seat: Seat) -> Color {
        let palette = Color::colors_for_side(self.side);
        let mut best_color = palette[0];
        let mut best_count = 0usize;
        for c in palette {
            let count = self.hands[usize::from(seat)]
                .iter()
                .filter(|&&cid| self.card(cid).face(self.side).color == Some(c))
                .count();
            if count > best_count {
                best_count = count;
                best_color = c;
            }
        }
        best_color
    }
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stack_tests.rs"]
mod stack_tests;

#[cfg(test)]
#[path = "seven_zero_jump_in_tests.rs"]
mod seven_zero_jump_in_tests;

#[cfg(test)]
#[path = "flip_tests.rs"]
mod flip_tests;

#[cfg(test)]
#[path = "no_mercy_tests.rs"]
mod no_mercy_tests;
