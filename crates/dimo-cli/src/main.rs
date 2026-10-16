//! Headless command line tool for DimoMark.
#![allow(clippy::print_stdout, clippy::print_stderr)] // Binary: prints its report.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use dimo_cli::evaluation::{SYNTH_SEEDS, run};
use dimo_cli::export::{export_project, parse_format, parse_language};
use dimo_core::Language;
use dimo_export::{BalloonMode, ExportFormat, RenderOptions};
use dimo_pdf::PdfEngine;

/// DimoMark command line interface.
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
    /// Exports a saved project through the same code as the desktop app (MF F2): ballooned PDF,
    /// characteristic list, check sheet or inspection report. Never changes the project.
    Export {
        /// The project file (`.dimo`) or folder.
        project: PathBuf,
        /// ballooned-pdf, csv, xlsx, check-sheet-pdf, check-sheet-xlsx, inspection-report-pdf
        /// or inspection-report-xlsx.
        #[arg(long, value_parser = parse_format)]
        format: ExportFormat,
        /// Language of list headers and reports: en or de.
        #[arg(long, value_parser = parse_language, default_value = "en")]
        language: Language,
        /// Blank result columns of a check sheet without samples, 1 to 10.
        #[arg(long, default_value_t = dimo_report::DEFAULT_BLANK_COLUMNS)]
        blank_columns: u8,
        /// Report date `YYYY-MM-DD` for a project without one (the app stores today's date at
        /// the first report export). Ignored when the project has a report date.
        #[arg(long)]
        date: Option<String>,
        /// Ballooned PDF: balloons as annotations instead of page content.
        #[arg(long)]
        annotations: bool,
        /// The file to write.
        #[arg(long, short)]
        out: PathBuf,
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
    match Cli::parse().command {
        Some(Command::Eval {
            what: Eval::BoxSelect { corpus, seeds },
        }) => eval_box_select(&corpus, seeds),
        Some(Command::Export {
            project,
            format,
            language,
            blank_columns,
            date,
            annotations,
            out,
        }) => {
            let options = RenderOptions {
                format,
                language,
                balloons: if annotations {
                    BalloonMode::Annotations
                } else {
                    BalloonMode::PageContent
                },
                blank_columns,
            };
            export(&project, options, date.as_deref(), &out)
        }
        None => ExitCode::SUCCESS,
    }
}

fn export(project: &Path, options: RenderOptions, date: Option<&str>, out: &Path) -> ExitCode {
    let engine = if options.format.needs_drawing() {
        match PdfEngine::start() {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        None
    };
    let written = export_project(project, options, date, engine.as_ref())
        .map_err(|e| e.to_string())
        .and_then(|bytes| {
            std::fs::write(out, bytes).map_err(|e| format!("{}: {e}", out.display()))
        });
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn eval_box_select(corpus: &Path, seeds: u64) -> ExitCode {
    let engine = match PdfEngine::start() {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    match run(&engine, corpus, 1..=seeds) {
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
