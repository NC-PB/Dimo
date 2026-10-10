//! Box select on real PDFs (T2.6, FR-REC-01, FR-REC-02, ADR 0006): the corpus drawing and
//! synthetic drawings with rotated callouts.
//!
//! Skips without PDFium like the `dimo-pdf` tests, fails instead with `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1`.

#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::characteristic::{CharacteristicKind, ToleranceRule, Unit};
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::id::SheetId;
use dimo_core::proposal::{BalloonPlacement, Proposal};
use dimo_core::truth::{TruthCharacteristic, TruthFile};
use dimo_core::{Environment as _, FixedEnvironment};
use dimo_detect::{BoxSelectContext, CalloutOnly, box_select};
use dimo_pdf::{Document, PdfEngine, PdfError};
use rust_decimal::Decimal;

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("../..")
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

/// The truth region grown by `margin` on every side, as a user draws it a bit generously.
fn grown(r: &OrientedBox, margin: f64) -> OrientedBox {
    OrientedBox {
        size: Size {
            width: r.size.width + 2.0 * margin,
            height: r.size.height + 2.0 * margin,
        },
        ..*r
    }
}

/// The same area as an axis aligned box on the sheet, as the box tool draws it.
fn axis_aligned(r: &OrientedBox) -> OrientedBox {
    let turned = (r.angle.rem_euclid(180.0) - 90.0).abs() < 1.0;
    let size = if turned {
        Size {
            width: r.size.height,
            height: r.size.width,
        }
    } else {
        r.size
    };
    OrientedBox {
        center: r.center,
        size,
        angle: 0.0,
    }
}

fn select(doc: &Document, sheet: SheetId, region: &OrientedBox) -> Vec<Proposal> {
    let text = doc.region_text(0, *region).unwrap();
    let context = BoxSelectContext {
        interpreter: &CalloutOnly,
        drawing_unit: Unit::Mm,
        placement: BalloonPlacement {
            position: Point {
                x: region.center.x + 30.0,
                y: region.center.y - 30.0,
            },
            anchor: region.center,
        },
        job_id: Some(1),
    };
    box_select(sheet, region, &text, &context)
}

fn sheet_id() -> SheetId {
    SheetId::from_uuid(FixedEnvironment::new().new_uuid())
}

/// Checks one proposal against its truth entry: text, kind, nominal, and limits where the
/// callout writes them (the fallback interpreter does not expand fits or general tolerances).
fn check(p: &Proposal, c: &TruthCharacteristic) {
    assert_eq!(p.requirement_text, c.requirement_text, "{}", c.id);
    assert_eq!(p.kind, c.kind, "{}", c.id);
    if c.kind != CharacteristicKind::Note {
        assert_eq!(p.nominal, c.nominal, "{}", c.id);
        assert!(p.parse_error.is_none(), "{}: {:?}", c.id, p.parse_error);
    }
    if c.tolerance_rule == Some(ToleranceRule::Explicit) {
        assert_eq!(p.upper_limit, c.upper_limit, "{}", c.id);
        assert_eq!(p.lower_limit, c.lower_limit, "{}", c.id);
        let rule = p.derivation.as_ref().and_then(|d| d.rule.kind());
        assert_eq!(rule, Some(ToleranceRule::Explicit), "{}", c.id);
    }
}

/// T2.6 acceptance: box select on `Ø30 H7 +0.0203 -0` gives the printed limits with rule
/// explicit, and every callout of the corpus drawing reads as in its truth file.
#[test]
fn test_drawing_1_callouts() {
    let Some(engine) = engine() else { return };
    let bytes = std::fs::read(repo_root().join("corpus/drawings/test_drawing_1.pdf")).unwrap();
    let doc = engine.open(bytes).unwrap();
    let truth = TruthFile::from_json_str(
        &std::fs::read_to_string(repo_root().join("corpus/truth/test_drawing_1.truth.json"))
            .unwrap(),
    )
    .unwrap();
    let sheet = sheet_id();

    let c03 = truth
        .characteristics
        .iter()
        .find(|c| c.id == "c03")
        .unwrap();
    let proposals = select(&doc, sheet, &grown(&c03.region, 2.0));
    assert_eq!(proposals.len(), 1);
    let p = &proposals[0];
    assert_eq!(p.requirement_text, "Ø30 H7 +0.0203 -0");
    assert_eq!(p.upper_limit, Some(Decimal::new(300_203, 4)));
    assert_eq!(p.lower_limit, Some(Decimal::new(30, 0)));
    assert_eq!(p.fit.as_deref(), Some("H7"));
    assert_eq!(p.unit, Some(Unit::Mm));
    assert_eq!(
        p.derivation.as_ref().and_then(|d| d.rule.kind()),
        Some(ToleranceRule::Explicit)
    );
    assert_eq!(p.source.sheet, sheet);
    assert_eq!(p.job_id, Some(1));

    for c in &truth.characteristics {
        let proposals = select(&doc, sheet, &grown(&c.region, 1.0));
        assert_eq!(proposals.len(), 1, "{}: {proposals:#?}", c.id);
        check(&proposals[0], c);
    }
}

/// T2.6 acceptance: rotated callouts on synthetic drawings are read correctly, with a box in
/// the reading direction and with an axis aligned box as the box tool draws it.
#[test]
fn synthetic_rotated_callouts() {
    let Some(engine) = engine() else { return };
    let sheet = sheet_id();
    let mut rotated = 0;
    for seed in 1..=8 {
        let drawing = dimo_synth::generate(seed, 15).unwrap();
        let doc = engine.open(drawing.pdf.clone()).unwrap();
        for c in &drawing.truth.characteristics {
            for region in [grown(&c.region, 1.5), axis_aligned(&grown(&c.region, 1.5))] {
                let proposals = select(&doc, sheet, &region);
                assert_eq!(proposals.len(), 1, "seed {seed} {}: {proposals:#?}", c.id);
                check(&proposals[0], c);
            }
            if (c.region.angle - 90.0).abs() < 1e-9 {
                rotated += 1;
            }
        }
    }
    assert!(rotated >= 8, "only {rotated} rotated callouts");
}
