//! `dimo-cli eval box-select` (T2.9): box select with the real tolerance engine on every truth
//! region of the corpus and of a fixed set of synthetic drawings, reported per category
//! (`dimo_detect::evaluation`). Here and not in `dimo-detect`, because only the CLI may depend
//! on the generator (rust.md dependency direction).

use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use dimo_core::truth::TruthFile;
use dimo_detect::evaluation::{Evaluation, engine_for, evaluate_drawing};
use dimo_pdf::PdfEngine;
use sha2::{Digest, Sha256};

/// Synthetic drawings of the evaluation: these seeds, [`SYNTH_COUNT`] callouts each.
pub const SYNTH_SEEDS: RangeInclusive<u64> = 1..=40;
/// Callouts per synthetic drawing.
pub const SYNTH_COUNT: usize = 15;

/// Why the evaluation could not run. A wrong reading is a failed case, not an error.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// A truth file is invalid.
    #[error("{path}: {source}")]
    Truth {
        /// The truth file.
        path: PathBuf,
        /// The cause.
        source: dimo_core::truth::TruthError,
    },
    /// A corpus drawing does not have the hash its truth file names.
    #[error("{path}: SHA-256 {found} differs from the truth file ({expected})")]
    Hash {
        /// The drawing.
        path: PathBuf,
        /// Hash in the truth file.
        expected: String,
        /// Hash of the file.
        found: String,
    },
    /// The synthetic generator failed.
    #[error("synthetic drawing seed {seed}: {message}")]
    Synth {
        /// Seed.
        seed: u64,
        /// The cause.
        message: String,
    },
    /// The tolerance engine could not be built from the truth settings.
    #[error("{drawing}: tolerance context: {source}")]
    Context {
        /// Drawing name.
        drawing: String,
        /// The cause.
        source: dimo_tolerance::ContextError,
    },
    /// PDFium could not open a drawing.
    #[error("{drawing}: {source}")]
    Pdf {
        /// Drawing name.
        drawing: String,
        /// The cause.
        source: dimo_pdf::PdfError,
    },
}

/// Runs the evaluation on every `*.truth.json` in `<corpus>/truth` (drawings below `corpus`,
/// hash checked) and on the synthetic drawings of `seeds`.
pub fn run(
    engine: &PdfEngine,
    corpus: &Path,
    seeds: RangeInclusive<u64>,
) -> Result<Evaluation, EvalError> {
    let mut evaluation = Evaluation::default();
    for path in truth_files(&corpus.join("truth"))? {
        let truth = read_truth(&path)?;
        let name = path
            .file_name()
            .map(|n| {
                n.to_string_lossy()
                    .trim_end_matches(".truth.json")
                    .to_owned()
            })
            .unwrap_or_default();
        let pdf_path = corpus.join(&truth.drawing.file);
        let pdf = std::fs::read(&pdf_path).map_err(|source| EvalError::Io {
            path: pdf_path.clone(),
            source,
        })?;
        let found = format!("{:x}", Sha256::digest(&pdf));
        if found != truth.drawing.sha256 {
            return Err(EvalError::Hash {
                path: pdf_path,
                expected: truth.drawing.sha256.clone(),
                found,
            });
        }
        evaluate(engine, &mut evaluation, &name, pdf, &truth)?;
    }
    for seed in seeds {
        let drawing = dimo_synth::generate(seed, SYNTH_COUNT).map_err(|e| EvalError::Synth {
            seed,
            message: e.to_string(),
        })?;
        evaluate(
            engine,
            &mut evaluation,
            &drawing.name,
            drawing.pdf,
            &drawing.truth,
        )?;
    }
    Ok(evaluation)
}

fn evaluate(
    engine: &PdfEngine,
    evaluation: &mut Evaluation,
    name: &str,
    pdf: Vec<u8>,
    truth: &TruthFile,
) -> Result<(), EvalError> {
    let interpreter = engine_for(truth).map_err(|source| EvalError::Context {
        drawing: name.to_owned(),
        source,
    })?;
    let doc = engine.open(pdf).map_err(|source| EvalError::Pdf {
        drawing: name.to_owned(),
        source,
    })?;
    evaluation
        .cases
        .extend(evaluate_drawing(&doc, truth, name, &interpreter));
    Ok(())
}

fn truth_files(dir: &Path) -> Result<Vec<PathBuf>, EvalError> {
    let io = |source| EvalError::Io {
        path: dir.to_owned(),
        source,
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.to_string_lossy().ends_with(".truth.json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_truth(path: &Path) -> Result<TruthFile, EvalError> {
    let text = std::fs::read_to_string(path).map_err(|source| EvalError::Io {
        path: path.to_owned(),
        source,
    })?;
    TruthFile::from_json_str(&text).map_err(|source| EvalError::Truth {
        path: path.to_owned(),
        source,
    })
}
