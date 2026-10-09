//! Prints the text runs of one sheet of a PDF with their position, for checking exports by hand
//! (for example that a ballooned PDF carries its balloon numbers, T1.9).
//!
//! ```text
//! cargo run -p dimo-pdf --example text_runs -- <file.pdf> [sheet]
//! ```

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "command line tool for developers"
)]

use std::process::ExitCode;

use dimo_pdf::PdfEngine;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: text_runs <file.pdf> [sheet]");
        return ExitCode::FAILURE;
    };
    let sheet = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let result = (|| {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let engine = PdfEngine::start().map_err(|e| e.to_string())?;
        let doc = engine.open(bytes).map_err(|e| e.to_string())?;
        let runs = doc.text_runs(sheet).map_err(|e| e.to_string())?;
        println!(
            "{} sheets, sheet {sheet}: {} runs",
            doc.sheet_count(),
            runs.len()
        );
        for run in runs {
            let b = run.bbox;
            println!(
                "{:8.1} {:8.1} {:6.1} {:6.1}  {:<12} {}",
                b.x, b.y, b.width, b.height, run.font_name, run.text
            );
        }
        Ok::<_, String>(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
