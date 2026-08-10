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
fn storage_report() {
    use dice_decathlon::twoplayer::{
        apply_turn_order, compress, javelin, m1500, Axis,
    };

    // 1500m is last, so nothing follows it. Javelin is second to last and
    // hands over to the 1500m with turn order already applied.
    // 1500m keeps the full axis. The difference is a *state variable*
    // here -- freezing a set folds its score straight into it -- so play
    // starting from d >= 0 still visits negative differences. Only the
    // entry value is restricted to the non-negative half, not the table.
    let m15_axis = Axis::for_event(417, 88, 83);
    // Only the non-negative half: the leader starts, so a first
    // mover is never behind. The rest follows by relabelling.
    let jav_axis = Axis::first_mover(Axis::for_event(387, 118, 30).hi);

    let after_javelin_axis = Axis::symmetric(m15_axis.hi + 48);
    let first = m1500::solve(after_javelin_axis);
    let v9 = apply_turn_order(&first, after_javelin_axis);
    let after = move |d: i32| v9[after_javelin_axis.idx(d)];

    println!(
        "{:9} {:>13} {:>12} {:>11} {:>10} {:>10}  {:>7}  {:>6}",
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
        compress::report("javelin", javelin::measure(jav_axis, &after))
    );
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
