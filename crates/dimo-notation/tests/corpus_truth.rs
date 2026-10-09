//! Every requirement text in `corpus/truth/*.truth.json` parses to the expected kind,
//! nominal, unit, fit and tolerance parts (T2.1 acceptance, FR-REC-08).
//!
//! Truth entries without a nominal (notes) are free text and must not parse as a callout.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::fs;
use std::path::{Path, PathBuf};

use dimo_core::characteristic::{ToleranceRule, Unit};
use dimo_core::truth::TruthFile;
use dimo_notation::{Tolerance, parse_callout};

fn truth_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/truth");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".truth.json"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_truth_requirement_text_parses() {
    let files = truth_files();
    assert!(!files.is_empty(), "no truth files found");
    let mut checked = 0;
    for path in files {
        let truth = TruthFile::from_json_str(&fs::read_to_string(&path).unwrap()).unwrap();
        for c in &truth.characteristics {
            let text = &c.requirement_text;
            let context = format!("{} {} {text:?}", path.display(), c.id);
            let Some(nominal) = c.nominal else {
                assert!(
                    parse_callout(text).is_err(),
                    "{context}: note parsed as callout"
                );
                continue;
            };
            let callout = parse_callout(text).unwrap_or_else(|e| panic!("{context}: {e}"));
            assert_eq!(callout.characteristic_kind(), c.kind, "{context}: kind");
            assert_eq!(callout.nominal.value, nominal, "{context}: nominal");
            assert_eq!(
                callout.nominal.value.scale(),
                nominal.scale(),
                "{context}: decimal places"
            );
            // Lengths without a marker take the drawing unit; angles always carry degrees.
            match callout.unit {
                Some(unit) => assert_eq!(Some(unit), c.unit, "{context}: unit"),
                None => assert_ne!(c.unit, Some(Unit::Deg), "{context}: unit"),
            }
            assert_eq!(
                callout.fit.as_ref().map(ToString::to_string),
                c.fit,
                "{context}: fit"
            );
            if c.tolerance_rule == Some(ToleranceRule::Explicit) {
                let tolerance = callout.tolerance.as_ref();
                assert!(tolerance.is_some(), "{context}: explicit tolerance missing");
                // Where the truth gives limits, the written values reproduce them.
                match (tolerance, c.upper_limit, c.lower_limit) {
                    (Some(Tolerance::Deviations { upper, lower }), Some(u), Some(l)) => {
                        assert_eq!(nominal + upper.value, u, "{context}: upper limit");
                        assert_eq!(nominal + lower.value, l, "{context}: lower limit");
                    }
                    (Some(Tolerance::Symmetric(t)), Some(u), Some(l)) => {
                        assert_eq!(nominal + t.value, u, "{context}: upper limit");
                        assert_eq!(nominal - t.value, l, "{context}: lower limit");
                    }
                    _ => {}
                }
            } else {
                assert_eq!(callout.tolerance, None, "{context}: tolerance");
            }
            assert_eq!(
                callout.reference || callout.basic,
                !c.inspect,
                "{context}: inspect"
            );
            checked += 1;
        }
    }
    assert!(checked >= 23, "only {checked} callouts checked");
}
