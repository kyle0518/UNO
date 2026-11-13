use serde::{Deserialize, Serialize};

use crate::card::{Card, CardId, Color, Face, Seat, Side};
use crate::config::RuleConfig;
use crate::deck::build_deck;
use crate::events::{DrawReason, EngineError, Event, Phase};
use crate::replay::ReplayStep;
use crate::rng::Rng;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub(crate) config: RuleConfig,
    pub(crate) seed: u64,
    pub(crate) rng: Rng,
    pub(crate) cards: Vec<Card>,
    pub(crate) side: Side,
    pub(crate) hands: Vec<Vec<CardId>>,
    pub(crate) draw_pile: Vec<CardId>,
    pub(crate) discard: Vec<CardId>,
    pub(crate) current: Seat,
    pub(crate) direction: i8,
    pub(crate) active_color: Option<Color>,
    pub(crate) pending_draw: u16,
    pub(crate) phase: Phase,
    pub(crate) uno_called: Vec<bool>,
    pub(crate) uno_window: Option<Seat>,
    pub(crate) drawn_card: Option<CardId>,
    pub(crate) wild4_was_legal: bool,
    pub(crate) challenge_offender: Option<Seat>,
    pub(crate) eliminated: Vec<bool>,
    pub(crate) elimination_order: Vec<Seat>,
    pub(crate) scores: Vec<i32>,
    pub(crate) round: u32,
    pub(crate) dealer: Seat,
    pub(crate) turn: u32,
    pub(crate) rankings: Option<Vec<Seat>>,
    pub(crate) start_wild_pending: bool,
    pub(crate) flip_just_flipped: bool,
    pub(crate) initial_events: Vec<Event>,
    pub(crate) steps: Vec<ReplayStep>,
}

impl GameState {
    /// 建立新對局、洗牌、發牌並翻開起始牌（RULES §2.2）。
    pub fn new(mut config: RuleConfig, seed: u64) -> Result<Self, EngineError> {
        config.validate()?;
        let mut rng = Rng::new(seed);
        let cards = build_deck(&config, &mut rng);
        let mut draw_pile: Vec<CardId> = (0..cards.len() as u16).collect();
        rng.shuffle(&mut draw_pile);
        Self::init_from_pile(config, seed, rng, cards, draw_pile, true)
    }

    /// 測試專用建構：`draw_order[0]` 為最先被抽出的牌。
    #[cfg(any(test, feature = "testing"))]
    pub fn new_with_deck(
        mut config: RuleConfig,
        draw_order: Vec<CardId>,
    ) -> Result<Self, EngineError> {
        config.validate()?;
        let mut rng = Rng::new(0);
        let cards = build_deck(&config, &mut rng);
        if draw_order.len() != cards.len() {
            return Err(EngineError::InvalidConfig(format!(
                "draw_order len {} != deck len {}",
                draw_order.len(),
                cards.len()
            )));
        }
        let mut seen = vec![false; cards.len()];
        for &cid in &draw_order {
            let idx = usize::from(cid);
            if idx >= cards.len() || seen[idx] {
                return Err(EngineError::InvalidConfig(format!(
                    "invalid or duplicate card id {cid}"
                )));
            }
            seen[idx] = true;
        }
        let draw_pile: Vec<CardId> = draw_order.into_iter().rev().collect();
        Self::init_from_pile(config, 0, rng, cards, draw_pile, false)
    }

    fn init_from_pile(
        config: RuleConfig,
        seed: u64,
        rng: Rng,
        cards: Vec<Card>,
        draw_pile: Vec<CardId>,
        reshuffle_forbidden_start: bool,
    ) -> Result<Self, EngineError> {
        let n = usize::from(config.num_players);
        let mut state = Self {
            config,
            seed,
            rng,
            cards,
            side: Side::Light,
            hands: vec![Vec::new(); n],
            draw_pile,
            discard: Vec::new(),
            current: 0,
            direction: 1,
            active_color: None,
            pending_draw: 0,
            phase: Phase::Play,
            uno_called: vec![false; n],
            uno_window: None,
            drawn_card: None,
            wild4_was_legal: false,
            challenge_offender: None,
            eliminated: vec![false; n],
            elimination_order: Vec::new(),
            scores: vec![0; n],
            round: 1,
            dealer: 0,
            turn: 0,
            rankings: None,
            start_wild_pending: false,
            flip_just_flipped: false,
            initial_events: Vec::new(),
            steps: Vec::new(),
        };
        let events = state.setup_round(reshuffle_forbidden_start);
        state.initial_events = events;
        Ok(state)
    }

    /// 取得第 1 輪開局事件（發牌、翻開起始牌、開始回合）。
    pub fn initial_events(&self) -> &[Event] {
        &self.initial_events
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_over(&self) -> bool {
        self.phase == Phase::GameOver
    }

    pub fn rankings(&self) -> Option<&[Seat]> {
        self.rankings.as_deref()
    }

    /// 目前正等待喊 UNO / 抓 UNO 的玩家座位。
    pub fn uno_window(&self) -> Option<Seat> {
        self.uno_window
    }

    /// 目前進行中的牌面正反面（Classic/No Mercy 固定為 Light）。
    pub fn side(&self) -> Side {
        self.side
    }

    /// 目前出牌方向（`1` 為順時針、`-1` 為逆時針）。
    pub fn direction(&self) -> i8 {
        self.direction
    }

    /// 目前有效顏色。
    pub fn active_color(&self) -> Option<Color> {
        self.active_color
    }

    /// 玩家人數。
    pub fn num_players(&self) -> u8 {
        self.config.num_players
    }

    /// 取得指定座位的手牌 CardId 切片（零配置）。
    pub fn hand_ids(&self, seat: Seat) -> &[CardId] {
        self.hands
            .get(usize::from(seat))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// 依 CardId 取得牌面定義。
    pub fn card_info(&self, id: CardId) -> Option<Card> {
        self.cards.get(usize::from(id)).copied()
    }

    /// 指定座位是否已被淘汰。
    pub fn is_eliminated(&self, seat: Seat) -> bool {
        self.eliminated
            .get(usize::from(seat))
            .copied()
            .unwrap_or(true)
    }

    /// 取得指定座位的隊伍編號（團隊模式）。
    pub fn team_of_seat(&self, seat: Seat) -> Option<u8> {
        self.team_of(seat)
    }

    /// 取得規則設定唯讀參照。
    pub fn config(&self) -> &RuleConfig {
        &self.config
    }

    /// 目前必須做決策的玩家；RoundOver / GameOver 時為 None。
    pub fn decision_player(&self) -> Option<Seat> {
        match self.phase {
            Phase::RoundOver | Phase::GameOver => None,
            Phase::Play
            | Phase::ChooseColor
            | Phase::ChooseTarget
            | Phase::Challenge
            | Phase::DrawnCardDecision => Some(self.current),
        }
    }

    pub(crate) fn card(&self, id: CardId) -> Card {
        self.cards[usize::from(id)]
    }

    pub(crate) fn top_card_id(&self) -> CardId {
        *self
            .discard
            .last()
            .expect("discard pile always has at least one card during play")
    }

    pub fn top_card(&self) -> Card {
        self.card(self.top_card_id())
    }

    /// 從 `from` 依目前方向前進 `steps` 步，跳過已淘汰座位。
    pub(crate) fn next_seat(&self, from: Seat, steps: u8) -> Seat {
        self.next_seat_dir(from, steps, self.direction)
    }

    pub(crate) fn next_seat_dir(&self, from: Seat, steps: u8, dir: i8) -> Seat {
        let n = self.config.num_players as i16;
        let mut cur = i16::from(from);
        let step_dir = i16::from(dir);
        for _ in 0..steps {
            loop {
                cur = (cur + step_dir).rem_euclid(n);
                if !self.eliminated[cur as usize] {
                    break;
                }
                if cur == i16::from(from) {
                    break;
                }
            }
        }
        cur as Seat
    }

    pub(crate) fn active_player_count(&self) -> usize {
        self.eliminated.iter().filter(|&&e| !e).count()
    }

    /// 還可從牌堆或重洗棄牌堆抽出的張數。
    pub fn available_draw_count(&self) -> usize {
        self.draw_pile.len() + self.discard.len().saturating_sub(1)
    }

    /// 依 R2.5.5 與 R2.5.7 逐張抽牌；會把重洗與抽牌事件加入 `events`，並回傳實際抽到的 CardId。
    pub(crate) fn draw_cards(
        &mut self,
        seat: Seat,
        count: u8,
        reason: DrawReason,
        events: &mut Vec<Event>,
    ) -> Vec<CardId> {
        let mut drawn_ids = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            if self.draw_pile.is_empty() && self.discard.len() > 1 {
                let top = self.discard.pop().expect("discard non-empty");
                self.draw_pile.append(&mut self.discard);
                self.discard.push(top);
                self.rng.shuffle(&mut self.draw_pile);
                events.push(Event::DeckReshuffled {
                    size: self.draw_pile.len() as u16,
                });
            }
            let Some(cid) = self.draw_pile.pop() else {
                break;
            };
            self.hands[usize::from(seat)].push(cid);
            drawn_ids.push(cid);
        }
        if self.hands[usize::from(seat)].len() >= 2 {
            self.uno_called[usize::from(seat)] = false;
        }
        let drawn_cards: Vec<Card> = drawn_ids.iter().map(|&id| self.card(id)).collect();
        events.push(Event::CardsDrawn {
            seat,
            count: drawn_ids.len() as u8,
            cards: Some(drawn_cards),
            reason,
        });
        drawn_ids
    }

    pub(crate) fn setup_round(&mut self, reshuffle_forbidden_start: bool) -> Vec<Event> {
        let n = usize::from(self.config.num_players);
        for h in &mut self.hands {
            h.clear();
        }
        self.discard.clear();
        self.side = Side::Light;
        self.direction = 1;
        self.active_color = None;
        self.pending_draw = 0;
        self.phase = Phase::Play;
        self.uno_called.fill(false);
        self.uno_window = None;
        self.drawn_card = None;
        self.wild4_was_legal = false;
        self.challenge_offender = None;
        self.start_wild_pending = false;
        self.flip_just_flipped = false;
        self.turn = 0;

        // 計算發牌順序：從莊家下一位開始，依座位遞增跳過淘汰者（R2.2.3）
        let active_cnt = self.active_player_count();
        let mut deal_order = Vec::with_capacity(active_cnt);
        let mut s = self.next_seat_dir(self.dealer, 1, 1);
        for _ in 0..active_cnt {
            deal_order.push(s);
            s = self.next_seat_dir(s, 1, 1);
        }
        for _ in 0..self.config.hand_size {
            for &seat in &deal_order {
                if let Some(cid) = self.draw_pile.pop() {
                    self.hands[usize::from(seat)].push(cid);
                }
            }
        }

        let mut events = Vec::new();
        let hand_sizes: Vec<u8> = (0..n).map(|i| self.hands[i].len() as u8).collect();
        events.push(Event::RoundStarted {
            round: self.round,
            dealer: self.dealer,
            hand_sizes,
        });

        // 翻起始牌（R2.2.4 / R2.2.6 / R4.1.12 / R4.2）
        let is_forbidden_start = |face: Face| {
            matches!(
                face,
                Face::WildDrawFour
                    | Face::WildDrawTwo
                    | Face::WildReverseDrawFour
                    | Face::WildDrawSix
                    | Face::WildDrawTen
                    | Face::WildColorRoulette
            )
        };
        let start_id = loop {
            let cid = self.draw_pile.pop().expect("draw_pile has start card");
            let face = self.card(cid).face(self.side).face;
            let forbidden = is_forbidden_start(face);
            if reshuffle_forbidden_start
                && forbidden
                && self
                    .draw_pile
                    .iter()
                    .any(|&id| !is_forbidden_start(self.card(id).face(self.side).face))
            {
                self.draw_pile.push(cid);
                self.rng.shuffle(&mut self.draw_pile);
                continue;
            }
            break cid;
        };
        self.discard.push(start_id);
        let start_card = self.card(start_id);
        events.push(Event::StartCardFlipped { card: start_card });

        let first = self.next_seat_dir(self.dealer, 1, 1);
        let start_face = start_card.face(self.side);
        self.active_color = start_face.color;

        match start_face.face {
            Face::Skip | Face::SkipEveryone => {
                events.push(Event::PlayerSkipped { seat: first });
                self.current = self.next_seat(first, 1);
                events.push(Event::TurnStarted { seat: self.current });
            }
            Face::Reverse => {
                self.direction = -1;
                events.push(Event::DirectionChanged { direction: -1 });
                self.current = self.dealer;
                events.push(Event::TurnStarted { seat: self.current });
            }
            Face::DrawOne | Face::DrawTwo | Face::DrawFour => {
                let amount = start_face.draw_value().unwrap_or(2) as u8;
                let after = self.next_seat(first, 1);
                self.draw_cards(first, amount, DrawReason::Penalty, &mut events);
                events.push(Event::PlayerSkipped { seat: first });
                if !self.check_mercy_elimination(&mut events) {
                    self.current = if self.eliminated[usize::from(after)] {
                        self.next_seat(first, 1)
                    } else {
                        after
                    };
                    events.push(Event::TurnStarted { seat: self.current });
                }
            }
            Face::Flip => {
                self.side = self.side.flip();
                self.draw_pile.reverse();
                events.push(Event::SideFlipped { side: self.side });
                let flipped_face = start_card.face(self.side);
                self.active_color = flipped_face.color;
                if flipped_face.color.is_some() {
                    self.current = first;
                    events.push(Event::TurnStarted { seat: first });
                } else {
                    // R4.1.12：Flip 起始牌翻過去為萬用牌時由莊家選色，選完後再由 first 開始
                    self.current = self.dealer;
                    self.phase = Phase::ChooseColor;
                    self.flip_just_flipped = true;
                    events.push(Event::TurnStarted { seat: self.dealer });
                }
            }
            Face::Wild
            | Face::WildReverseDrawFour
            | Face::WildDrawSix
            | Face::WildDrawTen
            | Face::WildColorRoulette => {
                self.current = first;
                self.phase = Phase::ChooseColor;
                self.start_wild_pending = true;
                events.push(Event::TurnStarted { seat: first });
            }
            _ => {
                self.current = first;
                events.push(Event::TurnStarted { seat: first });
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::build_deck;

    fn find_card(cards: &[Card], color: Option<Color>, face: Face) -> CardId {
        cards
            .iter()
            .find(|c| c.light.color == color && c.light.face == face)
            .expect("card must exist")
            .id
    }

    fn deck_with_start_card(
        cfg: &RuleConfig,
        start_color: Option<Color>,
        start_face: Face,
    ) -> Vec<CardId> {
        let cards = build_deck(cfg, &mut Rng::new(0));
        let start_id = find_card(&cards, start_color, start_face);
        let dealt = usize::from(cfg.num_players) * usize::from(cfg.hand_size);
        let mut order: Vec<CardId> = cards
            .iter()
            .map(|c| c.id)
            .filter(|&id| id != start_id)
            .collect();
        order.insert(dealt, start_id);
        order
    }

    #[test]
    fn r2_2_3_deals_hand_size_and_conserves_cards() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let state = GameState::new(cfg, 42).unwrap();
        assert_eq!(state.hands.len(), 4);
        for hand in &state.hands {
            assert_eq!(hand.len(), 7);
        }
        assert_eq!(state.available_draw_count(), 108 - 4 * 7 - 1);
        let total: usize = state.hands.iter().map(Vec::len).sum::<usize>()
            + state.draw_pile.len()
            + state.discard.len();
        assert_eq!(total, 108);
    }

    #[test]
    fn r2_2_6_start_card_number() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let order = deck_with_start_card(&cfg, Some(Color::Red), Face::Number(5));
        let state = GameState::new_with_deck(cfg, order).unwrap();
        assert_eq!(state.decision_player(), Some(1));
        assert_eq!(state.active_color, Some(Color::Red));
        assert_eq!(state.phase(), Phase::Play);
    }

    #[test]
    fn r2_2_6_start_card_skip() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let order = deck_with_start_card(&cfg, Some(Color::Blue), Face::Skip);
        let state = GameState::new_with_deck(cfg, order).unwrap();
        assert_eq!(state.decision_player(), Some(2));
        assert!(state
            .initial_events()
            .contains(&Event::PlayerSkipped { seat: 1 }));
    }

    #[test]
    fn r2_2_6_start_card_reverse() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let order = deck_with_start_card(&cfg, Some(Color::Green), Face::Reverse);
        let state = GameState::new_with_deck(cfg, order).unwrap();
        assert_eq!(state.direction, -1);
        assert_eq!(state.decision_player(), Some(0));

        // 2 人局 Reverse 起始：由莊家開始（等同跳過對手）
        let cfg2 = RuleConfig::preset("classic", 2).unwrap();
        let order2 = deck_with_start_card(&cfg2, Some(Color::Green), Face::Reverse);
        let state2 = GameState::new_with_deck(cfg2, order2).unwrap();
        assert_eq!(state2.decision_player(), Some(0));
    }

    #[test]
    fn r2_2_6_start_card_draw_two() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let order = deck_with_start_card(&cfg, Some(Color::Yellow), Face::DrawTwo);
        let state = GameState::new_with_deck(cfg, order).unwrap();
        assert_eq!(state.hands[1].len(), 9);
        assert_eq!(state.decision_player(), Some(2));
    }

    #[test]
    fn r2_2_6_start_card_wild() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let order = deck_with_start_card(&cfg, None, Face::Wild);
        let state = GameState::new_with_deck(cfg, order).unwrap();
        assert_eq!(state.phase(), Phase::ChooseColor);
        assert_eq!(state.decision_player(), Some(1));
        assert!(state.start_wild_pending);
    }

    #[test]
    fn r2_2_6_start_card_wild4_is_reshuffled() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        for seed in 0..50 {
            let state = GameState::new(cfg.clone(), seed).unwrap();
            assert_ne!(state.top_card().light.face, Face::WildDrawFour);
        }
    }
}
