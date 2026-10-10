//! Box select evaluation on the corpus drawings (T2.9, M2 exit criterion, FR-REC-01,
//! FR-REC-02, FR-REC-08, FR-TOL-01). Every truth region of every corpus truth file is box
//! selected with the real tolerance engine; every characteristic must match. The synthetic
//! drawings join in `dimo-cli` (only the CLI may depend on the generator).
//!
//! The truth of `test_drawing_1` is not yet reviewed by the owner and the tolerance tables are
//! drafts (D-43), so a pass here means "agrees with the draft truth".
//!
//! Skips without PDFium like the `dimo-pdf` tests, fails instead with `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1`.

#![allow(clippy::unwrap_used, clippy::print_stderr)] // Test crate; rust.md allows unwrap in tests.

use std::path::PathBuf;

use dimo_core::truth::TruthFile;
use dimo_detect::evaluation::{CaseResult, Category, Evaluation, engine_for, evaluate_drawing};
use dimo_pdf::{PdfEngine, PdfError};

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn engine() -> Option<PdfEngine> {
    match PdfEngine::start() {
        Ok(engine) => Some(engine),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

#[test]
fn every_corpus_truth_region_is_read_correctly() {
    let Some(engine) = engine() else { return };
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus().join("truth"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".truth.json"))
        .collect();
    files.sort();
    assert_ne!(files.len(), 0, "no truth files");
    let mut evaluation = Evaluation::default();
    for file in files {
        let truth = TruthFile::from_json_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        // The drawing hash is checked against PROVENANCE.md by the dimo-core corpus tests and
        // against the truth file by `dimo-cli eval box-select`.
        let pdf = std::fs::read(corpus().join(&truth.drawing.file)).unwrap();
        let doc = engine.open(pdf).unwrap();
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let interpreter = engine_for(&truth).unwrap();
        evaluation
            .cases
            .extend(evaluate_drawing(&doc, &truth, &name, &interpreter));
    }
    let report = evaluation.report();
    assert!(
        evaluation.cases.iter().all(CaseResult::correct),
        "corpus box select failures:\n{report}"
    );
    assert!(evaluation.passes(), "{report}");
    // test_drawing_1 holds 23 common callouts and one note.
    assert!(evaluation.gated().count >= 23, "{report}");
    assert!(evaluation.category(Category::Fit).count >= 6, "{report}");
}
