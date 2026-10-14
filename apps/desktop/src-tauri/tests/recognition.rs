//! Box select and typed callouts through the session (T2.6, FR-REC-01, FR-REC-02, ADR 0006,
//! M2 decision 5): proposals change nothing, accepting is one undo step, typed text gives the
//! same values as a box around the printed text.
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

#![allow(clippy::unwrap_used, clippy::print_stderr, clippy::assert_is_empty)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use dimo_core::characteristic::{FieldValue, Origin, ToleranceRule};
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::proposal::BalloonPlacement;
use dimo_core::{Command, Environment as _, SheetId};
use dimo_desktop::ipc::CommandError;
use dimo_desktop::recognition::{
    ExplainLanguage, InterpreterNote, parse_shorter_leg, propose, read_typed,
};
use dimo_desktop::session::AppSession;
use dimo_pdf::tiles::{TileConfig, TileService};
use dimo_pdf::{PdfEngine, PdfError};
use rust_decimal::Decimal;

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn service() -> Option<TileService> {
    match PdfEngine::start() {
        Ok(engine) => Some(TileService::new(engine, TileConfig::default())),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn drawing() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../../corpus/drawings/test_drawing_1.pdf")
}

/// The box around `Ø30 H7 +0.0203 -0` (truth c03, grown by 2 units).
const C03: OrientedBox = OrientedBox {
    center: Point {
        x: 594.3,
        y: 159.07,
    },
    size: Size {
        width: 93.28,
        height: 31.66,
    },
    angle: 0.0,
};

const PLACEMENT: BalloonPlacement = BalloonPlacement {
    position: Point { x: 660.0, y: 120.0 },
    anchor: Point { x: 641.0, y: 143.0 },
};

#[test]
fn box_select_accept_undo_redo() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let target = session.recognition_target(sheet).unwrap();

    let views = propose(&tiles, &target, sheet, C03, PLACEMENT, ExplainLanguage::En).unwrap();
    assert_eq!(views.len(), 1);
    // The tolerance engine explains the limits and finds the printed deviations differ from
    // ISO 286 H7 (T2.5).
    let explanation = views[0].explanation.clone().unwrap_or_default();
    assert!(explanation.contains("H7"), "{explanation}");
    let proposals: Vec<_> = views.into_iter().map(|v| v.proposal).collect();
    let p = &proposals[0];
    assert!(
        p.derivation
            .as_ref()
            .is_some_and(|d| d.hints.iter().any(|h| matches!(
                h,
                dimo_core::derivation::DerivationHint::FitDeviationsDiffer { .. }
            ))),
        "{:?}",
        p.derivation
    );
    assert_eq!(p.engines[1].name, dimo_detect::TOLERANCE_ENGINE);
    assert_eq!(p.requirement_text, "Ø30 H7 +0.0203 -0");
    assert_eq!(p.upper_limit, Some(Decimal::new(300_203, 4)));
    assert_eq!(p.lower_limit, Some(Decimal::new(30, 0)));
    assert_eq!(
        p.derivation.as_ref().and_then(|d| d.rule.kind()),
        Some(ToleranceRule::Explicit)
    );
    assert_eq!(p.placement, PLACEMENT);
    assert!(p.job_id.is_some());
    // ADR 0006: proposing changed nothing.
    assert!(session.project().unwrap().characteristics.is_empty());
    assert!(!session.has_unsaved_changes());

    // Accept is one undo step, undo removes it, redo brings it back.
    session
        .execute(Command::AcceptProposals {
            proposals,
            insert_after: None,
        })
        .unwrap();
    let project = session.project().unwrap();
    assert_eq!(project.characteristics.len(), 1);
    assert_eq!(project.balloons.len(), 1);
    let c = &project.characteristics[0];
    assert_eq!(c.origin, Origin::BoxSelect);
    assert_eq!(c.upper_limit, Some(Decimal::new(300_203, 4)));
    assert_eq!(c.sources[0].region, C03);
    session.undo().unwrap();
    assert!(session.project().unwrap().characteristics.is_empty());
    assert!(session.project().unwrap().balloons.is_empty());
    session.redo().unwrap();
    assert_eq!(session.project().unwrap().characteristics.len(), 1);

    // An empty part of the sheet has no proposals.
    let empty = OrientedBox {
        center: Point { x: 30.0, y: 30.0 },
        ..C03
    };
    assert!(
        propose(
            &tiles,
            &target,
            sheet,
            empty,
            PLACEMENT,
            ExplainLanguage::En
        )
        .unwrap()
        .is_empty()
    );
    // A region without area is refused.
    let flat = OrientedBox {
        size: Size {
            width: 0.0,
            height: 10.0,
        },
        ..C03
    };
    assert!(matches!(
        propose(&tiles, &target, sheet, flat, PLACEMENT, ExplainLanguage::En),
        Err(CommandError::InvalidArgument { .. })
    ));
}

/// Performance test, ignored in the default run (`scripts/perf-tests.sh`): a box select on a
/// loaded page stays below 50 ms.
#[test]
#[ignore = "performance test, run with scripts/perf-tests.sh"]
fn perf_box_select_under_50_ms() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let target = session.recognition_target(sheet).unwrap();

    // Warm: the page is loaded once; then a box select is a region query on the cached page.
    propose(&tiles, &target, sheet, C03, PLACEMENT, ExplainLanguage::En).unwrap();
    let started = Instant::now();
    let views = propose(&tiles, &target, sheet, C03, PLACEMENT, ExplainLanguage::En).unwrap();
    let took = started.elapsed();
    eprintln!("box select on test_drawing_1: {took:?}");
    assert_eq!(views.len(), 1);
    assert!(
        took < Duration::from_millis(50),
        "box select took {took:?}, the direct call needs to stay below 50 ms"
    );
}

/// M2 decision 5: typed text gives the same values as the box selection.
#[test]
fn typed_text_reads_like_box_select() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let target = session.recognition_target(sheet).unwrap();
    let typed = read_typed(&target, "Ø30 H7 +0.0203 -0", None, ExplainLanguage::De);
    assert_eq!(typed.parse_error, None);
    assert!(
        typed.explanation.as_deref().is_some_and(|e| !e.is_empty()),
        "{typed:?}"
    );
    assert!(typed.values.contains(&FieldValue::UpperLimit(
        Some(Decimal::new(300_203, 4)).into()
    )));
    let note = read_typed(&target, "BREAK ALL SHARP EDGES", None, ExplainLanguage::En);
    assert!(note.parse_error.is_some());
    assert_eq!(
        note.values,
        [FieldValue::RequirementText("BREAK ALL SHARP EDGES".into())]
    );

    // A sheet of no open drawing is refused.
    let other = SheetId::from_uuid(dimo_core::FixedEnvironment::new().new_uuid());
    assert!(matches!(
        session.recognition_target(other),
        Err(CommandError::InvalidArgument { .. })
    ));
}

/// T2.9a acceptance: the shorter leg typed for an angle gives the ISO 2768-1 angle limits and an
/// explanation that names the leg and the limits; without it the note asks for the leg.
#[test]
fn shorter_leg_gives_angle_limits() {
    use dimo_core::derivation::TableRef;
    use dimo_core::project::TableClass;
    use dimo_core::{Length, Unit};
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    session
        .execute(Command::SetToleranceSettings {
            settings: dimo_core::ToleranceSettings {
                general: Some(TableClass {
                    table: TableRef {
                        id: "iso-2768-1".into(),
                        version: 1,
                    },
                    class: "m".into(),
                }),
                ..dimo_core::ToleranceSettings::default()
            },
        })
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let target = session.recognition_target(sheet).unwrap();
    assert_eq!(target.unit, Unit::Mm);

    let unknown = read_typed(&target, "30°", None, ExplainLanguage::En);
    assert_eq!(unknown.notes, [InterpreterNote::ShorterLegUnknown]);
    assert!(
        unknown
            .values
            .contains(&FieldValue::UpperLimit(None.into()))
    );

    let leg = parse_shorter_leg(Some(" 40 "), target.unit).unwrap();
    assert_eq!(
        leg,
        Some(Length {
            value: Decimal::from(40),
            unit: Unit::Mm
        })
    );
    let typed = read_typed(&target, "30°", leg, ExplainLanguage::En);
    assert!(typed.notes.is_empty(), "{typed:?}");
    assert!(
        typed
            .values
            .contains(&FieldValue::UpperLimit(Some(Decimal::new(305, 1)).into()))
    );
    assert!(
        typed
            .values
            .contains(&FieldValue::LowerLimit(Some(Decimal::new(295, 1)).into()))
    );
    assert!(typed.values.contains(&FieldValue::ShorterLeg(leg)));
    let explanation = typed.explanation.unwrap_or_default();
    assert!(
        explanation.contains("The shorter leg is 40 mm")
            && explanation.contains("Limits 29.5° to 30.5°"),
        "{explanation}"
    );
    let german = read_typed(&target, "30°", leg, ExplainLanguage::De);
    let explanation = german.explanation.unwrap_or_default();
    assert!(
        explanation.contains("40 mm lang") && explanation.contains("Grenzmaße 29,5° bis 30,5°"),
        "{explanation}"
    );

    // Empty text is no leg; other text is refused.
    assert_eq!(parse_shorter_leg(None, Unit::Mm).unwrap(), None);
    assert_eq!(parse_shorter_leg(Some("  "), Unit::Mm).unwrap(), None);
    assert_eq!(
        parse_shorter_leg(Some("1,5"), Unit::In).unwrap(),
        Some(Length {
            value: Decimal::new(15, 1),
            unit: Unit::In
        })
    );
    for bad in ["0", "-3", "abc", "1/2"] {
        assert!(
            matches!(
                parse_shorter_leg(Some(bad), Unit::Mm),
                Err(CommandError::InvalidArgument { .. })
            ),
            "{bad}"
        );
    }
}
