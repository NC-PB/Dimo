//! `dimo-synth`: write a synthetic drawing PDF and its truth file.
#![allow(clippy::print_stdout)] // Binary: reports the written files.

use std::path::PathBuf;

use clap::Parser;
use dimo_synth::{generate, generate_stress};

/// Generate a synthetic dimensioned drawing and its ground truth from a seed.
#[derive(Debug, Parser)]
#[command(name = "dimo-synth", version, about)]
struct Cli {
    /// Seed. The same seed and tool version always produce byte identical files.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Number of callouts on the sheet (1 to 15).
    #[arg(long, default_value_t = 12)]
    count: usize,
    /// Write a stress document for the performance harness (T0.9) instead of a dimensioned
    /// drawing: `--sheets` A0 sheets of dense line work and text in `<out>/stress_<seed>_<sheets>.pdf`.
    /// No truth file. `--count` is ignored.
    #[arg(long)]
    stress: bool,
    /// Number of A0 sheets of the stress document.
    #[arg(long, default_value_t = 50, requires = "stress")]
    sheets: usize,
    /// Output directory (created if missing). Do not point it into `corpus/`.
    #[arg(long, default_value = "target/synth")]
    out: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if cli.stress {
        std::fs::create_dir_all(&cli.out)?;
        let pdf = cli
            .out
            .join(format!("stress_{}_{}.pdf", cli.seed, cli.sheets));
        std::fs::write(&pdf, generate_stress(cli.seed, cli.sheets))?;
        println!("{}", pdf.display());
        return Ok(());
    }
    let drawing = generate(cli.seed, cli.count)?;
    std::fs::create_dir_all(&cli.out)?;
    let pdf = cli.out.join(format!("{}.pdf", drawing.name));
    let truth = cli.out.join(format!("{}.truth.json", drawing.name));
    std::fs::write(&pdf, &drawing.pdf)?;
    std::fs::write(&truth, &drawing.truth_json)?;
    println!("{}\n{}", pdf.display(), truth.display());
    Ok(())
}
