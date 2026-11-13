use std::cmp::Reverse;

use crate::card::{points, CardId, Seat};
use crate::config::Scoring;
use crate::deck::build_deck;
use crate::events::{EliminationReason, EngineError, Event, Phase};
use crate::replay::{ReplayAction, ReplayStep};
use crate::state::GameState;

impl GameState {
    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn dealer(&self) -> Seat {
        self.dealer
    }

    pub fn scores(&self) -> &[i32] {
        &self.scores
    }

    pub(crate) fn compute_hand_points(&self) -> Vec<u32> {
        let n = usize::from(self.config.num_players);
        (0..n)
            .map(|s| {
                self.hands[s]
                    .iter()
                    .map(|&cid| {
                        let face = self.card(cid).face(self.side);
                        points(&face, self.config.mode, self.side)
                    })
                    .sum()
            })
            .collect()
    }

    pub(crate) fn compute_rankings(&self, winner: Option<Seat>, hand_points: &[u32]) -> Vec<Seat> {
        let n = usize::from(self.config.num_players);
        let mut active: Vec<Seat> = (0..n as Seat)
            .filter(|&s| !self.eliminated[usize::from(s)])
            .collect();
        active.sort_by_key(|&s| {
            let idx = usize::from(s);
            let win_rank = match winner {
                Some(w) if self.config.scoring == Scoring::SingleRound => {
                    if s == w || self.same_team(w, s) {
                        0u8
                    } else {
                        1u8
                    }
                }
                _ => 0u8,
            };
            (
                win_rank,
                Reverse(self.scores[idx]),
                hand_points[idx],
                self.hands[idx].len(),
                s,
            )
        });

        // 已淘汰玩家依淘汰順序反序排在未淘汰玩家之後（愈晚淘汰名次愈高）
        for &elim_seat in self.elimination_order.iter().rev() {
            if !active.contains(&elim_seat) {
                active.push(elim_seat);
            }
        }
        active
    }

    /// 結束本輪並依規則計分與判斷遊戲是否結束（RULES §2.9 / §4.3 / §4.4）。
    pub(crate) fn finish_round(&mut self, winner: Option<Seat>, events: &mut Vec<Event>) {
        let n = usize::from(self.config.num_players);
        let hand_points = self.compute_hand_points();
        let mut round_scores = vec![0i32; n];

        if !self.config.elimination {
            if let Some(w) = winner {
                if self.config.teams.is_some() {
                    let opp_sum: i32 = (0..n)
                        .filter(|&s| !self.same_team(w, s as Seat))
                        .map(|s| hand_points[s] as i32)
                        .sum();
                    for (s, rs) in round_scores.iter_mut().enumerate() {
                        if self.same_team(w, s as Seat) {
                            *rs = opp_sum;
                            self.scores[s] += opp_sum;
                        }
                    }
                } else {
                    let total_pts: i32 = (0..n)
                        .filter(|&s| s != usize::from(w))
                        .map(|s| hand_points[s] as i32)
                        .sum();
                    let wi = usize::from(w);
                    round_scores[wi] = total_pts;
                    self.scores[wi] += total_pts;
                }
            }
        }

        events.push(Event::RoundEnded {
            winner,
            scores: round_scores,
            hand_points: hand_points.clone(),
            totals: self.scores.clone(),
        });

        if self.config.elimination && self.active_player_count() > 1 {
            if let Some(loser) = (0..n as Seat)
                .filter(|&s| !self.eliminated[usize::from(s)])
                .max_by_key(|&s| {
                    let idx = usize::from(s);
                    (hand_points[idx], self.hands[idx].len(), s)
                })
            {
                let li = usize::from(loser);
                self.eliminated[li] = true;
                self.elimination_order.push(loser);
                self.uno_called[li] = false;
                if self.uno_window == Some(loser) {
                    self.uno_window = None;
                }
                events.push(Event::PlayerEliminated {
                    seat: loser,
                    reason: EliminationReason::Elimination,
                });
            }
        }

        let game_finished = if self.config.elimination {
            self.active_player_count() <= 1
        } else {
            match self.config.scoring {
                Scoring::SingleRound => true,
                Scoring::Points { target } => {
                    self.scores.iter().any(|&total| total >= target as i32)
                }
            }
        };

        self.uno_window = None;
        self.drawn_card = None;
        self.challenge_offender = None;
        self.wild4_was_legal = false;
        self.pending_draw = 0;

        if game_finished {
            let rankings = self.compute_rankings(winner, &hand_points);
            self.rankings = Some(rankings.clone());
            self.phase = Phase::GameOver;
            events.push(Event::GameEnded { rankings });
        } else {
            self.phase = Phase::RoundOver;
        }
    }

    /// 遊戲限時到期（R2.10.2）：立即依當前積分與手牌點數排名並結束遊戲。
    pub(crate) fn finish_by_time(&mut self, events: &mut Vec<Event>) {
        let hand_points = self.compute_hand_points();
        let rankings = self.compute_rankings(None, &hand_points);
        self.uno_window = None;
        self.drawn_card = None;
        self.challenge_offender = None;
        self.wild4_was_legal = false;
        self.pending_draw = 0;
        self.rankings = Some(rankings.clone());
        self.phase = Phase::GameOver;
        events.push(Event::GameEnded { rankings });
    }

    /// 從 `Phase::RoundOver` 進入下一輪（RULES §2.9.2）：輪莊並沿用既有決定性 RNG 重洗牌組。
    pub fn next_round(&mut self) -> Result<Vec<Event>, EngineError> {
        match self.phase {
            Phase::RoundOver => {}
            Phase::GameOver => return Err(EngineError::GameOver),
            _ => return Err(EngineError::IllegalAction),
        }

        let cards = build_deck(&self.config, &mut self.rng);
        let mut draw_pile: Vec<CardId> = (0..cards.len() as u16).collect();
        self.rng.shuffle(&mut draw_pile);

        self.cards = cards;
        self.draw_pile = draw_pile;
        self.dealer = self.next_seat_dir(self.dealer, 1, 1);
        self.round += 1;

        let events = self.setup_round(true);
        self.steps.push(ReplayStep {
            seat: None,
            action: ReplayAction::next_round(),
            t: 0,
        });
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, Color, Face};
    use crate::config::RuleConfig;
    use crate::deck::build_deck;
    use crate::events::Action;
    use crate::rng::Rng;

    fn pick_card(cards: &[Card], used: &mut [bool], color: Option<Color>, face: Face) -> CardId {
        let found = cards
            .iter()
            .find(|c| !used[usize::from(c.id)] && c.light.color == color && c.light.face == face)
            .expect("requested card must exist in unused cards")
            .id;
        used[usize::from(found)] = true;
        found
    }

    fn custom_round_deck(
        cfg: &RuleConfig,
        hands_spec: &[Vec<(Option<Color>, Face)>],
        start_spec: (Option<Color>, Face),
    ) -> Vec<CardId> {
        let cards = build_deck(cfg, &mut Rng::new(0));
        let mut used = vec![false; cards.len()];

        let mut picked_hands: Vec<Vec<CardId>> = Vec::with_capacity(hands_spec.len());
        for spec in hands_spec {
            let mut h = Vec::new();
            for &(c, f) in spec {
                h.push(pick_card(&cards, &mut used, c, f));
            }
            picked_hands.push(h);
        }
        let start_id = pick_card(&cards, &mut used, start_spec.0, start_spec.1);

        let mut remaining: Vec<CardId> = cards
            .iter()
            .map(|c| c.id)
            .filter(|&id| !used[usize::from(id)])
            .collect();

        let n = usize::from(cfg.num_players);
        let h_size = usize::from(cfg.hand_size);
        let mut order = Vec::with_capacity(cards.len());
        for round_idx in 0..h_size {
            for offset in 0..n {
                let seat = (1 + offset) % n;
                if let Some(id) = picked_hands
                    .get(seat)
                    .and_then(|v| v.get(round_idx))
                    .copied()
                {
                    order.push(id);
                } else {
                    order.push(remaining.pop().unwrap());
                }
            }
        }
        order.push(start_id);
        order.extend(remaining);
        order
    }

    #[test]
    fn r2_9_hand_calculated_scoring_and_rankings_tie_breaks() {
        let mut cfg = RuleConfig::preset("classic", 4).unwrap();
        cfg.hand_size = 3;
        cfg.scoring = Scoring::SingleRound;

        // seat 0: Red 0 (0) + Yellow 9 (9) + Blue Skip (20) = 29
        // seat 1: Red 1 (1) -> wins? Give seat 1 Red 1 and let seat 1 win only if hand_size=1?
        // Using hand_size=3: seat 1 holds Red 1, Blue 1, Green 1 -> let seat 1 win via 3 turns!
        let order = custom_round_deck(
            &cfg,
            &[
                vec![
                    (Some(Color::Red), Face::Number(0)),
                    (Some(Color::Yellow), Face::Number(9)),
                    (Some(Color::Blue), Face::Skip),
                ],
                vec![
                    (Some(Color::Red), Face::Number(2)),
                    (Some(Color::Red), Face::Number(3)),
                    (Some(Color::Red), Face::Number(4)),
                ],
                vec![
                    (None, Face::Wild),
                    (None, Face::WildDrawFour),
                    (Some(Color::Green), Face::Reverse),
                ],
                vec![
                    (Some(Color::Green), Face::DrawTwo),
                    (Some(Color::Red), Face::Number(5)),
                    (Some(Color::Yellow), Face::Number(0)),
                ],
            ],
            (Some(Color::Red), Face::Number(1)),
        );
        let mut s = GameState::new_with_deck(cfg, order).unwrap();
        // seat 1 連出三張 Red，其他玩家用 Draw 過回合？更簡單：驗證單張立即獲勝 + 同分 tie-break
        drop(s);

        // 直接以 hand_size = 1 與 hand_size = 2 精確測試計分與平手順序
        let mut cfg1 = RuleConfig::preset("classic", 4).unwrap();
        cfg1.hand_size = 2;
        cfg1.scoring = Scoring::SingleRound;
        // seat 0: Yellow 9 (9) + Wild (50) = 59
        // seat 1: Red 1 (1), Red 2 (2) -> 第 1 輪打 Red 1，剩 1 張；第 2 輪打 Red 2 清空獲勝
        // seat 2: WildDrawFour (50) + Green Skip (20) = 70
        // seat 3: Blue 5 (5) + Green 5 (5) = 10 -> 輸家中手牌最少應排第 2 名！
        let order1 = custom_round_deck(
            &cfg1,
            &[
                vec![(Some(Color::Yellow), Face::Number(9)), (None, Face::Wild)],
                vec![
                    (Some(Color::Red), Face::Number(1)),
                    (Some(Color::Red), Face::Number(2)),
                ],
                vec![(None, Face::WildDrawFour), (Some(Color::Green), Face::Skip)],
                vec![
                    (Some(Color::Blue), Face::Number(5)),
                    (Some(Color::Green), Face::Number(5)),
                ],
            ],
            (Some(Color::Red), Face::Number(8)),
        );
        s = GameState::new_with_deck(cfg1, order1).unwrap();
        let c1_a = s.hands[1][0];
        s.apply(1, Action::Play { card: c1_a }).unwrap(); // seat 1: 剩 1 張
        let ev = s.apply(0, Action::EndByTime).unwrap(); // 先不測 EndByTime，繼續走完一輪：
        drop(ev);

        // 重建讓 2,3,0 都做決策直到輪回 seat 1 出完最後一張
        let mut cfg_single = RuleConfig::preset("classic", 4).unwrap();
        cfg_single.hand_size = 1;
        cfg_single.scoring = Scoring::SingleRound;
        let order_s = custom_round_deck(
            &cfg_single,
            &[
                vec![(Some(Color::Blue), Face::Skip)],       // 20
                vec![(Some(Color::Red), Face::Number(2))],   // 贏家 -> 0
                vec![(None, Face::WildDrawFour)],            // 50
                vec![(Some(Color::Green), Face::Number(5))], // 5
            ],
            (Some(Color::Red), Face::Number(9)),
        );
        let mut sg = GameState::new_with_deck(cfg_single, order_s).unwrap();
        let win_card = sg.hands[1][0];
        let events = sg.apply(1, Action::Play { card: win_card }).unwrap();
        assert_eq!(sg.phase(), Phase::GameOver);
        assert!(events.contains(&Event::RoundEnded {
            winner: Some(1),
            scores: vec![0, 75, 0, 0],
            hand_points: vec![20, 0, 50, 5],
            totals: vec![0, 75, 0, 0],
        }));
        // 排名：seat 1 (75分) -> seat 3 (0分,5點) -> seat 0 (0分,20點) -> seat 2 (0分,50點)
        assert_eq!(sg.rankings(), Some(&[1, 3, 0, 2][..]));
    }

    #[test]
    fn r4_3_team_scoring_excludes_teammate_hand_and_awards_both() {
        let mut cfg = RuleConfig::preset("classic", 4).unwrap();
        cfg.teams = Some(2); // Team 0: seats 0,2; Team 1: seats 1,3
        cfg.hand_size = 1;
        cfg.scoring = Scoring::SingleRound;
        let order = custom_round_deck(
            &cfg,
            &[
                vec![(Some(Color::Blue), Face::Skip)],     // 20 (敵隊)
                vec![(Some(Color::Red), Face::Number(3))], // 贏家（Team 1）
                vec![(None, Face::Wild)],                  // 50 (敵隊)
                vec![(None, Face::WildDrawFour)],          // 50 (同隊隊友，不計入得分！)
            ],
            (Some(Color::Red), Face::Number(8)),
        );
        let mut s = GameState::new_with_deck(cfg, order).unwrap();
        let card = s.hands[1][0];
        let ev = s.apply(1, Action::Play { card }).unwrap();
        // 敵隊總分 = 20 + 50 = 70；Team 1 (seat 1 & seat 3) 各得 70
        assert!(ev.contains(&Event::RoundEnded {
            winner: Some(1),
            scores: vec![0, 70, 0, 70],
            hand_points: vec![20, 0, 50, 50],
            totals: vec![0, 70, 0, 70],
        }));
        assert_eq!(s.rankings(), Some(&[1, 3, 0, 2][..]));
    }

    #[test]
    fn r2_9_multi_round_points_accumulates_and_rotates_dealer() {
        let mut cfg = RuleConfig::preset("classic", 4).unwrap();
        cfg.hand_size = 1;
        cfg.scoring = Scoring::Points { target: 60 };

        let order = custom_round_deck(
            &cfg,
            &[
                vec![(Some(Color::Blue), Face::Number(9))],   // 9
                vec![(Some(Color::Red), Face::Number(1))],    // wins R1 -> 24 pts (< 60)
                vec![(Some(Color::Green), Face::Number(8))],  // 8
                vec![(Some(Color::Yellow), Face::Number(7))], // 7
            ],
            (Some(Color::Red), Face::Number(5)),
        );
        let mut s = GameState::new_with_deck(cfg, order).unwrap();
        // 非 RoundOver 呼叫 next_round 回傳 IllegalAction
        assert_eq!(s.next_round(), Err(EngineError::IllegalAction));

        let c1 = s.hands[1][0];
        s.apply(1, Action::Play { card: c1 }).unwrap();
        assert_eq!(s.phase(), Phase::RoundOver);
        assert_eq!(s.scores(), &[0, 24, 0, 0]);
        assert_eq!(s.rankings(), None);

        // 進入第 2 輪：輪莊到 seat 1
        let ev2 = s.next_round().unwrap();
        assert_eq!(s.round(), 2);
        assert_eq!(s.dealer(), 1);
        assert!(ev2.iter().any(|e| matches!(
            e,
            Event::RoundStarted {
                round: 2,
                dealer: 1,
                ..
            }
        )));
        assert_eq!(s.scores(), &[0, 24, 0, 0]);

        // 再透過結束條件觸發超過 target=60 -> GameOver
        // (使用 EndByTime 或直接模擬直到 GameOver 也行，此處測試 GameOver 後 next_round 回傳 Err(GameOver))
        s.apply(0, Action::EndByTime).unwrap();
        assert_eq!(s.next_round(), Err(EngineError::GameOver));
    }

    #[test]
    fn r2_9_tie_break_on_equal_scores_and_hand_points() {
        let mut cfg = RuleConfig::preset("classic", 4).unwrap();
        cfg.hand_size = 1;
        cfg.scoring = Scoring::SingleRound;
        let order = custom_round_deck(
            &cfg,
            &[
                vec![(Some(Color::Blue), Face::Number(5))],   // 5 pts
                vec![(Some(Color::Red), Face::Number(1))],    // winner
                vec![(Some(Color::Green), Face::Number(5))], // 5 pts (tie with seat 0 -> seat 0 wins tie)
                vec![(Some(Color::Yellow), Face::Number(9))], // 9 pts
            ],
            (Some(Color::Red), Face::Number(4)),
        );
        let mut s = GameState::new_with_deck(cfg, order).unwrap();
        let c1 = s.hands[1][0];
        s.apply(1, Action::Play { card: c1 }).unwrap();
        assert_eq!(s.rankings(), Some(&[1, 0, 2, 3][..]));
    }
}

#[cfg(test)]
#[path = "teams_elimination_tests.rs"]
mod teams_elimination_tests;
