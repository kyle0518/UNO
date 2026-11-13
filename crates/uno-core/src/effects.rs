use crate::card::{CardId, Color, Face, Seat};
use crate::config::{ModeKind, StackDraw, UnoRule, Wild4Rule};
use crate::events::{DrawReason, EliminationReason, Event, Phase};
use crate::state::GameState;

#[derive(Clone, Copy)]
enum WildPenalty {
    Fixed(u16),
    UntilColor(Color),
}

impl GameState {
    pub(crate) fn flush_drawn_chunk(
        &self,
        seat: Seat,
        chunk: &mut Vec<CardId>,
        reason: DrawReason,
        events: &mut Vec<Event>,
    ) {
        if chunk.is_empty() {
            return;
        }
        let cards = chunk.iter().map(|&id| self.card(id)).collect();
        events.push(Event::CardsDrawn {
            seat,
            count: chunk.len() as u8,
            cards: Some(cards),
            reason,
        });
        chunk.clear();
    }

    pub(crate) fn pop_draw_one(
        &mut self,
        seat: Seat,
        chunk: &mut Vec<CardId>,
        reason: DrawReason,
        events: &mut Vec<Event>,
    ) -> Option<CardId> {
        if self.draw_pile.is_empty() && self.discard.len() > 1 {
            self.flush_drawn_chunk(seat, chunk, reason, events);
            let top = self.discard.pop().expect("discard non-empty");
            self.draw_pile.append(&mut self.discard);
            self.discard.push(top);
            self.rng.shuffle(&mut self.draw_pile);
            events.push(Event::DeckReshuffled {
                size: self.draw_pile.len() as u16,
            });
        }
        let cid = self.draw_pile.pop()?;
        self.hands[usize::from(seat)].push(cid);
        chunk.push(cid);
        if chunk.len() == 255 {
            self.flush_drawn_chunk(seat, chunk, reason, events);
        }
        Some(cid)
    }

    pub(crate) fn draw_until_color(
        &mut self,
        seat: Seat,
        color: Color,
        reason: DrawReason,
        events: &mut Vec<Event>,
    ) {
        let mut chunk = Vec::new();
        while let Some(cid) = self.pop_draw_one(seat, &mut chunk, reason, events) {
            if self.card(cid).face(self.side).color == Some(color) {
                break;
            }
        }
        if self.hands[usize::from(seat)].len() >= 2 {
            self.uno_called[usize::from(seat)] = false;
        }
        self.flush_drawn_chunk(seat, &mut chunk, reason, events);
    }

    pub(crate) fn draw_many_cards(
        &mut self,
        seat: Seat,
        total: u16,
        reason: DrawReason,
        events: &mut Vec<Event>,
    ) {
        if total == 0 {
            return;
        }
        let mut remaining = total;
        while remaining > 0 {
            let chunk = remaining.min(255) as u8;
            let drawn = self.draw_cards(seat, chunk, reason, events);
            if drawn.len() < usize::from(chunk) {
                break;
            }
            remaining -= u16::from(chunk);
        }
    }

    /// 出牌後更新 UNO 視窗與自動喊 UNO（R2.7）。
    pub(crate) fn update_uno_on_one_card(&mut self, seat: Seat, events: &mut Vec<Event>) {
        let idx = usize::from(seat);
        if self.hands[idx].len() != 1 {
            return;
        }
        match self.config.uno_rule {
            UnoRule::Auto => {
                if !self.uno_called[idx] {
                    self.uno_called[idx] = true;
                    events.push(Event::UnoCalled { seat });
                }
                self.uno_window = None;
            }
            UnoRule::Manual => {
                if !self.uno_called[idx] {
                    self.uno_window = Some(seat);
                }
            }
        }
    }

    pub(crate) fn advance_to(&mut self, seat: Seat, events: &mut Vec<Event>) {
        self.phase = Phase::Play;
        self.current = seat;
        self.turn += 1;
        events.push(Event::TurnStarted { seat: self.current });
    }

    pub(crate) fn advance_turn(&mut self, from: Seat, steps: u8, events: &mut Vec<Event>) {
        let next = self.next_seat(from, steps);
        self.advance_to(next, events);
    }

    /// R4.2.8 Mercy Rule：手牌 >= 25 張立即淘汰並將手牌洗回牌堆底部；若只剩 1 人則結束該局。
    /// 若本函式已觸發 `finish_round` 回傳 `true`。
    pub(crate) fn check_mercy_elimination(&mut self, events: &mut Vec<Event>) -> bool {
        if self.config.mode != ModeKind::NoMercy {
            return false;
        }
        let n = usize::from(self.config.num_players);
        let mut any_eliminated = false;
        for s in 0..n {
            if !self.eliminated[s] && self.hands[s].len() >= 25 {
                let seat = s as Seat;
                self.eliminated[s] = true;
                self.elimination_order.push(seat);
                self.uno_called[s] = false;
                if self.uno_window == Some(seat) {
                    self.uno_window = None;
                }
                let mut reclaimed = std::mem::take(&mut self.hands[s]);
                self.rng.shuffle(&mut reclaimed);
                reclaimed.append(&mut self.draw_pile);
                self.draw_pile = reclaimed;
                events.push(Event::PlayerEliminated {
                    seat,
                    reason: EliminationReason::Mercy,
                });
                any_eliminated = true;
            }
        }
        if any_eliminated && self.active_player_count() <= 1 {
            let survivor = (0..n).find(|&s| !self.eliminated[s]).map(|s| s as Seat);
            self.finish_round(survivor, events);
            return true;
        }
        false
    }

    pub(crate) fn play_card(
        &mut self,
        seat: Seat,
        card_id: CardId,
        jump_in: bool,
        events: &mut Vec<Event>,
    ) {
        let idx = usize::from(seat);
        let pos = self.hands[idx]
            .iter()
            .position(|&id| id == card_id)
            .expect("legal Play card must exist in player's hand");
        self.hands[idx].remove(pos);

        let card = self.card(card_id);
        let face = card.face(self.side);

        // 必須在修改 active_color 前紀錄 +4 / +2(Flip) / DrawColor 出牌是否合法（R2.6.1 / R4.1.8-9）
        if matches!(
            face.face,
            Face::WildDrawFour | Face::WildDrawTwo | Face::WildDrawColor
        ) {
            self.wild4_was_legal = !self.has_active_color_in_hand(seat);
        }

        self.discard.push(card_id);
        events.push(Event::CardPlayed {
            seat,
            card,
            jump_in,
        });

        let hand_empty = self.hands[idx].is_empty();
        let defer_uno = matches!(face.face, Face::DiscardAll)
            || (self.config.seven_zero && matches!(face.face, Face::Number(0)));
        if !hand_empty && !defer_uno {
            self.update_uno_on_one_card(seat, events);
        }

        match face.face {
            Face::Number(n) => {
                self.active_color = face.color;
                if hand_empty {
                    self.finish_round(Some(seat), events);
                } else if self.config.seven_zero && n == 7 {
                    self.current = seat;
                    self.phase = Phase::ChooseTarget;
                } else if self.config.seven_zero && n == 0 {
                    let active: Vec<Seat> = (0..self.config.num_players)
                        .filter(|&s| !self.eliminated[usize::from(s)])
                        .collect();
                    let mut rotated: Vec<(Seat, Vec<CardId>)> = Vec::with_capacity(active.len());
                    for s in active {
                        let dest = self.next_seat(s, 1);
                        let hand = std::mem::take(&mut self.hands[usize::from(s)]);
                        rotated.push((dest, hand));
                    }
                    for (dest, hand) in rotated {
                        self.hands[usize::from(dest)] = hand;
                    }
                    self.uno_called.fill(false);
                    self.uno_window = None;
                    events.push(Event::HandsRotated {
                        direction: self.direction,
                    });
                    self.advance_turn(seat, 1, events);
                } else {
                    self.advance_turn(seat, 1, events);
                }
            }
            Face::Skip => {
                self.active_color = face.color;
                let skipped = self.next_seat(seat, 1);
                events.push(Event::PlayerSkipped { seat: skipped });
                if hand_empty {
                    self.finish_round(Some(seat), events);
                } else {
                    self.advance_turn(seat, 2, events);
                }
            }
            Face::Reverse => {
                self.active_color = face.color;
                self.direction = -self.direction;
                events.push(Event::DirectionChanged {
                    direction: self.direction,
                });
                let two_active = self.active_player_count() == 2;
                if two_active {
                    let skipped = self.next_seat(seat, 1);
                    events.push(Event::PlayerSkipped { seat: skipped });
                }
                if hand_empty {
                    self.finish_round(Some(seat), events);
                } else if two_active {
                    self.advance_turn(seat, 2, events);
                } else {
                    self.advance_turn(seat, 1, events);
                }
            }
            Face::DrawTwo => {
                self.active_color = face.color;
                self.apply_stackable_draw(seat, 2, hand_empty, events);
            }
            Face::DrawFour => {
                self.active_color = face.color;
                self.apply_stackable_draw(seat, 4, hand_empty, events);
            }
            Face::Wild => {
                self.active_color = None;
                self.current = seat;
                self.phase = Phase::ChooseColor;
            }
            Face::WildDrawFour | Face::WildDrawTwo | Face::WildDrawColor => {
                self.active_color = None;
                self.challenge_offender = Some(seat);
                self.current = seat;
                self.phase = Phase::ChooseColor;
            }
            Face::DrawOne => {
                self.active_color = face.color;
                self.apply_stackable_draw(seat, 1, hand_empty, events);
            }
            Face::DrawFive => {
                self.active_color = face.color;
                self.apply_stackable_draw(seat, 5, hand_empty, events);
            }
            Face::SkipEveryone => {
                self.active_color = face.color;
                let mut s = self.next_seat(seat, 1);
                while s != seat {
                    events.push(Event::PlayerSkipped { seat: s });
                    s = self.next_seat(s, 1);
                }
                if hand_empty {
                    self.finish_round(Some(seat), events);
                } else {
                    self.advance_turn(seat, 0, events);
                }
            }
            Face::Flip => {
                self.side = self.side.flip();
                self.draw_pile.reverse();
                events.push(Event::SideFlipped { side: self.side });
                let flipped_face = self.top_card().face(self.side);
                self.active_color = flipped_face.color;
                if flipped_face.color.is_none() {
                    self.flip_just_flipped = true;
                    self.current = seat;
                    self.phase = Phase::ChooseColor;
                } else if hand_empty {
                    self.finish_round(Some(seat), events);
                } else {
                    self.advance_turn(seat, 1, events);
                }
            }
            Face::DiscardAll => {
                self.active_color = face.color;
                let hand = std::mem::take(&mut self.hands[idx]);
                let mut extra_ids = Vec::new();
                let mut kept = Vec::with_capacity(hand.len());
                for cid in hand {
                    if self.card(cid).face(self.side).color == face.color {
                        extra_ids.push(cid);
                    } else {
                        kept.push(cid);
                    }
                }
                self.hands[idx] = kept;
                if !extra_ids.is_empty() {
                    let top_discard = self.discard.pop().expect("DiscardAll card was just pushed");
                    self.discard.extend(extra_ids.iter().copied());
                    self.discard.push(top_discard);
                    events.push(Event::CardsDiscarded {
                        seat,
                        cards: extra_ids.iter().map(|&id| self.card(id)).collect(),
                    });
                }
                if self.hands[idx].is_empty() {
                    self.finish_round(Some(seat), events);
                } else {
                    self.update_uno_on_one_card(seat, events);
                    self.advance_turn(seat, 1, events);
                }
            }
            Face::WildReverseDrawFour
            | Face::WildDrawSix
            | Face::WildDrawTen
            | Face::WildColorRoulette => {
                self.active_color = None;
                self.current = seat;
                self.phase = Phase::ChooseColor;
            }
        }
    }

    fn apply_stackable_draw(
        &mut self,
        seat: Seat,
        amount: u16,
        hand_empty: bool,
        events: &mut Vec<Event>,
    ) {
        if hand_empty {
            let victim = self.next_seat(seat, 1);
            let total = self.pending_draw + amount;
            let reason = if self.pending_draw > 0 {
                self.pending_draw = 0;
                events.push(Event::PendingDrawChanged { amount: 0 });
                DrawReason::Stack
            } else {
                DrawReason::Penalty
            };
            self.draw_many_cards(victim, total, reason, events);
            events.push(Event::PlayerSkipped { seat: victim });
            if !self.check_mercy_elimination(events) {
                self.finish_round(Some(seat), events);
            }
            return;
        }

        if self.config.stack_draw == StackDraw::None {
            let victim = self.next_seat(seat, 1);
            let next_after = self.next_seat(victim, 1);
            self.draw_many_cards(victim, amount, DrawReason::Penalty, events);
            events.push(Event::PlayerSkipped { seat: victim });
            if !self.check_mercy_elimination(events) {
                self.advance_to(next_after, events);
            }
        } else {
            self.pending_draw += amount;
            events.push(Event::PendingDrawChanged {
                amount: self.pending_draw,
            });
            self.advance_turn(seat, 1, events);
        }
    }

    pub(crate) fn resolve_choose_color(
        &mut self,
        seat: Seat,
        color: crate::card::Color,
        events: &mut Vec<Event>,
    ) {
        self.active_color = Some(color);
        events.push(Event::ColorChosen { seat, color });

        // R2.2.6：起始牌為 Wild 時，選完色後第一位玩家仍出牌
        if self.start_wild_pending {
            self.start_wild_pending = false;
            self.phase = Phase::Play;
            self.current = seat;
            return;
        }

        let top_face = self.top_card().face(self.side).face;
        let hand_empty = self.hands[usize::from(seat)].is_empty();

        // R4.1.7 / R4.1.12：剛由 Flip 牌翻成萬用頂牌時選色後直接輪下一位，不觸發頂牌原有效果
        if self.flip_just_flipped {
            self.flip_just_flipped = false;
            self.challenge_offender = None;
            self.wild4_was_legal = false;
            if hand_empty {
                self.finish_round(Some(seat), events);
            } else {
                self.advance_turn(seat, 1, events);
            }
            return;
        }

        match top_face {
            Face::Wild => {
                self.challenge_offender = None;
                self.wild4_was_legal = false;
                if hand_empty {
                    self.finish_round(Some(seat), events);
                } else {
                    self.advance_turn(seat, 1, events);
                }
            }
            Face::WildDrawFour => {
                self.resolve_challengeable_wild_draw(
                    seat,
                    WildPenalty::Fixed(4),
                    hand_empty,
                    events,
                );
            }
            Face::WildDrawTwo => {
                self.resolve_challengeable_wild_draw(
                    seat,
                    WildPenalty::Fixed(2),
                    hand_empty,
                    events,
                );
            }
            Face::WildDrawColor => {
                self.resolve_challengeable_wild_draw(
                    seat,
                    WildPenalty::UntilColor(color),
                    hand_empty,
                    events,
                );
            }
            Face::WildReverseDrawFour => {
                self.direction = -self.direction;
                events.push(Event::DirectionChanged {
                    direction: self.direction,
                });
                self.apply_stackable_draw(seat, 4, hand_empty, events);
            }
            Face::WildDrawSix => {
                self.apply_stackable_draw(seat, 6, hand_empty, events);
            }
            Face::WildDrawTen => {
                self.apply_stackable_draw(seat, 10, hand_empty, events);
            }
            Face::WildColorRoulette => {
                let victim = self.next_seat(seat, 1);
                let next_after = self.next_seat(victim, 1);
                self.draw_until_color(victim, color, DrawReason::Roulette, events);
                events.push(Event::PlayerSkipped { seat: victim });
                self.challenge_offender = None;
                self.wild4_was_legal = false;
                if !self.check_mercy_elimination(events) {
                    if hand_empty {
                        self.finish_round(Some(seat), events);
                    } else {
                        self.advance_to(next_after, events);
                    }
                }
            }
            _ => {
                if hand_empty {
                    self.finish_round(Some(seat), events);
                } else {
                    self.advance_turn(seat, 1, events);
                }
            }
        }
    }

    fn resolve_challengeable_wild_draw(
        &mut self,
        seat: Seat,
        penalty: WildPenalty,
        hand_empty: bool,
        events: &mut Vec<Event>,
    ) {
        if hand_empty {
            let victim = self.next_seat(seat, 1);
            match penalty {
                WildPenalty::Fixed(amt) => {
                    let total = self.pending_draw + amt;
                    let reason = if self.pending_draw > 0 {
                        self.pending_draw = 0;
                        events.push(Event::PendingDrawChanged { amount: 0 });
                        DrawReason::Stack
                    } else {
                        DrawReason::Penalty
                    };
                    self.draw_many_cards(victim, total, reason, events);
                }
                WildPenalty::UntilColor(c) => {
                    self.draw_until_color(victim, c, DrawReason::DrawColor, events);
                }
            }
            events.push(Event::PlayerSkipped { seat: victim });
            self.challenge_offender = None;
            self.wild4_was_legal = false;
            if !self.check_mercy_elimination(events) {
                self.finish_round(Some(seat), events);
            }
            return;
        }

        if let WildPenalty::Fixed(amt) = penalty {
            if self.config.stack_draw != StackDraw::None {
                self.pending_draw += amt;
                events.push(Event::PendingDrawChanged {
                    amount: self.pending_draw,
                });
                self.challenge_offender = None;
                self.wild4_was_legal = false;
                self.advance_turn(seat, 1, events);
                return;
            }
        }

        match self.config.wild4_rule {
            Wild4Rule::Challenge => {
                self.phase = Phase::Challenge;
                self.challenge_offender = Some(seat);
                self.current = self.next_seat(seat, 1);
                self.turn += 1;
                events.push(Event::TurnStarted { seat: self.current });
            }
            Wild4Rule::Strict | Wild4Rule::Free => {
                let victim = self.next_seat(seat, 1);
                let next_after = self.next_seat(victim, 1);
                match penalty {
                    WildPenalty::Fixed(amt) => {
                        self.draw_many_cards(victim, amt, DrawReason::Penalty, events);
                    }
                    WildPenalty::UntilColor(c) => {
                        self.draw_until_color(victim, c, DrawReason::DrawColor, events);
                    }
                }
                events.push(Event::PlayerSkipped { seat: victim });
                self.challenge_offender = None;
                self.wild4_was_legal = false;
                if !self.check_mercy_elimination(events) {
                    self.advance_to(next_after, events);
                }
            }
        }
    }

    /// 七零規則出 7 選目標換手牌（R3.2.1 / R3.2.3）。
    pub(crate) fn resolve_choose_target(
        &mut self,
        seat: Seat,
        target: Seat,
        events: &mut Vec<Event>,
    ) {
        self.hands.swap(usize::from(seat), usize::from(target));
        self.uno_called.fill(false);
        self.uno_window = None;
        events.push(Event::HandsSwapped { a: seat, b: target });
        self.advance_turn(seat, 1, events);
    }
}
