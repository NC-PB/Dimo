//! Drawing import end to end: sheets read through `dimo-pdf`, stored by `dimo-io` (T1.3,
//! FR-DOC-07). `dimo-io` may not depend on `dimo-pdf`, so this glue lives with the CLI, which
//! may use every crate.
//!
//! Skips without PDFium, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::{FixedEnvironment, ProjectInfo, Sha256Hex, SheetKind, Size};
use dimo_io::project::{
    Layout, ProjectError, ProjectFile, ProjectSession, SheetInfo, import_drawing,
};
use dimo_pdf::{PdfEngine, PdfError};

/// SHA-256 of `corpus/drawings/test_drawing_1.pdf` from `corpus/PROVENANCE.md`.
const TEST_DRAWING_1_SHA256: &str =
    "635a89735fc1a305a99c3d42e394785d0ca00e2d82f2af1d5bdba8b66fca5c82";

fn corpus_drawing() -> Vec<u8> {
    let path = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../corpus/drawings/test_drawing_1.pdf");
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

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

/// What `dimo-pdf` reports per sheet, and the hash of the analyzed bytes.
fn analyze(engine: &PdfEngine, bytes: &[u8]) -> (Vec<SheetInfo>, Sha256Hex) {
    let pdf = engine.open(bytes.to_vec()).unwrap();
    let sheets = (0..pdf.sheet_count())
        .map(|i| {
            let size = pdf.sheet_size(i).unwrap();
            SheetInfo {
                size: Size {
                    width: size.width,
                    height: size.height,
                },
                kind: pdf.analyze_sheet(i).unwrap().kind,
            }
        })
        .collect();
    let hash = Sha256Hex::parse(&pdf.content_hash().to_hex()).unwrap();
    (sheets, hash)
}

#[test]
fn test_drawing_1_imports_saves_and_reopens() {
    let Some(engine) = engine() else { return };
    let bytes = corpus_drawing();
    let (sheets, hash) = analyze(&engine, &bytes);
    assert_eq!(hash.as_str(), TEST_DRAWING_1_SHA256);

    let mut env = FixedEnvironment::new();
    let drawing = import_drawing(
        bytes.clone(),
        "test_drawing_1.pdf",
        &hash,
        &sheets,
        &mut env,
    )
    .unwrap();
    assert_eq!(drawing.revision.sheets.len(), 1);
    assert_eq!(drawing.revision.sheets[0].kind, SheetKind::VectorText);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_drawing_1.dimo");
    let mut session = ProjectSession::create(ProjectInfo::default(), drawing);
    session.save_as(&path, Layout::Zip).unwrap();

    let loaded = ProjectFile::load(&path).unwrap().file;
    let stored = loaded.drawing(&hash).unwrap();
    assert_eq!(stored, bytes.as_slice(), "byte identical to the import");
    // The stored drawing opens again and has the same sheets.
    assert_eq!(analyze(&engine, stored).0, sheets);
}

#[test]
fn bytes_other_than_the_analyzed_ones_are_refused() {
    let Some(engine) = engine() else { return };
    let bytes = corpus_drawing();
    let (sheets, hash) = analyze(&engine, &bytes);
    let mut changed = bytes;
    changed.push(b'\n');
    let error = import_drawing(
        changed,
        "x.pdf",
        &hash,
        &sheets,
        &mut FixedEnvironment::new(),
    )
    .unwrap_err();
    assert!(
        matches!(error, ProjectError::HashMismatch { .. }),
        "{error}"
    );
}
