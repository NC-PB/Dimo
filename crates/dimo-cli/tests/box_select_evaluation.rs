//! M2 exit criterion (T2.9, FR-REC-01, FR-REC-02, FR-REC-08, FR-TOL-01, FR-TOL-02, FR-TOL-04):
//! box select with the real tolerance engine on the corpus and the fixed synthetic set reaches
//! 99 percent correct common callouts. The same run as `dimo-cli eval box-select`.
//!
//! Skips without PDFium like the `dimo-pdf` tests, fails instead with `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1`.

#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_cli::evaluation::{SYNTH_COUNT, SYNTH_SEEDS, run};
use dimo_detect::evaluation::{Category, classify};
use dimo_pdf::{PdfEngine, PdfError};
use dimo_synth::Form;

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

fn expected(form: Form) -> Category {
    match form {
        Form::Linear => Category::LinearGeneral,
        Form::Diameter => Category::DiameterGeneral,
        Form::Radius => Category::RadiusGeneral,
        Form::Angle => Category::AngleGeneral,
        Form::Chamfer => Category::ChamferGeneral,
        Form::Symmetric => Category::Symmetric,
        Form::Asymmetric => Category::Asymmetric,
        Form::OneSided => Category::OneSided,
        Form::Limit => Category::Limit,
        Form::Fit => Category::Fit,
        Form::FitPrinted => Category::FitPrinted,
        Form::Thread => Category::Thread,
        Form::Reference => Category::Reference,
    }
}

/// The classifier puts every generated form into its category, and the fixed synthetic set
/// holds enough cases of every common category for a percentage to mean something.
#[test]
fn synthetic_forms_fall_into_their_categories() {
    let mut counts = std::collections::BTreeMap::new();
    for seed in SYNTH_SEEDS {
        let drawing = dimo_synth::generate(seed, SYNTH_COUNT).unwrap();
        for (form, c) in drawing.forms.iter().zip(&drawing.truth.characteristics) {
            assert_eq!(classify(c), expected(*form), "seed {seed} {}", c.id);
            // T2.9a: every generated angle carries the shorter leg its limits depend on.
            assert_eq!(
                c.shorter_leg.is_some(),
                *form == Form::Angle,
                "seed {seed} {}",
                c.id
            );
            *counts.entry(classify(c)).or_insert(0) += 1;
        }
    }
    for category in Category::ALL.into_iter().filter(|c| c.is_common()) {
        let n = counts.get(&category).copied().unwrap_or(0);
        assert!(n >= 30, "{}: only {n} cases", category.label());
    }
}

/// T2.9 and T2.9a acceptance: at least 99 percent correct limits on common callouts, strict
/// (angles under a general tolerance read with the shorter leg of the truth, as entered on the
/// card). The report is printed with `--nocapture` and on failure.
#[test]
fn box_select_reaches_the_exit_criterion() {
    let Some(engine) = engine() else { return };
    let evaluation = run(&engine, &corpus(), SYNTH_SEEDS).unwrap();
    let report = evaluation.report();
    eprintln!("{report}");
    assert!(evaluation.passes(), "{report}");
    assert!(evaluation.strict().count >= 500, "{report}");
    // Every synthetic angle (40 seeds give 43) was read again with its leg.
    let legs = evaluation.cases.iter().filter(|c| c.leg_entered).count();
    assert!(legs >= 30, "only {legs} legs entered\n{report}");
}
