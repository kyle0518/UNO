use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::card::{Card, CardFace, CardId, Color, Seat, Side};
use crate::config::{ModeKind, RuleConfig};
use crate::events::{Action, Event, Phase};
use crate::state::GameState;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PublicPlayer {
    pub seat: Seat,
    pub hand_size: u8,
    pub hand_backs: Option<Vec<CardFace>>,
    pub uno_called: bool,
    pub eliminated: bool,
    pub team: Option<u8>,
    pub score: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlayerView {
    pub me: Option<Seat>,
    pub config: RuleConfig,
    pub phase: Phase,
    pub decision_player: Option<Seat>,
    pub current: Seat,
    pub direction: i8,
    pub side: Side,
    pub top_card: Card,
    pub active_color: Option<Color>,
    pub pending_draw: u16,
    pub draw_pile_size: u16,
    pub draw_pile_top_back: Option<CardFace>,
    pub discard: Vec<Card>,
    pub my_hand: Vec<Card>,
    pub drawn_card: Option<CardId>,
    pub players: Vec<PublicPlayer>,
    pub uno_window: Option<Seat>,
    pub round: u32,
    pub turn: u32,
    pub dealer: Seat,
    pub legal: Vec<Action>,
    pub rankings: Option<Vec<Seat>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GodView {
    pub config: RuleConfig,
    pub side: Side,
    pub hands: Vec<Vec<Card>>,
    pub draw_pile: Vec<Card>,
    pub discard: Vec<Card>,
    pub current: Seat,
    pub direction: i8,
    pub active_color: Option<Color>,
    pub pending_draw: u16,
    pub phase: Phase,
    pub uno_called: Vec<bool>,
    pub uno_window: Option<Seat>,
    pub drawn_card: Option<Card>,
    pub wild4_was_legal: bool,
    pub challenge_offender: Option<Seat>,
    pub eliminated: Vec<bool>,
    pub elimination_order: Vec<Seat>,
    pub scores: Vec<i32>,
    pub round: u32,
    pub dealer: Seat,
    pub turn: u32,
    pub rankings: Option<Vec<Seat>>,
    pub start_wild_pending: bool,
    pub flip_just_flipped: bool,
}

pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// 依觀看者座位過濾事件中的隱藏資訊（`CardsDrawn.cards` 僅保留給抽牌者本人）。
pub fn filter_events(events: &[Event], viewer: Option<Seat>) -> Vec<Event> {
    events
        .iter()
        .map(|ev| match ev {
            Event::CardsDrawn {
                seat,
                count,
                cards,
                reason,
            } => {
                let keep = viewer == Some(*seat);
                Event::CardsDrawn {
                    seat: *seat,
                    count: *count,
                    cards: if keep { cards.clone() } else { None },
                    reason: *reason,
                }
            }
            other => other.clone(),
        })
        .collect()
}

impl GameState {
    /// 建構指定座位的玩家視圖，不洩漏任何他家正面手牌、牌堆順序或他人剛抽出的牌。
    pub fn view(&self, seat: Seat) -> PlayerView {
        self.build_view(Some(seat))
    }

    /// 建構觀戰者視圖（無手牌、無合法動作）。
    pub fn spectator_view(&self) -> PlayerView {
        self.build_view(None)
    }

    fn build_view(&self, viewer: Option<Seat>) -> PlayerView {
        let n = usize::from(self.config.num_players);
        let valid_viewer = viewer.filter(|&s| usize::from(s) < n);
        let is_flip = self.config.mode == ModeKind::Flip;
        let back_side = self.side.flip();

        let players = (0..n)
            .map(|i| {
                let s = i as Seat;
                let hand_backs = if is_flip && valid_viewer != Some(s) {
                    Some(
                        self.hands[i]
                            .iter()
                            .map(|&cid| self.card(cid).face(back_side))
                            .collect(),
                    )
                } else {
                    None
                };
                PublicPlayer {
                    seat: s,
                    hand_size: self.hands[i].len() as u8,
                    hand_backs,
                    uno_called: self.uno_called[i],
                    eliminated: self.eliminated[i],
                    team: self.team_of(s),
                    score: self.scores[i],
                }
            })
            .collect();

        let my_hand = match valid_viewer {
            Some(s) => self.hands[usize::from(s)]
                .iter()
                .map(|&cid| self.card(cid))
                .collect(),
            None => Vec::new(),
        };
        let legal = match valid_viewer {
            Some(s) => self.legal_actions(s),
            None => Vec::new(),
        };

        let decision = self.decision_player();
        let drawn_card = match (valid_viewer, self.phase, self.drawn_card) {
            (Some(s), Phase::DrawnCardDecision, Some(cid)) if decision == Some(s) => Some(cid),
            _ => None,
        };

        let draw_pile_top_back = if is_flip {
            self.draw_pile
                .last()
                .map(|&cid| self.card(cid).face(back_side))
        } else {
            None
        };

        PlayerView {
            me: valid_viewer,
            config: self.config.clone(),
            phase: self.phase,
            decision_player: decision,
            current: self.current,
            direction: self.direction,
            side: self.side,
            top_card: self.top_card(),
            active_color: self.active_color,
            pending_draw: self.pending_draw,
            draw_pile_size: self.draw_pile.len() as u16,
            draw_pile_top_back,
            discard: self.discard.iter().map(|&cid| self.card(cid)).collect(),
            my_hand,
            drawn_card,
            players,
            uno_window: self.uno_window,
            round: self.round,
            turn: self.turn,
            dealer: self.dealer,
            legal,
            rankings: self.rankings.clone(),
        }
    }

    /// 建構完整上帝視角（僅供重播、除錯與狀態雜湊使用）。
    pub fn god_view(&self) -> GodView {
        GodView {
            config: self.config.clone(),
            side: self.side,
            hands: self
                .hands
                .iter()
                .map(|h| h.iter().map(|&cid| self.card(cid)).collect())
                .collect(),
            draw_pile: self.draw_pile.iter().map(|&cid| self.card(cid)).collect(),
            discard: self.discard.iter().map(|&cid| self.card(cid)).collect(),
            current: self.current,
            direction: self.direction,
            active_color: self.active_color,
            pending_draw: self.pending_draw,
            phase: self.phase,
            uno_called: self.uno_called.clone(),
            uno_window: self.uno_window,
            drawn_card: self.drawn_card.map(|cid| self.card(cid)),
            wild4_was_legal: self.wild4_was_legal,
            challenge_offender: self.challenge_offender,
            eliminated: self.eliminated.clone(),
            elimination_order: self.elimination_order.clone(),
            scores: self.scores.clone(),
            round: self.round,
            dealer: self.dealer,
            turn: self.turn,
            rankings: self.rankings.clone(),
            start_wild_pending: self.start_wild_pending,
            flip_just_flipped: self.flip_just_flipped,
        }
    }

    /// 對 GodView JSON 計算決定性 FNV-1a 64-bit 雜湊。
    pub fn state_hash(&self) -> u64 {
        let bytes =
            serde_json::to_vec(&self.god_view()).expect("GodView serialization is infallible");
        fnv1a64(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Color, Face};
    use crate::config::RuleConfig;
    use crate::events::DrawReason;

    #[test]
    fn player_view_and_spectator_view_hide_opponent_and_deck_cards() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let state = GameState::new(cfg, 123).unwrap();

        let v0 = state.view(0);
        assert_eq!(v0.me, Some(0));
        assert_eq!(v0.my_hand.len(), 7);
        assert_eq!(v0.players.len(), 4);
        assert_eq!(v0.players[1].hand_size, 7);
        assert!(v0.players[1].hand_backs.is_none());
        assert!(v0.draw_pile_top_back.is_none());

        let v0_json = serde_json::to_string(&v0).unwrap();
        for &opp_cid in &state.hands[1] {
            let cid_in_v0 = v0.my_hand.iter().any(|c| c.id == opp_cid) || v0.top_card.id == opp_cid;
            assert!(!cid_in_v0);
            let card_json = serde_json::to_string(&state.card(opp_cid)).unwrap();
            assert!(!v0_json.contains(&card_json));
        }

        let spec = state.spectator_view();
        assert_eq!(spec.me, None);
        assert!(spec.my_hand.is_empty());
        assert!(spec.legal.is_empty());
        assert!(spec.drawn_card.is_none());

        // 若進入 DrawnCardDecision，僅當前決策者看得到 drawn_card
        let mut s = state;
        for _ in 0..40 {
            let Some(p) = s.decision_player() else {
                break;
            };
            if s.phase() == Phase::Play && s.legal_actions(p).contains(&Action::Draw) {
                s.apply(p, Action::Draw).unwrap();
                if s.phase() == Phase::DrawnCardDecision {
                    assert!(s.view(p).drawn_card.is_some());
                    let other = (p + 1) % 4;
                    assert!(s.view(other).drawn_card.is_none());
                    assert!(s.spectator_view().drawn_card.is_none());
                    break;
                }
            } else {
                let act = s.legal_actions(p)[0];
                s.apply(p, act).unwrap();
            }
        }
    }

    #[test]
    fn filter_events_redacts_cards_drawn_only_for_non_drawer() {
        let sample_card = Card {
            id: 7,
            light: crate::card::CardFace {
                color: Some(Color::Red),
                face: Face::Number(7),
            },
            dark: None,
        };
        let events = vec![
            Event::TurnStarted { seat: 1 },
            Event::CardsDrawn {
                seat: 1,
                count: 1,
                cards: Some(vec![sample_card]),
                reason: DrawReason::Voluntary,
            },
        ];

        let f1 = filter_events(&events, Some(1));
        assert_eq!(f1, events);

        let f0 = filter_events(&events, Some(0));
        assert_eq!(
            f0[1],
            Event::CardsDrawn {
                seat: 1,
                count: 1,
                cards: None,
                reason: DrawReason::Voluntary,
            }
        );

        let f_spec = filter_events(&events, None);
        assert_eq!(f_spec, f0);
    }

    #[test]
    fn state_hash_matches_identical_games_and_differs_on_different_seeds() {
        let cfg = RuleConfig::preset("classic", 4).unwrap();
        let g1 = GameState::new(cfg.clone(), 999).unwrap();
        let g2 = GameState::new(cfg.clone(), 999).unwrap();
        let g3 = GameState::new(cfg, 1000).unwrap();

        assert_eq!(g1.state_hash(), g2.state_hash());
        assert_ne!(g1.state_hash(), g3.state_hash());
    }
}
