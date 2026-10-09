//! T0.11: a generated drawing writes to disk, its truth validates against the committed schema
//! `docs/schema/truth.schema.json` and the `dimo-core` loader, and output is deterministic.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::fs;
use std::path::{Path, PathBuf};

use dimo_core::characteristic::ToleranceRule;
use dimo_core::truth::TruthFile;
use dimo_synth::generate;
use sha2::{Digest, Sha256};

fn schema_path() -> PathBuf {
    Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../docs/schema/truth.schema.json")
}

fn validate(truth_json: &str) {
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(schema_path()).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("urn:dimo:truth.schema.json", schema)
        .unwrap();
    let index = compiler
        .compile("urn:dimo:truth.schema.json", &mut schemas)
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(truth_json).unwrap();
    if let Err(error) = schemas.validate(&value, index) {
        panic!("truth does not match the schema: {error:#}");
    }
}

#[test]
fn generated_files_validate_against_the_schema_and_the_loader() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("synth_validate");
    fs::create_dir_all(&dir).unwrap();
    for seed in [1, 2, 3, 42] {
        let drawing = generate(seed, 15).unwrap();
        let pdf_path = dir.join(format!("{}.pdf", drawing.name));
        let truth_path = dir.join(format!("{}.truth.json", drawing.name));
        fs::write(&pdf_path, &drawing.pdf).unwrap();
        fs::write(&truth_path, &drawing.truth_json).unwrap();

        let truth_text = fs::read_to_string(&truth_path).unwrap();
        validate(&truth_text);
        let loaded = TruthFile::from_json_str(&truth_text).unwrap();
        assert_eq!(loaded, drawing.truth);
        assert_eq!(loaded.characteristics.len(), 15);

        let pdf = fs::read(&pdf_path).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert_eq!(
            format!("{:x}", Sha256::digest(&pdf)),
            loaded.drawing.sha256,
            "truth hash matches the PDF on disk"
        );
    }
}

#[test]
fn same_seed_gives_identical_bytes_and_other_seeds_differ() {
    let a = generate(7, 12).unwrap();
    let b = generate(7, 12).unwrap();
    assert_eq!(a.pdf, b.pdf);
    assert_eq!(a.truth_json, b.truth_json);
    assert_ne!(a.pdf, generate(8, 12).unwrap().pdf);
}

#[test]
fn all_callout_styles_occur() {
    let mut rules = std::collections::BTreeSet::new();
    let mut texts = String::new();
    for seed in 1..=10 {
        for c in generate(seed, 15).unwrap().truth.characteristics {
            rules.insert(format!("{:?}", c.tolerance_rule.unwrap()));
            texts.push_str(&c.requirement_text);
            if c.fit.is_some() {
                assert_eq!(c.tolerance_rule, Some(ToleranceRule::Fit));
                assert!(c.review_note.is_some(), "fit limits are drafts");
            }
        }
    }
    assert_eq!(
        rules.len(),
        3,
        "explicit, fit, no_tolerance_defined: {rules:?}"
    );
    for symbol in ['Ø', '±', '\u{2212}'] {
        assert!(texts.contains(symbol), "{symbol} is generated");
    }
}

#[test]
fn rejects_bad_count() {
    assert!(generate(1, 0).is_err());
    assert!(generate(1, 16).is_err());
}
