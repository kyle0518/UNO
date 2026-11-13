use uno_core::{
    Action, CardFace, Event, Face, GameState, ModeKind, Phase, PlayerView, Rng, RuleConfig,
    Scoring, Seat, Side,
};

use crate::heuristic::choose_heuristic_action;
use crate::tactical::{
    best_catch_uno, choose_tactical_with_ctx, compute_action_priors, StateCtx, ViewCtx,
};
use crate::Bot;

/// 桌面環境預設模擬次數。
pub const DEFAULT_SIMS: usize = 400;
/// 手機單機環境建議模擬次數。
pub const MOBILE_SIMS: usize = 200;

// 輕量先驗穩定項（僅作為同級別統計誤差的微弱調節，不壓過實際模擬勝率）
const ROOT_PRIOR_PSEUDOCOUNT: f64 = 4.0;
const ROOT_PRIOR_BASE: f64 = 0.22;
const ROOT_PRIOR_SPAN: f64 = 0.06;
// 前瞻深度 44 步（約 11 輪桌面週期）：覆蓋多數終局同時維持高模擬吞吐
const MAX_ROLLOUT_STEPS: usize = 44;

/// 使用 Information Set Monte Carlo Tree Search（扁平根節點 + UCB1）的困難 Bot。
#[derive(Debug, Clone)]
pub struct HardBot {
    rng: Rng,
    sims: usize,
}

impl HardBot {
    /// 以指定亂數種子與每次決策的模擬次數建立 `HardBot`。
    #[must_use]
    pub fn new(seed: u64, sims: usize) -> Self {
        Self {
            rng: Rng::new(seed),
            sims,
        }
    }

    /// 取得目前設定的模擬次數。
    #[must_use]
    pub fn sims(&self) -> usize {
        self.sims
    }
}

impl Bot for HardBot {
    fn name(&self) -> &'static str {
        "hard"
    }

    fn act(&mut self, state_view: &PlayerView) -> Action {
        if state_view.legal.is_empty() {
            return Action::Draw;
        }
        if state_view.legal.len() == 1 {
            return state_view.legal[0];
        }
        if state_view.legal.contains(&Action::CallUno) {
            return Action::CallUno;
        }
        let me_fallback = state_view.decision_player.or(state_view.me).unwrap_or(0);
        let root_ctx = ViewCtx(state_view, me_fallback);
        if let Some(catch) = best_catch_uno(&root_ctx, &state_view.legal) {
            return catch;
        }
        if state_view.phase == Phase::Challenge {
            return choose_challenge_from_view(state_view);
        }
        if self.sims == 0 {
            return choose_tactical_with_ctx(&root_ctx, &state_view.legal);
        }

        let Some(me) = state_view.me.or(state_view.decision_player) else {
            return choose_heuristic_action(state_view);
        };

        let candidates = build_root_candidates(state_view);
        if candidates.len() == 1 {
            return candidates[0];
        }

        let Ok(base_world) = GameState::from_view_sampled(state_view, &mut self.rng) else {
            return choose_tactical_with_ctx(&root_ctx, &state_view.legal);
        };

        let priors = compute_action_priors(&root_ctx, &candidates);
        let num_actions = candidates.len();
        let sims = self.sims.max(1);

        let mut visits = vec![0_usize; num_actions];
        let mut total_values = vec![0.0_f64; num_actions];

        let num_players = state_view.players.len();
        let elimination = state_view.config.elimination;
        let has_teams = state_view.config.teams.is_some();
        let points_target = match state_view.config.scoring {
            Scoring::Points { target } => Some(target as i32),
            _ => None,
        };

        // 階段 1：用共同隨機數（CRN）公平比較所有候選動作
        let stage1_budget = if num_actions <= 2 {
            sims
        } else {
            ((sims * 3) / 5).max(num_actions * 4)
        };
        let crn_rounds = (stage1_budget / num_actions).max(1);
        let mut used_sims = 0_usize;
        for _ in 0..crn_rounds {
            if used_sims >= sims {
                break;
            }
            let sampled_base = base_world.sample_world(me, &mut self.rng);
            for (arm, &root_action) in candidates.iter().enumerate() {
                if used_sims >= sims {
                    break;
                }
                let value = simulate_on_world(
                    &sampled_base,
                    me,
                    root_action,
                    num_players,
                    elimination,
                    has_teams,
                    points_target,
                );
                visits[arm] += 1;
                total_values[arm] += value;
                used_sims += 1;
            }
        }

        // 階段 2：保留前幾名強候選動作，持續用配對共同隨機數（CRN）聚焦細分差距
        if used_sims < sims && num_actions > 2 {
            let mut ranked: Vec<usize> = (0..num_actions).collect();
            ranked.sort_by(|&a, &b| {
                let sa = root_bayes_score(visits[a], total_values[a], priors[a]);
                let sb = root_bayes_score(visits[b], total_values[b], priors[b]);
                sb.total_cmp(&sa)
            });
            let focus_count = num_actions.min(3);
            let finalists = &ranked[..focus_count];
            while used_sims < sims {
                let sampled_base = base_world.sample_world(me, &mut self.rng);
                for &arm in finalists {
                    if used_sims >= sims {
                        break;
                    }
                    let value = simulate_on_world(
                        &sampled_base,
                        me,
                        candidates[arm],
                        num_players,
                        elimination,
                        has_teams,
                        points_target,
                    );
                    visits[arm] += 1;
                    total_values[arm] += value;
                    used_sims += 1;
                }
            }
        }

        let mut best_idx = 0_usize;
        let mut best_score = f64::NEG_INFINITY;
        for i in 0..num_actions {
            let score = root_bayes_score(visits[i], total_values[i], priors[i]);
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        candidates[best_idx]
    }
}

fn root_bayes_score(visits: usize, total_value: f64, prior: f64) -> f64 {
    let v = visits as f64;
    let prior_mean = ROOT_PRIOR_BASE + ROOT_PRIOR_SPAN * prior;
    (total_value + ROOT_PRIOR_PSEUDOCOUNT * prior_mean) / (v + ROOT_PRIOR_PSEUDOCOUNT)
}

fn build_root_candidates(view: &PlayerView) -> Vec<Action> {
    let has_play = view
        .legal
        .iter()
        .any(|a| matches!(a, Action::Play { .. } | Action::JumpIn { .. }));

    let mut seen_play_faces: Vec<CardFace> = Vec::new();
    let mut seen_jump_faces: Vec<CardFace> = Vec::new();
    let mut candidates = Vec::with_capacity(view.legal.len());

    for &action in &view.legal {
        match action {
            Action::Draw if has_play => continue,
            Action::Play { card } => {
                if let Some(c) = view.my_hand.iter().find(|c| c.id == card) {
                    let face = c.face(view.side);
                    if view.config.mode != ModeKind::Flip {
                        if seen_play_faces.contains(&face) {
                            continue;
                        }
                        seen_play_faces.push(face);
                    }
                }
                candidates.push(action);
            }
            Action::JumpIn { card } => {
                if let Some(c) = view.my_hand.iter().find(|c| c.id == card) {
                    let face = c.face(view.side);
                    if view.config.mode != ModeKind::Flip {
                        if seen_jump_faces.contains(&face) {
                            continue;
                        }
                        seen_jump_faces.push(face);
                    }
                }
                candidates.push(action);
            }
            _ => candidates.push(action),
        }
    }

    if candidates.is_empty() {
        view.legal.clone()
    } else {
        candidates
    }
}

fn choose_challenge_from_view(view: &PlayerView) -> Action {
    let me = view.decision_player.or(view.me).unwrap_or(0);
    let my_hand_len = view.my_hand.len();
    let n = view.players.len();
    if n > 1 && view.discard.len() >= 2 && view.legal.contains(&Action::Challenge) {
        let step = if view.direction >= 0 { -1_i32 } else { 1_i32 };
        let offender = {
            let mut idx = i32::from(me);
            let mut found = None;
            for _ in 0..n {
                idx = (idx + step).rem_euclid(n as i32);
                let s = idx as Seat;
                if s != me && !view.players[usize::from(s)].eliminated {
                    found = Some(&view.players[usize::from(s)]);
                    break;
                }
            }
            found
        };
        if let Some(off) = offender {
            if off.hand_size == 0 {
                return Action::Challenge;
            }
            if let Some(prev_col) = view.discard[..view.discard.len() - 1]
                .iter()
                .rev()
                .find_map(|c| c.face(view.side).color)
            {
                let seen_discard = view.discard[..view.discard.len() - 1]
                    .iter()
                    .filter(|c| c.face(view.side).color == Some(prev_col))
                    .count();
                let seen_hand = view
                    .my_hand
                    .iter()
                    .filter(|c| c.face(view.side).color == Some(prev_col))
                    .count();
                let seen_total = seen_discard + seen_hand;
                let total_color = total_cards_of_color(&view.config);

                // 手牌剩 1-2 張時吃 +4 幾乎必輸，若嫌疑大（或對手 >=3 張且多數同色未現）值得反制質疑
                if my_hand_len <= 2 && off.hand_size >= 3 && seen_total * 10 < total_color * 4 {
                    return Action::Challenge;
                }
                if off.hand_size >= 5 && seen_total * 10 < total_color * 3 {
                    return Action::Challenge;
                }
            }
        }
    }
    if view.legal.contains(&Action::Accept) {
        Action::Accept
    } else {
        choose_heuristic_action(view)
    }
}

fn total_cards_of_color(config: &RuleConfig) -> usize {
    let per_deck = match config.mode {
        ModeKind::Flip => 26_usize,
        ModeKind::NoMercy => 36_usize,
        ModeKind::Classic => 25_usize,
    };
    per_deck * usize::from(config.deck_count.max(1))
}

fn simulate_on_world(
    base_world: &GameState,
    me: Seat,
    root_action: Action,
    num_players: usize,
    elimination: bool,
    has_teams: bool,
    points_target: Option<i32>,
) -> f64 {
    let mut sim_state = base_world.clone();

    let Ok(mut last_events) = sim_state.apply(me, root_action) else {
        return 0.0;
    };

    let mut steps = 0_usize;
    while !matches!(sim_state.phase(), Phase::RoundOver | Phase::GameOver)
        && steps < MAX_ROLLOUT_STEPS
    {
        steps += 1;

        if let Some(u_seat) = sim_state.uno_window() {
            let u_legal = sim_state.legal_actions(u_seat);
            if u_legal.contains(&Action::CallUno) {
                if let Ok(ev) = sim_state.apply(u_seat, Action::CallUno) {
                    last_events = ev;
                    continue;
                }
            }
        }

        let Some(actor) = sim_state.decision_player() else {
            break;
        };

        let legal = sim_state.legal_actions(actor);
        if legal.is_empty() {
            break;
        }

        let action = if legal.len() == 1 {
            legal[0]
        } else if sim_state.phase() == Phase::DrawnCardDecision {
            legal
                .iter()
                .copied()
                .find(|a| matches!(a, Action::Play { .. }))
                .unwrap_or(legal[0])
        } else if sim_state.phase() == Phase::Challenge {
            choose_challenge_in_world(&sim_state, actor, &legal)
        } else {
            let ctx = StateCtx(&sim_state, actor);
            choose_tactical_with_ctx(&ctx, &legal)
        };

        match sim_state.apply(actor, action) {
            Ok(ev) => last_events = ev,
            Err(_) => break,
        }
    }

    evaluate_rollout(
        &sim_state,
        &last_events,
        me,
        num_players,
        elimination,
        has_teams,
        points_target,
    )
}

fn choose_challenge_in_world(sim_state: &GameState, actor: Seat, legal: &[Action]) -> Action {
    let n = usize::from(sim_state.num_players());
    if n > 1 && legal.contains(&Action::Challenge) {
        let step = if sim_state.direction() >= 0 {
            -1_i32
        } else {
            1_i32
        };
        let mut idx = i32::from(actor);
        for _ in 0..n {
            idx = (idx + step).rem_euclid(n as i32);
            let s = idx as Seat;
            if s != actor && !sim_state.is_eliminated(s) {
                let off_len = sim_state.hand_ids(s).len();
                if off_len == 0
                    || (off_len >= 4 && sim_state.hand_ids(actor).len() <= 2)
                    || off_len >= 6
                {
                    return Action::Challenge;
                }
                break;
            }
        }
    }
    if legal.contains(&Action::Accept) {
        Action::Accept
    } else {
        legal[0]
    }
}

fn evaluate_rollout(
    sim_state: &GameState,
    last_events: &[Event],
    me: Seat,
    n: usize,
    elimination: bool,
    has_teams: bool,
    points_target: Option<i32>,
) -> f64 {
    if n <= 1 {
        return 1.0;
    }

    if let Some(Event::RoundEnded {
        winner,
        scores,
        totals,
        ..
    }) = last_events
        .iter()
        .rev()
        .find(|e| matches!(e, Event::RoundEnded { .. }))
    {
        if elimination {
            if let Some(rankings) = sim_state.rankings() {
                return reward_from_rankings(rankings, me, n);
            }
            let alive = (0..n)
                .filter(|&i| !sim_state.is_eliminated(i as Seat))
                .count();
            if sim_state.is_eliminated(me) {
                return 0.0;
            }
            if Some(me) == *winner {
                return 1.0;
            }
            return if alive > 1 { 0.5 } else { 1.0 };
        }

        if let Some(win_seat) = winner {
            if has_teams {
                let my_team = sim_state.team_of_seat(me);
                let win_team = sim_state.team_of_seat(*win_seat);
                return if my_team.is_some() && win_team == my_team {
                    1.0
                } else {
                    0.0
                };
            }
            if *win_seat == me {
                if let Some(target) = points_target {
                    let my_total = totals.get(usize::from(me)).copied().unwrap_or(0);
                    if my_total >= target {
                        return 1.0;
                    }
                    let gained = scores.get(usize::from(me)).copied().unwrap_or(0).max(0) as f64;
                    return 0.84 + 0.12 * (gained / 150.0).min(1.0);
                }
                return 1.0;
            }
            if let Some(target) = points_target {
                let win_total = totals.get(usize::from(*win_seat)).copied().unwrap_or(0);
                if win_total >= target {
                    return 0.0;
                }
                let my_total = totals.get(usize::from(me)).copied().unwrap_or(0);
                let gap = (win_total - my_total).max(0);
                if win_total >= target - 80 {
                    return 0.01;
                }
                return if gap == 0 { 0.04 } else { 0.0 };
            }
            return 0.0;
        }

        let mut order: Vec<(i32, Seat)> = totals
            .iter()
            .enumerate()
            .map(|(idx, &score)| (-score, idx as Seat))
            .collect();
        order.sort_unstable();
        if let Some(rank) = order.iter().position(|&(_, s)| s == me) {
            return 1.0 - (rank as f64) / ((n - 1) as f64);
        }
    }

    if let Some(rankings) = sim_state.rankings() {
        return reward_from_rankings(rankings, me, n);
    }

    evaluate_midgame(sim_state, me, n, has_teams)
}

fn evaluate_midgame(sim_state: &GameState, me: Seat, n: usize, has_teams: bool) -> f64 {
    if sim_state.is_eliminated(me) {
        return 0.0;
    }

    let side = sim_state.side();
    let active_color = sim_state.active_color();
    let my_team = if has_teams {
        sim_state.team_of_seat(me)
    } else {
        None
    };

    let hand_strength = |seat: Seat| -> f64 {
        let ids = sim_state.hand_ids(seat);
        let len = ids.len() as f64;
        if len == 0.0 {
            return 0.0;
        }
        let mut wilds = 0.0_f64;
        let mut disrupt = 0.0_f64;
        let mut matching_active = 0.0_f64;
        let mut color_mask = 0_u8;
        for &cid in ids {
            if let Some(card) = sim_state.card_info(cid) {
                let cf = card.face(side);
                if cf.is_wild() {
                    wilds += 1.0;
                } else if let Some(col) = cf.color {
                    color_mask |= 1_u8 << col.slot();
                    if Some(col) == active_color {
                        matching_active += 1.0;
                    }
                    if matches!(
                        cf.face,
                        Face::DrawTwo
                            | Face::DrawFive
                            | Face::Skip
                            | Face::SkipEveryone
                            | Face::Reverse
                    ) {
                        disrupt += 1.0;
                    }
                }
            }
        }
        let colors = f64::from(color_mask.count_ones());
        let mono_bonus = if colors <= 1.0 { 0.25 } else { 0.0 };
        let active_bonus = if matching_active > 0.0 { 0.15 } else { 0.0 };
        (len - 0.85 * wilds - 0.15 * disrupt - mono_bonus - active_bonus).max(0.20)
    };

    // Softmax 將所有人剩餘等效手牌數轉成總和為 1.0 的勝率（4 人均勢 = 0.25，與 RoundEnded 期望一致）
    let mut team_weight = 0.0_f64;
    let mut total_weight = 0.0_f64;
    let alpha = 0.65_f64;

    for i in 0..n {
        let s = i as Seat;
        if sim_state.is_eliminated(s) {
            continue;
        }
        let eff = hand_strength(s);
        let w = (-alpha * eff).exp();
        total_weight += w;
        if s == me || (has_teams && my_team.is_some() && sim_state.team_of_seat(s) == my_team) {
            team_weight += w;
        }
    }

    if total_weight <= 0.0 {
        return 0.25;
    }
    let _ = Side::Light;
    (team_weight / total_weight).clamp(0.01, 0.96)
}

fn reward_from_rankings(rankings: &[Seat], me: Seat, total_players: usize) -> f64 {
    let denom = total_players.saturating_sub(1).max(1) as f64;
    match rankings.iter().position(|&s| s == me) {
        Some(rank) => 1.0 - (rank as f64) / denom,
        None => 0.0,
    }
}
