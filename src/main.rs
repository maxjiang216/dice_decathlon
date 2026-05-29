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
    /// Solve disciplines and write artifacts under the output dir.
    Analyze {
        /// Discipline key, or omit to analyze all of them.
        key: Option<String>,
        /// Output directory (default: `output`).
        #[arg(long, default_value = "output")]
        out: PathBuf,
    },
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
