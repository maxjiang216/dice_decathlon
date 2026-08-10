//! Command-line entry point for the Decathlon solvers.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use dice_decathlon::analysis;
use dice_decathlon::disciplines;
use dice_decathlon::policy::Solved;

#[derive(Parser)]
#[command(
    name = "decathlon",
    about = "Optimal EV policies for Knizia's Decathlon dice game"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the available discipline keys.
    List,
    /// Solve one discipline and print its expected value.
    Solve {
        /// Discipline key, e.g. `100m` (see `list`).
        key: String,
    },
    /// Compare two-player policy storage across compression schemes.
    Storage,
    /// Solve the whole competition and print the value of a lead.
    Chain,
    /// Solve disciplines and write artifacts under the output dir.
    Analyze {
        /// Discipline key, or omit to analyze all of them.
        key: Option<String>,
        /// Output directory (default: `output`).
        #[arg(long, default_value = "output")]
        out: PathBuf,
    },
}

/// Print the two-player policy storage comparison.
///
/// Events must be solved back to front: each needs the value function of
/// everything after it, and only the 1500m stands alone.
fn storage_report() {
    use dice_decathlon::twoplayer::{
        compress, discus, highjump, javelin, longjump, m1500, polevault,
        running, shotput, Axis,
    };

    /// Read a first-mover value function at any difference.
    ///
    /// The leading player starts, so a first mover is never behind and
    /// the table only covers `d >= 0`; below zero the two players swap
    /// roles. At a tie the rulebook's die roll makes it an even mix.
    fn as_player(table: &[f64], axis: Axis, d: i32) -> f64 {
        match d.signum() {
            1 => table[axis.idx(d)],
            -1 => 1.0 - table[axis.idx(-d)],
            _ => 0.5,
        }
    }

    // 1500m keeps the full axis: the difference is a state variable there,
    // so play starting level still visits negative differences.
    let m15_axis = Axis::for_event(417, 88, 83);
    let m15 = m1500::solve(m15_axis);
    let v9 = move |d: i32| as_player(&m15, m15_axis, d);

    // Javelin and pole vault hold the difference fixed while they run, so
    // the non-negative half is the whole table.
    let jav_axis = Axis::first_mover(Axis::for_event(387, 118, 30).hi);
    let jav = javelin::solve_first_mover(jav_axis, &v9);
    let v8 = move |d: i32| as_player(&jav, jav_axis, d);

    let pv_axis = Axis::first_mover(Axis::for_event(339, 166, 48).hi);
    let pv = polevault::solve_first_mover(pv_axis, &v8);
    let v7 = move |d: i32| as_player(&pv, pv_axis, d);

    let dis_axis = Axis::first_mover(Axis::for_event(309, 196, 30).hi);
    let dis = discus::solve_first_mover(dis_axis, &v7);
    let v6 = move |d: i32| as_player(&dis, dis_axis, d);

    // The running events keep the full axis: their difference is a state
    // variable, so play starting level still visits negative differences.
    let hur_axis = Axis::for_event(284, 221, 30);
    let hurdles = running::hurdles();
    let hur = hurdles.solve_first_mover(hur_axis, &v6);
    let v5 = move |d: i32| as_player(&hur, hur_axis, d);

    let m400_axis = Axis::for_event(196, 309, 78);
    let m400 = running::m400();
    let f400 = m400.solve_first_mover(m400_axis, &v5);
    let v4 = move |d: i32| as_player(&f400, m400_axis, d);

    let hj_axis = Axis::first_mover(Axis::for_event(166, 339, 30).hi);
    let hj = highjump::solve_first_mover(hj_axis, &v4);
    let v3 = move |d: i32| as_player(&hj, hj_axis, d);

    let sp_axis = Axis::first_mover(Axis::for_event(118, 387, 48).hi);
    let sp = shotput::solve_first_mover(sp_axis, &v3);
    let v2 = move |d: i32| as_player(&sp, sp_axis, d);

    let lj_axis = Axis::first_mover(Axis::for_event(88, 417, 30).hi);
    let lj = longjump::solve_first_mover(lj_axis, &v2);
    let v1 = move |d: i32| as_player(&lj, lj_axis, d);

    let m100_axis = Axis::for_event(0, 505, 68);
    let m100 = running::m100();

    println!(
        "{:10} {:>13} {:>12} {:>11} {:>10} {:>10}  {:>7}  {:>6}",
        "event",
        "states",
        "raw B",
        "packed B",
        "dense B",
        "sparse B",
        "deviate",
        "best"
    );
    println!("{}", compress::report("1500m", m1500::measure(m15_axis)));
    println!(
        "{}",
        compress::report("javelin", javelin::measure(jav_axis, &v9))
    );
    println!(
        "{}",
        compress::report("polevault", polevault::measure(pv_axis, &v8))
    );
    println!(
        "{}",
        compress::report("discus", discus::measure(dis_axis, &v7))
    );
    println!(
        "{}",
        compress::report("110mh", hurdles.measure(hur_axis, &v6))
    );
    println!("{}", compress::report("400m", m400.measure(m400_axis, &v5)));
    println!(
        "{}",
        compress::report("highjump", highjump::measure(hj_axis, &v4))
    );
    println!(
        "{}",
        compress::report("shotput", shotput::measure(sp_axis, &v3))
    );
    println!(
        "{}",
        compress::report("longjump", longjump::measure(lj_axis, &v2))
    );
    println!("{}", compress::report("100m", m100.measure(m100_axis, &v1)));
}

fn report(solved: &Solved) {
    println!(
        "{:<10} {:<18} EV={:>8.4}  SD={:>7.4}  range=[{}, {}]",
        solved.key,
        solved.name,
        solved.dist.mean(),
        solved.dist.std_dev(),
        solved.dist.mass.keys().next().unwrap(),
        solved.dist.mass.keys().next_back().unwrap(),
    );
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Storage => storage_report(),
        Command::Chain => chain_report(),
        Command::List => {
            for (key, _) in disciplines::registry() {
                println!("{key}");
            }
        }
        Command::Solve { key } => {
            let Some(solver) = disciplines::find(&key) else {
                eprintln!("unknown discipline: {key}");
                return ExitCode::FAILURE;
            };
            report(&solver());
        }
        Command::Analyze { key, out } => {
            let solvers: Vec<disciplines::Solver> = if let Some(k) = key {
                let Some(solver) = disciplines::find(&k) else {
                    eprintln!("unknown discipline: {k}");
                    return ExitCode::FAILURE;
                };
                vec![solver]
            } else {
                disciplines::registry()
                    .into_iter()
                    .map(|(_, s)| s)
                    .collect()
            };
            for solver in solvers {
                let solved = solver();
                report(&solved);
                if let Err(e) = analysis::write_outputs(&solved, &out) {
                    eprintln!("failed writing {}: {e}", solved.key);
                    return ExitCode::FAILURE;
                }
            }
            println!("artifacts written under {}", out.display());
        }
    }
    ExitCode::SUCCESS
}

/// Solve the whole competition and print what each score difference is
/// worth entering each event.
fn chain_report() {
    use dice_decathlon::twoplayer::chain::Chain;

    let chain = Chain::solve();
    println!("Win probability of the player about to start each event.\n");
    print!("{:>5}", "d");
    for key in Chain::keys() {
        print!("{key:>10}");
    }
    println!();
    for d in [-40, -20, -10, -3, 0, 3, 10, 20, 40] {
        print!("{d:>5}");
        for (e, _) in Chain::keys().iter().enumerate() {
            print!("{:>10.4}", chain.value(e, d));
        }
        println!();
    }
}
