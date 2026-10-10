//! Every limit of the corpus truth files is reproduced by parser plus engine (T2.5 acceptance,
//! FR-TOL-01). The drawings declare no general tolerance (notes of `test_drawing_1`, finding 2),
//! so the context is a new project's: no general tolerance, no rules, unit mm.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use common::context::{Cfg, context};
use dimo_core::characteristic::Unit;
use dimo_core::truth::TruthFile;
use dimo_notation::parse_callout;
use dimo_tolerance::interpret;

#[test]
fn truth_limits_are_reproduced() {
    let dir = common::repo_root().join("corpus/truth");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".truth.json"))
        .collect();
    files.sort();
    assert_ne!(files.len(), 0, "no truth files");
    let ctx = context(Cfg::None, Unit::Mm, None);
    let mut checked = 0;
    let mut failures = Vec::new();
    for file in files {
        let truth = TruthFile::from_json_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        for c in truth.characteristics.iter().filter(|c| c.nominal.is_some()) {
            let callout = parse_callout(&c.requirement_text)
                .unwrap_or_else(|e| panic!("{}: {}: {e}", c.id, c.requirement_text));
            let i = interpret(&callout, &ctx);
            let got = (
                i.kind,
                i.nominal,
                i.unit,
                i.fit.clone(),
                i.derivation.rule.kind(),
                i.upper_limit,
                i.lower_limit,
                i.inspect,
            );
            let want = (
                c.kind,
                c.nominal,
                c.unit,
                c.fit.clone(),
                c.tolerance_rule,
                c.upper_limit,
                c.lower_limit,
                c.inspect,
            );
            if got != want {
                failures.push(format!(
                    "{} {}: got {got:?}, want {want:?}",
                    c.id, c.requirement_text
                ));
            }
            checked += 1;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(checked >= 23, "only {checked} characteristics checked");
}
