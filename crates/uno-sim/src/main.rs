use std::process;
use std::time::Instant;

use clap::{Args, Parser, Subcommand};
use uno_bots::{make_bot, Bot, BotKind, HardBot, DEFAULT_SIMS};
use uno_core::{Action, GameState, Phase, RuleConfig, Scoring};

#[derive(Debug, Parser)]
#[command(name = "uno-sim", about = "UNO 批次對戰模擬與 AI 評估工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 執行多個 Bot 之間的輪替座位批次對戰
    Arena(ArenaArgs),
}

#[derive(Debug, Args)]
struct ArenaArgs {
    /// 模式 preset ID（classic / party / flip / no_mercy / teams / elimination / blitz / daily）
    #[arg(long, default_value = "classic")]
    mode: String,

    /// 玩家人數
    #[arg(long, default_value_t = 4)]
    players: u8,

    /// 各槽位 Bot 類型，以逗號分隔（例如 hard,heuristic,heuristic,random）
    #[arg(long, value_delimiter = ',', required = true)]
    bots: Vec<String>,

    /// 總對戰局數
    #[arg(long, default_value_t = 2000)]
    games: usize,

    /// 基礎亂數種子
    #[arg(long, default_value_t = 1)]
    seed: u64,

    /// Hard Bot 每次決策模擬次數
    #[arg(long, default_value_t = DEFAULT_SIMS)]
    sims: usize,

    /// 是否只進行單輪（覆蓋模式 preset 的目標分數）
    #[arg(long, default_value_t = false)]
    single_round: bool,
}

#[derive(Default, Clone, Debug)]
struct SlotStats {
    wins: usize,
    rank_sum: usize,
    games: usize,
}

fn wilson_95(wins: usize, total: usize) -> (f64, f64) {
    if total == 0 {
        return (0.0, 0.0);
    }
    const Z: f64 = 1.959_963_984_540_054;
    let n = total as f64;
    let p = (wins as f64) / n;
    let z2 = Z * Z;
    let denom = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denom;
    let margin = (Z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt()) / denom;
    ((center - margin).max(0.0), (center + margin).min(1.0))
}

fn build_bot(kind: &BotKind, seed: u64, sims: usize) -> Result<Box<dyn Bot>, String> {
    match kind {
        BotKind::Hard => Ok(Box::new(HardBot::new(seed, sims))),
        other => make_bot(other, seed).map_err(|e| e.to_string()),
    }
}

fn run_arena(args: &ArenaArgs) -> Result<(), String> {
    if args.games == 0 {
        return Err("--games 必須大於 0".into());
    }
    if args.bots.len() != usize::from(args.players) {
        return Err(format!(
            "--bots 數量 ({}) 必須等於 --players ({})",
            args.bots.len(),
            args.players
        ));
    }

    let mut config = RuleConfig::preset(&args.mode, args.players).map_err(|e| e.to_string())?;
    if args.single_round {
        config.scoring = Scoring::SingleRound;
    }

    let bot_kinds: Vec<BotKind> = args
        .bots
        .iter()
        .map(|s| s.parse::<BotKind>().map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    let n = usize::from(args.players);
    let mut stats = vec![SlotStats::default(); n];
    let mut total_steps = 0_u64;
    let start = Instant::now();

    for game_idx in 0..args.games {
        let game_seed = args.seed.wrapping_add(game_idx as u64);
        let mut state = GameState::new(config.clone(), game_seed).map_err(|e| e.to_string())?;

        // 座位每局輪替，消除先行順位偏差：seat = (bot_slot + game_idx) % n
        let mut seat_to_slot = vec![0_usize; n];
        let mut seat_bots: Vec<Option<Box<dyn Bot>>> = (0..n).map(|_| None).collect();
        for (slot, kind) in bot_kinds.iter().enumerate() {
            let seat = (slot + game_idx) % n;
            seat_to_slot[seat] = slot;
            let bot_seed = game_seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((slot as u64) + 1);
            seat_bots[seat] = Some(build_bot(kind, bot_seed, args.sims)?);
        }

        let mut bots: Vec<Box<dyn Bot>> = seat_bots
            .into_iter()
            .map(|b| b.ok_or_else(|| "內部錯誤：缺少 Bot".to_string()))
            .collect::<Result<Vec<_>, _>>()?;

        let mut game_steps = 0_usize;
        while !matches!(state.phase(), Phase::GameOver) && game_steps < 50_000 {
            if matches!(state.phase(), Phase::RoundOver) {
                state.next_round().map_err(|e| e.to_string())?;
                continue;
            }

            if let Some(u_seat) = state.uno_window() {
                if state.legal_actions(u_seat).contains(&Action::CallUno) {
                    let u_view = state.view(u_seat);
                    let action = bots[usize::from(u_seat)].act(&u_view);
                    state.apply(u_seat, action).map_err(|e| e.to_string())?;
                    game_steps += 1;
                    total_steps += 1;
                    continue;
                }
            }

            let seat = state
                .decision_player()
                .ok_or_else(|| "進行中盤面卻無決策玩家".to_string())?;
            let view = state.view(seat);
            let action = bots[usize::from(seat)].act(&view);
            state.apply(seat, action).map_err(|e| e.to_string())?;
            game_steps += 1;
            total_steps += 1;
        }

        if let Some(rankings) = state.rankings() {
            for (rank_idx, &seat) in rankings.iter().enumerate() {
                let slot = seat_to_slot[usize::from(seat)];
                stats[slot].games += 1;
                stats[slot].rank_sum += rank_idx + 1;
                if rank_idx == 0 {
                    stats[slot].wins += 1;
                }
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let steps_per_sec = if elapsed > 0.0 {
        (total_steps as f64) / elapsed
    } else {
        0.0
    };

    println!(
        "UNO Arena | mode={} | players={} | games={} | sims={} | seed={} | elapsed={:.2}s | steps={} ({:.0} steps/s)",
        args.mode, args.players, args.games, args.sims, args.seed, elapsed, total_steps, steps_per_sec
    );
    println!(
        "{:<6} {:<16} {:>7} {:>9} {:>19} {:>10}",
        "Slot", "Bot", "Wins", "WinRate", "95% CI (Wilson)", "AvgRank"
    );

    for (slot, st) in stats.iter().enumerate() {
        let win_rate = if st.games > 0 {
            (st.wins as f64) * 100.0 / (st.games as f64)
        } else {
            0.0
        };
        let (ci_lo, ci_hi) = wilson_95(st.wins, st.games);
        let avg_rank = if st.games > 0 {
            (st.rank_sum as f64) / (st.games as f64)
        } else {
            0.0
        };
        println!(
            "#{:<5} {:<16} {:>7} {:>8.2}% [{:>6.2}%, {:>6.2}%] {:>10.3}",
            slot,
            bot_kinds[slot].to_string(),
            st.wins,
            win_rate,
            ci_lo * 100.0,
            ci_hi * 100.0,
            avg_rank
        );
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Arena(ref args) => run_arena(args),
    };
    if let Err(err) = result {
        eprintln!("uno-sim 錯誤：{err}");
        process::exit(1);
    }
}
