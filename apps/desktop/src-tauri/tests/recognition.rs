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
use dimo_desktop::recognition::{propose, read_typed};
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

    // Warm: the page is loaded once; then a box select is a region query on the cached page.
    propose(&tiles, &target, sheet, C03, PLACEMENT).unwrap();
    let started = Instant::now();
    let proposals = propose(&tiles, &target, sheet, C03, PLACEMENT).unwrap();
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(50),
        "box select took {took:?}, the direct call needs to stay below 50 ms"
    );
    eprintln!("box select on test_drawing_1: {took:?}");

    assert_eq!(proposals.len(), 1);
    let p = &proposals[0];
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
        propose(&tiles, &target, sheet, empty, PLACEMENT)
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
        propose(&tiles, &target, sheet, flat, PLACEMENT),
        Err(CommandError::InvalidArgument { .. })
    ));
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
    let typed = read_typed(&target, "Ø30 H7 +0.0203 -0");
    assert_eq!(typed.parse_error, None);
    assert!(typed.values.contains(&FieldValue::UpperLimit(
        Some(Decimal::new(300_203, 4)).into()
    )));
    let note = read_typed(&target, "BREAK ALL SHARP EDGES");
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
