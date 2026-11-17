//! 地獄難度 Bot：Single-Observer ISMCTS（資訊集蒙地卡羅樹搜尋）。
//!
//! 與 Hard 的差異（依 ISMCTS 文獻與 2024 TU Wien 對 MO/SO-ISMCTS 的比較結論）：
//! - 維護「單一搜尋樹」而非根節點扁平蒙地卡羅：每個節點跨多次 determinization
//!   累積統計，避免 PIMC 的 strategy fusion（只在單一想像世界成立的路線被高估）。
//! - 每次迭代重新取樣一個 determinized 世界（`sample_world`），沿樹以 UCB1 選擇、
//!   展開一個新節點，再以 tactical rollout 補完並回傳 rank 型報酬。
//! - 對手節點以「最小化我方報酬」選擇（多人競爭近似），隊友節點則最大化。
//! - 葉節點 rollout 沿用 Hard 的自適應深度（殘局打到回合結束）與評估函式。

use uno_core::{Action, CardFace, Color, GameState, Phase, PlayerView, Rng, Seat};

/// 樹節點的抽象動作鍵：出牌以「牌面」而非具體 card id 表示，
/// 使不同 determinization 下同一手牌能共享節點統計（ISMCTS 的動作抽象）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AKey {
    Play(CardFace),
    JumpIn(CardFace),
    ChooseColor(Color),
    ChooseTarget(Seat),
    Other(Action),
}

fn action_key(state: &GameState, action: Action) -> AKey {
    match action {
        Action::Play { card } => state
            .card_info(card)
            .map(|c| AKey::Play(c.face(state.side())))
            .unwrap_or(AKey::Other(action)),
        Action::JumpIn { card } => state
            .card_info(card)
            .map(|c| AKey::JumpIn(c.face(state.side())))
            .unwrap_or(AKey::Other(action)),
        Action::ChooseColor { color } => AKey::ChooseColor(color),
        Action::ChooseTarget { target } => AKey::ChooseTarget(target),
        other => AKey::Other(other),
    }
}

use crate::hard::{
    build_root_candidates, choose_challenge_from_view, evaluate_rollout, rollout_horizon,
    simulate_on_world,
};
use crate::tactical::{
    best_catch_uno, choose_tactical_with_ctx, compute_action_priors, StateCtx, ViewCtx,
};
use crate::Bot;

/// 地獄難度預設迭代次數（遠高於 Hard 的 400 次根模擬）。
pub const HELL_SIMS: usize = 1_500;
const UCB_C: f64 = 0.7;
const MAX_TREE_NODES: usize = 60_000;
const MAX_TREE_DEPTH: usize = 48;

#[derive(Debug, Clone)]
struct Node {
    visits: u32,
    total_value: f64,
    children: Vec<(AKey, usize)>,
}

impl Node {
    fn new() -> Self {
        Self {
            visits: 0,
            total_value: 0.0,
            children: Vec::new(),
        }
    }
}

/// 地獄難度 Bot（SO-ISMCTS）。
#[derive(Debug, Clone)]
pub struct HellBot {
    rng: Rng,
    sims: usize,
}

impl HellBot {
    #[must_use]
    pub fn new(seed: u64, sims: usize) -> Self {
        Self {
            rng: Rng::new(seed),
            sims,
        }
    }

    #[must_use]
    pub fn sims(&self) -> usize {
        self.sims
    }
}

impl Bot for HellBot {
    fn name(&self) -> &'static str {
        "hell"
    }

    fn act(&mut self, view: &PlayerView) -> Action {
        if view.legal.is_empty() {
            return Action::Draw;
        }
        if view.legal.len() == 1 {
            return view.legal[0];
        }
        if view.legal.contains(&Action::CallUno) {
            return Action::CallUno;
        }
        let me = view.me.or(view.decision_player).unwrap_or(0);
        let root_ctx = ViewCtx(view, me);
        if let Some(catch) = best_catch_uno(&root_ctx, &view.legal) {
            return catch;
        }
        if view.phase == Phase::Challenge {
            return choose_challenge_from_view(view);
        }
        if self.sims == 0 {
            return choose_tactical_with_ctx(&root_ctx, &view.legal);
        }
        let candidates = build_root_candidates(view);
        if candidates.len() == 1 {
            return candidates[0];
        }
        let Ok(base_world) = GameState::from_view_sampled(view, &mut self.rng) else {
            return choose_tactical_with_ctx(&root_ctx, &view.legal);
        };

        let n = view.players.len();
        let elimination = view.config.elimination;
        let has_teams = view.config.teams.is_some();
        let points_target = match view.config.scoring {
            uno_core::Scoring::Points { target } => Some(target as i32),
            _ => None,
        };
        let my_team = base_world.team_of_seat(me);

        let mut nodes: Vec<Node> = vec![Node::new()];
        for _ in 0..self.sims {
            let world = base_world.sample_world(me, &mut self.rng);
            ismcts_iteration(
                &mut nodes,
                world,
                me,
                my_team,
                n,
                elimination,
                has_teams,
                points_target,
                &candidates,
            );
        }

        // Ensemble determinization：以共同隨機數扁平 MC 再評估根候選，
        // 與樹統計平均，降低單一搜尋偏差（tree 與 flat 互補）。
        let flat_sims = 600_usize;
        let mut flat_total: std::collections::HashMap<AKey, (f64, usize)> =
            std::collections::HashMap::new();
        if !candidates.is_empty() {
            let rounds = (flat_sims / candidates.len()).max(1);
            for _ in 0..rounds {
                let sampled = base_world.sample_world(me, &mut self.rng);
                for &cand in &candidates {
                    let v = simulate_on_world(
                        &sampled,
                        me,
                        cand,
                        n,
                        elimination,
                        has_teams,
                        points_target,
                    );
                    let k = action_key(&sampled, cand);
                    let e = flat_total.entry(k).or_insert((0.0, 0));
                    e.0 += v;
                    e.1 += 1;
                }
            }
        }

        // 根節點決策：樹平均與扁平 MC 平均各半（抽象鍵映射回具體動作）
        let root = &nodes[0];
        let mut best: Option<(Action, u32, f64)> = None;
        for &(key, child) in &root.children {
            let cn = &nodes[child];
            let tree_mean = if cn.visits > 0 {
                cn.total_value / f64::from(cn.visits)
            } else {
                0.0
            };
            let flat_mean = flat_total
                .get(&key)
                .map(|(sum, cnt)| sum / (*cnt).max(1) as f64)
                .unwrap_or(tree_mean);
            let mean = 0.5 * tree_mean + 0.5 * flat_mean;
            // 在 candidates 中找同抽象鍵的具體動作
            let concrete = candidates.iter().copied().find(|a| {
                let k = match a {
                    Action::Play { card } | Action::JumpIn { card } => {
                        view.my_hand.iter().find(|c| c.id == *card).map(|c| {
                            if matches!(a, Action::Play { .. }) {
                                AKey::Play(c.face(view.side))
                            } else {
                                AKey::JumpIn(c.face(view.side))
                            }
                        })
                    }
                    Action::ChooseColor { color } => Some(AKey::ChooseColor(*color)),
                    Action::ChooseTarget { target } => Some(AKey::ChooseTarget(*target)),
                    other => Some(AKey::Other(*other)),
                };
                k.as_ref() == Some(&key)
            });
            let Some(action) = concrete else { continue };
            let better = match best {
                None => true,
                Some((_, bv, bm)) => {
                    mean > bm + 1e-12 || ((mean - bm).abs() <= 1e-12 && cn.visits > bv)
                }
            };
            if better {
                best = Some((action, cn.visits, mean));
            }
        }
        match best {
            Some((action, _, _)) if view.legal.contains(&action) => action,
            _ => choose_tactical_with_ctx(&root_ctx, &view.legal),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn ismcts_iteration(
    nodes: &mut Vec<Node>,
    mut state: GameState,
    me: Seat,
    my_team: Option<u8>,
    n: usize,
    elimination: bool,
    has_teams: bool,
    points_target: Option<i32>,
    root_candidates: &[Action],
) {
    // path 記錄 (節點, 到達該節點的行動者)；根節點為 None
    let mut path: Vec<(usize, Option<Seat>)> = vec![(0, None)];
    let mut node_idx = 0_usize;
    let mut expanded = false;

    for _depth in 0..MAX_TREE_DEPTH {
        if matches!(state.phase(), Phase::RoundOver | Phase::GameOver) {
            break;
        }
        // UNO 視窗：可 CallUno 者直接補喊（不分支，與 rollout 語義一致）
        if let Some(u) = state.uno_window() {
            if state.legal_actions(u).contains(&Action::CallUno) {
                if state.apply(u, Action::CallUno).is_err() {
                    break;
                }
                continue;
            }
        }
        let Some(actor) = state.decision_player() else {
            break;
        };
        let legal = if node_idx == 0 {
            root_candidates.to_vec()
        } else {
            state.legal_actions(actor)
        };
        if legal.is_empty() {
            break;
        }

        // 以抽象鍵比對：本世界可行（有同鍵合法動作）的既有子節點
        let legal_keys: Vec<AKey> = legal.iter().map(|&a| action_key(&state, a)).collect();
        let available: Vec<(AKey, usize)> = nodes[node_idx]
            .children
            .iter()
            .copied()
            .filter(|(k, _)| legal_keys.contains(k))
            .collect();
        let untried: Vec<Action> = legal
            .iter()
            .copied()
            .filter(|a| {
                let k = action_key(&state, *a);
                !nodes[node_idx].children.iter().any(|(ck, _)| *ck == k)
            })
            .collect();

        if !untried.is_empty() && nodes.len() < MAX_TREE_NODES {
            // 擴展先驗最高的新節點（tactical prior 排序，避免浪費迭代在劣勢分支）
            let ctx = StateCtx(&state, actor);
            let priors = compute_action_priors(&ctx, &untried);
            let mut best_i = 0_usize;
            for (i, &p) in priors.iter().enumerate() {
                if p > priors[best_i] {
                    best_i = i;
                }
            }
            let action = untried[best_i];
            let key = action_key(&state, action);
            if state.apply(actor, action).is_err() {
                break;
            }
            let new_idx = nodes.len();
            nodes.push(Node::new());
            nodes[node_idx].children.push((key, new_idx));
            path.push((new_idx, Some(actor)));
            expanded = true;
            break;
        }

        if available.is_empty() {
            break;
        }
        // UCB 選擇：子節點統計為「當前行動者視角」，一律最大化 + 探索
        let parent_visits = nodes[node_idx].visits.max(1) as f64;
        let mut best_child: Option<(usize, AKey, f64)> = None;
        for (key, child) in &available {
            let cn = &nodes[*child];
            let mean = if cn.visits > 0 {
                cn.total_value / f64::from(cn.visits)
            } else {
                0.0
            };
            let explore = UCB_C * ((parent_visits.ln() + 1.0) / f64::from(cn.visits.max(1))).sqrt();
            let score = mean + explore;
            if best_child.map(|(_, _, s)| score > s).unwrap_or(true) {
                best_child = Some((*child, *key, score));
            }
        }
        let Some((child, key, _)) = best_child else {
            break;
        };
        // 抽象鍵 -> 本世界的具體合法動作
        let Some(action) = legal
            .iter()
            .copied()
            .find(|&a| action_key(&state, a) == key)
        else {
            break;
        };
        if state.apply(actor, action).is_err() {
            break;
        }
        path.push((child, Some(actor)));
        node_idx = child;
    }

    // 葉節點評估：tactical rollout 至自適應深度後以 Hard 的評估函式計分
    let _ = expanded;
    let value = rollout_value(&state, me, n, elimination, has_teams, points_target);

    // 回傳：各節點累積「到達該節點之行動者」視角的報酬（隊友同我方、對手取 1-v 近似）
    for &(idx, actor_opt) in &path {
        nodes[idx].visits += 1;
        let node_value = match actor_opt {
            None => value,
            Some(actor) => {
                let ally = actor == me
                    || (has_teams && my_team.is_some() && state.team_of_seat(actor) == my_team);
                if ally {
                    value
                } else {
                    1.0 - value
                }
            }
        };
        nodes[idx].total_value += node_value;
    }
}

fn rollout_value(
    state: &GameState,
    me: Seat,
    n: usize,
    elimination: bool,
    has_teams: bool,
    points_target: Option<i32>,
) -> f64 {
    let mut sim = state.clone();
    // 樹已提供搜尋深度，中局葉 rollout 上限收斂至 80 步；殘局（1000）維持打滿
    let raw_horizon = rollout_horizon(&sim, me, n);
    let horizon = if raw_horizon >= 1_000 {
        raw_horizon
    } else {
        raw_horizon.min(80)
    };
    let mut last_events: Vec<uno_core::Event> = Vec::new();
    let mut steps = 0_usize;
    while !matches!(sim.phase(), Phase::RoundOver | Phase::GameOver) && steps < horizon {
        steps += 1;
        if let Some(u) = sim.uno_window() {
            if sim.legal_actions(u).contains(&Action::CallUno) {
                match sim.apply(u, Action::CallUno) {
                    Ok(ev) => {
                        last_events = ev;
                        continue;
                    }
                    Err(_) => break,
                }
            }
        }
        let Some(actor) = sim.decision_player() else {
            break;
        };
        let legal = sim.legal_actions(actor);
        if legal.is_empty() {
            break;
        }
        let action = if legal.len() == 1 {
            legal[0]
        } else if sim.phase() == Phase::DrawnCardDecision {
            legal
                .iter()
                .copied()
                .find(|a| matches!(a, Action::Play { .. }))
                .unwrap_or(legal[0])
        } else {
            let ctx = StateCtx(&sim, actor);
            choose_tactical_with_ctx(&ctx, &legal)
        };
        match sim.apply(actor, action) {
            Ok(ev) => last_events = ev,
            Err(_) => break,
        }
    }
    evaluate_rollout(
        &sim,
        &last_events,
        me,
        n,
        elimination,
        has_teams,
        points_target,
    )
}
