use uno_core::{
    Action, CardFace, CardId, Color, Face, GameState, PlayerView, Seat, Side, Wild4Rule,
};

#[derive(Clone, Copy)]
pub(crate) struct SeatSummary {
    pub(crate) seat: Seat,
    pub(crate) hand_size: u8,
    pub(crate) eliminated: bool,
    pub(crate) team: Option<u8>,
}

pub(crate) trait TacticalCtx {
    fn me(&self) -> Seat;
    fn direction(&self) -> i8;
    fn active_color(&self) -> Option<Color>;
    fn wild4_rule(&self) -> Wild4Rule;
    fn for_each_hand_face(&self, f: impl FnMut(CardFace));
    fn card_face(&self, id: CardId) -> Option<CardFace>;
    fn num_players(&self) -> usize;
    fn seat_summary(&self, seat: Seat) -> Option<SeatSummary>;
    fn discard_color_count(&self, color: Color) -> i32;
}

pub(crate) struct ViewCtx<'a>(pub(crate) &'a PlayerView, pub(crate) Seat);

impl TacticalCtx for ViewCtx<'_> {
    fn me(&self) -> Seat {
        self.1
    }
    fn direction(&self) -> i8 {
        self.0.direction
    }
    fn active_color(&self) -> Option<Color> {
        self.0.active_color
    }
    fn wild4_rule(&self) -> Wild4Rule {
        self.0.config.wild4_rule
    }
    fn for_each_hand_face(&self, mut f: impl FnMut(CardFace)) {
        let side = self.0.side;
        for c in &self.0.my_hand {
            f(c.face(side));
        }
    }
    fn card_face(&self, id: CardId) -> Option<CardFace> {
        let side = self.0.side;
        self.0
            .my_hand
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.face(side))
    }
    fn num_players(&self) -> usize {
        self.0.players.len()
    }
    fn seat_summary(&self, seat: Seat) -> Option<SeatSummary> {
        self.0.players.get(usize::from(seat)).map(|p| SeatSummary {
            seat: p.seat,
            hand_size: p.hand_size,
            eliminated: p.eliminated,
            team: p.team,
        })
    }
    fn discard_color_count(&self, color: Color) -> i32 {
        let side = self.0.side;
        self.0
            .discard
            .iter()
            .filter(|c| c.face(side).color == Some(color))
            .count() as i32
    }
}

pub(crate) struct StateCtx<'a>(pub(crate) &'a GameState, pub(crate) Seat);

impl TacticalCtx for StateCtx<'_> {
    fn me(&self) -> Seat {
        self.1
    }
    fn direction(&self) -> i8 {
        self.0.direction()
    }
    fn active_color(&self) -> Option<Color> {
        self.0.active_color()
    }
    fn wild4_rule(&self) -> Wild4Rule {
        self.0.config().wild4_rule
    }
    fn for_each_hand_face(&self, mut f: impl FnMut(CardFace)) {
        let side = self.0.side();
        for &cid in self.0.hand_ids(self.1) {
            if let Some(c) = self.0.card_info(cid) {
                f(c.face(side));
            }
        }
    }
    fn card_face(&self, id: CardId) -> Option<CardFace> {
        let side = self.0.side();
        self.0.card_info(id).map(|c| c.face(side))
    }
    fn num_players(&self) -> usize {
        usize::from(self.0.num_players())
    }
    fn seat_summary(&self, seat: Seat) -> Option<SeatSummary> {
        if usize::from(seat) >= usize::from(self.0.num_players()) {
            return None;
        }
        Some(SeatSummary {
            seat,
            hand_size: self.0.hand_ids(seat).len() as u8,
            eliminated: self.0.is_eliminated(seat),
            team: self.0.team_of_seat(seat),
        })
    }
    fn discard_color_count(&self, _color: Color) -> i32 {
        0
    }
}

pub(crate) fn compute_action_priors(ctx: &impl TacticalCtx, candidates: &[Action]) -> Vec<f64> {
    let scores: Vec<f64> = candidates
        .iter()
        .map(|&a| f64::from(score_tactical_action(ctx, candidates, a)))
        .collect();
    let min_s = scores.iter().copied().fold(f64::INFINITY, f64::min);
    let max_s = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_s - min_s).max(1.0);
    scores.into_iter().map(|s| (s - min_s) / span).collect()
}

pub(crate) fn best_catch_uno(ctx: &impl TacticalCtx, legal: &[Action]) -> Option<Action> {
    legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::CatchUno { target } => {
                let size = ctx
                    .seat_summary(target)
                    .map(|p| p.hand_size)
                    .unwrap_or(u8::MAX);
                Some((size, target, a))
            }
            _ => None,
        })
        .min_by_key(|&(size, target, _)| (size, target))
        .map(|(_, _, a)| a)
}

pub(crate) fn choose_tactical_with_ctx(ctx: &impl TacticalCtx, legal: &[Action]) -> Action {
    if legal.contains(&Action::CallUno) {
        return Action::CallUno;
    }
    if let Some(catch) = best_catch_uno(ctx, legal) {
        return catch;
    }
    if let Some(&play) = legal.iter().find(|a| matches!(a, Action::Play { .. })) {
        if legal.contains(&Action::Pass) {
            return play;
        }
    }

    legal
        .iter()
        .copied()
        .max_by_key(|&a| score_tactical_action(ctx, legal, a))
        .unwrap_or(Action::Draw)
}

fn score_tactical_action(ctx: &impl TacticalCtx, legal: &[Action], action: Action) -> i32 {
    match action {
        Action::ChooseColor { color } => score_choose_color(ctx, color),
        Action::ChooseTarget { target } => {
            let my_team = ctx.seat_summary(ctx.me()).and_then(|p| p.team);
            let p = ctx.seat_summary(target);
            let is_teammate = my_team.is_some() && p.and_then(|x| x.team) == my_team;
            let hand_size = i32::from(p.map(|x| x.hand_size).unwrap_or(u8::MAX));
            if is_teammate {
                -1000 - hand_size
            } else {
                1000 - hand_size
            }
        }
        Action::Play { card } | Action::JumpIn { card } => score_play_card(ctx, legal, card),
        Action::Draw => -500,
        _ => 0,
    }
}

fn score_choose_color(ctx: &impl TacticalCtx, color: Color) -> i32 {
    let mut in_hand = 0_i32;
    let mut action_bonus = 0_i32;
    ctx.for_each_hand_face(|cf| {
        if cf.color == Some(color) {
            in_hand += 1;
            if matches!(
                cf.face,
                Face::DrawTwo | Face::DrawFour | Face::DrawFive | Face::Skip | Face::SkipEveryone
            ) {
                action_bonus += 3;
            }
        }
    });
    in_hand * 24 + action_bonus + ctx.discard_color_count(color) * 3
}

fn score_play_card(ctx: &impl TacticalCtx, legal: &[Action], card_id: CardId) -> i32 {
    let Some(face) = ctx.card_face(card_id) else {
        return -100;
    };

    let me = ctx.me();
    let my_team = ctx.seat_summary(me).and_then(|p| p.team);
    let n = ctx.num_players();
    let active_players = (0..n)
        .filter(|&i| ctx.seat_summary(i as Seat).is_some_and(|p| !p.eliminated))
        .count();

    let next = relative_active_player(ctx, me, 1, ctx.direction());
    let after_next = relative_active_player(ctx, me, 2, ctx.direction());
    let prev = relative_active_player(ctx, me, 1, -ctx.direction());

    let is_opp = |p: Option<SeatSummary>| p.is_some_and(|x| my_team.is_none() || x.team != my_team);
    let next_is_opp = is_opp(next);
    let after_next_is_opp = is_opp(after_next);
    let prev_is_opp = is_opp(prev);
    let next_hand = next.map_or(7, |x| i32::from(x.hand_size));
    let after_next_hand = after_next.map_or(7, |x| i32::from(x.hand_size));
    let prev_hand = prev.map_or(7, |x| i32::from(x.hand_size));
    let next_danger = next_is_opp && next_hand <= 2;
    let after_next_danger = after_next_is_opp && after_next_hand <= 2;
    let prev_danger = prev_is_opp && prev_hand <= 2;
    let opp_at_one = (0..n).any(|i| {
        ctx.seat_summary(i as Seat).is_some_and(|p| {
            !p.eliminated
                && p.seat != me
                && (my_team.is_none() || p.team != my_team)
                && p.hand_size <= 1
        })
    });

    let mut my_hand_size = 0_i32;
    let mut color_counts = [0_i32; 4];
    let mut has_active_color = false;
    ctx.for_each_hand_face(|cf| {
        my_hand_size += 1;
        if let Some(col) = cf.color {
            color_counts[usize::from(col.slot())] += 1;
            if Some(col) == ctx.active_color() {
                has_active_color = true;
            }
        }
    });

    let mut has_non_wild_play = false;
    let mut has_non_wild_stop = false;
    for &a in legal {
        if let Action::Play { card } | Action::JumpIn { card } = a {
            if let Some(cf) = ctx.card_face(card) {
                if !cf.is_wild() {
                    has_non_wild_play = true;
                    if matches!(
                        cf.face,
                        Face::DrawOne
                            | Face::DrawTwo
                            | Face::DrawFour
                            | Face::DrawFive
                            | Face::Skip
                            | Face::SkipEveryone
                            | Face::Reverse
                    ) {
                        has_non_wild_stop = true;
                    }
                }
            }
        }
    }

    let mut score = 0_i32;
    if let Some(col) = face.color {
        score += color_counts[usize::from(col.slot())] * 6;
    }

    match face.face {
        Face::Number(v) => score += i32::from(v),
        Face::DiscardAll => score += 25,
        Face::DrawOne | Face::DrawTwo | Face::DrawFour | Face::DrawFive => {
            score -= 8;
            if next_danger {
                score += 65;
            } else if !next_is_opp {
                score -= 60;
            } else if opp_at_one {
                score += 18;
            }
        }
        Face::Skip | Face::SkipEveryone => {
            score -= 8;
            if matches!(face.face, Face::Skip) && active_players > 2 {
                if !next_is_opp {
                    score -= 55;
                } else if next_danger {
                    score += if after_next_danger { -40 } else { 60 };
                } else if after_next_danger {
                    score -= 40;
                } else {
                    if after_next_is_opp {
                        score += (after_next_hand - next_hand).clamp(-4, 4) * 3;
                    }
                    if opp_at_one {
                        score += 15;
                    }
                }
            } else if next_danger {
                score += 60;
            } else if opp_at_one {
                score += 15;
            }
        }
        Face::Reverse => {
            score -= 8;
            if active_players == 2 {
                if next_danger {
                    score += 60;
                } else if opp_at_one {
                    score += 15;
                }
            } else if next_danger && !prev_danger && prev_hand > next_hand {
                score += 55;
            } else if prev_danger {
                score -= 70;
            } else {
                score += (prev_hand - next_hand).clamp(-4, 4) * 4;
                if !prev_is_opp {
                    score += 15;
                }
                if opp_at_one {
                    score += 15;
                }
            }
        }
        Face::Flip => score -= 4,
        Face::Wild => {
            if has_non_wild_play {
                score -= 60;
            }
        }
        Face::WildDrawTwo | Face::WildDrawFour | Face::WildDrawColor => {
            let is_bluff = ctx.wild4_rule() == Wild4Rule::Challenge && has_active_color;
            if is_bluff {
                if next_is_opp && next_hand <= 1 && !has_non_wild_stop && my_hand_size <= 3 {
                    score += 50;
                } else {
                    score -= 150;
                }
            } else if !has_active_color && next_danger {
                score += 70;
            } else if has_non_wild_play {
                score -= 35;
            }
        }
        Face::WildReverseDrawFour
        | Face::WildDrawSix
        | Face::WildDrawTen
        | Face::WildColorRoulette => {
            if next_danger {
                score += 65;
            } else if has_non_wild_play {
                score -= 10;
            }
        }
    }

    let _ = Side::Light;
    score
}

fn relative_active_player(
    ctx: &impl TacticalCtx,
    from: Seat,
    steps: u8,
    dir: i8,
) -> Option<SeatSummary> {
    let n = ctx.num_players();
    if n <= 1 {
        return None;
    }
    let mut cur = i32::from(from);
    let step_dir = if dir >= 0 { 1_i32 } else { -1_i32 };
    for _ in 0..steps {
        for _ in 0..n {
            cur = (cur + step_dir).rem_euclid(n as i32);
            if let Some(p) = ctx.seat_summary(cur as Seat) {
                if !p.eliminated {
                    break;
                }
            }
        }
    }
    ctx.seat_summary(cur as Seat)
        .filter(|p| !p.eliminated && p.seat != from)
}
