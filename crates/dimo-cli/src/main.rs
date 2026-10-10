//! Headless command line tool for Dimo.
#![allow(clippy::print_stdout, clippy::print_stderr)] // Binary: prints its report.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use dimo_cli::evaluation::{SYNTH_SEEDS, run};
use dimo_pdf::PdfEngine;

/// Dimo command line interface.
#[derive(Debug, Parser)]
#[command(name = "dimo", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluations of the recognition (spec 08 Evaluation).
    Eval {
        #[command(subcommand)]
        what: Eval,
    },
}

#[derive(Debug, Subcommand)]
enum Eval {
    /// Box select with the tolerance engine on every truth region of the corpus and of the
    /// synthetic drawings, reported per callout category (M2 exit criterion, T2.9). Exits with
    /// status 1 when fewer than 99 percent of the common callouts are correct.
    BoxSelect {
        /// The corpus directory (holds `truth/` and the drawings).
        #[arg(long, default_value = "corpus")]
        corpus: PathBuf,
        /// Number of synthetic drawings (seeds 1 to N). The default is the fixed CI set.
        #[arg(long, default_value_t = *SYNTH_SEEDS.end())]
        seeds: u64,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Some(Command::Eval {
        what: Eval::BoxSelect { corpus, seeds },
    }) = cli.command
    else {
        return ExitCode::SUCCESS;
    };
    let engine = match PdfEngine::start() {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    match run(&engine, &corpus, 1..=seeds) {
        Ok(evaluation) => {
            print!("{}", evaluation.report());
            if evaluation.passes() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
