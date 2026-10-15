//! Tolerance settings, explanations, re-interpretation, custom table import and history behind
//! the IPC commands (T2.8, FR-TOL-07, FR-TOL-08, FR-CHR-10).
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::Language;
use dimo_core::{
    ChangeSource, CharId, Command, DerivationRule, FieldValue, Point, SheetField, TableClass,
    TableRef, ToleranceDerivation,
};
use dimo_desktop::ipc::{ArgumentReason, CommandError, TableImportReason};
use dimo_desktop::session::AppSession;
use dimo_desktop::tolerance::{self, TableImport, TableUse};
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

const SHOP_TABLE: &str = r#"[table]
id = "shop"
title = "Shop table"
kind = "custom"
unit = "mm"
version = 1
status = "draft"
source = "test fixture"

[[part]]
id = "linear"
source = "fixture"
kind = "symmetric"
applies_to = "linear"
value_unit = "mm"
columns = ["a"]
rows = [
  { min_inclusive = "0.5", max_inclusive = "100", values = ["0.02"] },
]
"#;

fn d(text: &str) -> Option<Decimal> {
    text.parse().ok()
}

/// Adds a characteristic with only its requirement text, as typed before T2.6.
fn add(session: &mut AppSession, text: &str) -> CharId {
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let patched = session
        .execute(Command::AddCharacteristic {
            sheet,
            position: Point { x: 200.0, y: 150.0 },
            anchor: Point { x: 237.5, y: 166.1 },
            region: None,
            values: vec![FieldValue::RequirementText(text.into())],
            insert_after: None,
        })
        .unwrap();
    session
        .project()
        .unwrap()
        .characteristics
        .iter()
        .find(|c| c.requirement_text == text)
        .map_or_else(|| panic!("{patched:?}"), |c| c.id)
}

fn limits(session: &AppSession, id: CharId) -> (Option<Decimal>, Option<Decimal>) {
    let c = session.project().unwrap().characteristic(id).unwrap();
    (c.upper_limit, c.lower_limit)
}

fn general(class: &str) -> Command {
    Command::SetToleranceSettings {
        settings: dimo_core::ToleranceSettings {
            general: Some(TableClass {
                table: TableRef {
                    id: "iso-2768-1".into(),
                    version: 1,
                },
                class: class.into(),
            }),
            ..dimo_core::ToleranceSettings::default()
        },
    }
}

// T2.8 acceptance: changing the general tolerance and re-interpreting updates limits and
// explanations, in one undo step; hand edited limits stay.
#[test]
fn reinterpret_after_a_settings_change() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let plain = add(&mut session, "50");
    let manual = add(&mut session, "60");
    session
        .execute(Command::UpdateFields {
            ids: vec![manual],
            values: vec![FieldValue::UpperLimit(d("60.5").into())],
        })
        .unwrap();
    assert_eq!(
        session
            .project()
            .unwrap()
            .characteristic(manual)
            .unwrap()
            .derivation,
        Some(ToleranceDerivation::manual())
    );

    // The shipped tables with their classes.
    let tables = tolerance::tables(&session).unwrap();
    let iso = tables.iter().find(|t| t.table.id == "iso-2768-1").unwrap();
    assert_eq!(iso.usage, TableUse::General);
    assert_eq!(iso.classes, ["f", "m", "c", "v"]);
    assert!(iso.draft);
    assert!(tables.iter().any(|t| t.usage == TableUse::Fit));

    session.execute(general("m")).unwrap();
    let result = tolerance::reinterpret(&mut session, &[plain, manual]).unwrap();
    assert_eq!(
        (
            result.reinterpreted,
            result.skipped_manual,
            result.skipped_unreadable
        ),
        (1, 1, 0)
    );
    assert_eq!(limits(&session, plain), (d("50.3"), d("49.7")));
    assert_eq!(limits(&session, manual).0, d("60.5"));
    let english = tolerance::explanation(&session, plain, Language::En)
        .unwrap()
        .unwrap();
    assert!(english.contains("class m"), "{english}");
    let german = tolerance::explanation(&session, plain, Language::De)
        .unwrap()
        .unwrap();
    assert_ne!(english, german);

    session.execute(general("f")).unwrap();
    // Settings alone do not change limits.
    assert_eq!(limits(&session, plain), (d("50.3"), d("49.7")));
    tolerance::reinterpret(&mut session, &[plain]).unwrap();
    assert_eq!(limits(&session, plain), (d("50.15"), d("49.85")));
    let english = tolerance::explanation(&session, plain, Language::En)
        .unwrap()
        .unwrap();
    assert!(english.contains("class f"), "{english}");
    session.undo().unwrap();
    assert_eq!(limits(&session, plain), (d("50.3"), d("49.7")));

    // FR-CHR-10: the history tells a rule change from a manual one.
    let history = session.characteristic_history(plain).unwrap();
    let sources: Vec<ChangeSource> = history.iter().map(|h| h.source).collect();
    assert_eq!(sources.first(), Some(&ChangeSource::Manual));
    assert!(sources.contains(&ChangeSource::Rule), "{sources:?}");
    let history = session.characteristic_history(manual).unwrap();
    assert!(history.iter().all(|h| h.source == ChangeSource::Manual));
}

/// Adds a characteristic read from typed text with the current settings, as the app does.
fn typed(session: &mut AppSession, text: &str) -> CharId {
    use dimo_desktop::recognition::read_typed;
    let id = add(session, text);
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let target = session.recognition_target(sheet).unwrap();
    let reading = read_typed(&target, text, None, Language::En);
    session
        .execute(Command::UpdateFields {
            ids: vec![id],
            values: reading.values,
        })
        .unwrap();
    id
}

// T3.13, M3 decision 22: after the general tolerance changed, the update gives characteristics
// without tolerance or with a general tolerance the new one, in one undo step; limits entered
// by hand, explicit tolerances and unreadable text stay.
#[test]
fn general_tolerance_update_after_a_settings_change() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let plain = typed(&mut session, "50");
    let explicit = typed(&mut session, "Ø20 +0.1 -0");
    let thread = typed(&mut session, "M8");
    let unreadable = typed(&mut session, "SEE NOTE");
    let manual = typed(&mut session, "60");
    session
        .execute(Command::UpdateFields {
            ids: vec![manual],
            values: vec![FieldValue::UpperLimit(d("60.5").into())],
        })
        .unwrap();
    assert_eq!(limits(&session, plain), (None, None));
    // No general tolerance yet: nothing to update.
    assert_eq!(tolerance::general_update_candidates(&session).unwrap(), []);

    session.execute(general("m")).unwrap();
    assert_eq!(
        tolerance::general_update_candidates(&session).unwrap(),
        [plain]
    );
    let before = session.project().unwrap().clone();
    let result = tolerance::apply_general(&mut session).unwrap();
    assert_eq!(result.reinterpreted, 1);
    assert_eq!(limits(&session, plain), (d("50.3"), d("49.7")));
    for id in [explicit, thread, unreadable, manual] {
        assert_eq!(
            session.project().unwrap().characteristic(id),
            before.characteristic(id)
        );
    }
    assert_eq!(tolerance::general_update_candidates(&session).unwrap(), []);

    // A new class updates the general tolerance again; one undo restores the old limits.
    session.execute(general("f")).unwrap();
    assert_eq!(
        tolerance::general_update_candidates(&session).unwrap(),
        [plain]
    );
    tolerance::apply_general(&mut session).unwrap();
    assert_eq!(limits(&session, plain), (d("50.15"), d("49.85")));
    assert_eq!(limits(&session, manual).0, d("60.5"));
    session.undo().unwrap();
    assert_eq!(limits(&session, plain), (d("50.3"), d("49.7")));
    // After the undo the offer applies again.
    assert_eq!(
        tolerance::general_update_candidates(&session).unwrap(),
        [plain]
    );
}

// FR-TOL-07: import of a custom table, errors with the line, crash before save (M2 decision 4).
#[test]
fn custom_table_import_survives_a_crash() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let autosave = dir.path().join("autosave");
    let file = dir.path().join("shop.toml");
    let broken = dir.path().join("broken.toml");
    std::fs::write(&file, SHOP_TABLE).unwrap();
    std::fs::write(&broken, SHOP_TABLE.replace("version = 1", "version = one")).unwrap();
    let expected = {
        let mut session = AppSession::new(Some(autosave.clone()));
        session
            .create_from_drawing(&tiles, &drawing(), false)
            .unwrap();
        let TableImport::Invalid {
            reason,
            line,
            message,
        } = tolerance::import(&mut session, &broken).unwrap()
        else {
            panic!("the broken table was imported");
        };
        assert_eq!(line, Some(6), "{message}");
        assert_eq!(reason, TableImportReason::Syntax, "{message}");
        assert!(message.contains("broken.toml"), "{message}");
        assert!(!session.has_unsaved_changes());

        let TableImport::Imported { table, patched } =
            tolerance::import(&mut session, &file).unwrap()
        else {
            panic!("the table was refused");
        };
        assert_eq!(table.id, "shop");
        assert!(!patched.patch.is_empty());
        // The same file again changes nothing and makes no undo step.
        let before = session.project().unwrap().clone();
        assert_eq!(
            tolerance::import(&mut session, &file).unwrap(),
            TableImport::Unchanged {
                table: table.clone()
            }
        );
        assert_eq!(*session.project().unwrap(), before);
        assert!(
            tolerance::tables(&session)
                .unwrap()
                .iter()
                .any(|t| t.table.id == "shop" && t.usage == TableUse::Custom && t.classes == ["a"])
        );
        let mut settings = session.project().unwrap().settings.tolerance.clone();
        settings.drawing_rule = Some(TableClass {
            table: table.clone(),
            class: "a".into(),
        });
        session
            .execute(Command::SetToleranceSettings { settings })
            .unwrap();
        let id = add(&mut session, "50");
        tolerance::reinterpret(&mut session, &[id]).unwrap();
        let c = session.project().unwrap().characteristic(id).unwrap();
        assert!(matches!(
            c.derivation.as_ref().map(|d| &d.rule),
            Some(DerivationRule::DrawingRule { .. })
        ));
        assert_eq!(limits(&session, id), (d("50.02"), d("49.98")));
        session.project().unwrap().clone()
        // Crash: the session is dropped without saving.
    };

    let mut session = AppSession::new(Some(autosave));
    assert!(session.recover_unsaved(&tiles).unwrap());
    assert_eq!(*session.project().unwrap(), expected);
    let custom = session.custom_tables().unwrap();
    assert_eq!(custom, [("shop".to_owned(), SHOP_TABLE.to_owned())]);
    // The restored project can be saved: the table is there.
    session.save_as(&dir.path().join("saved.dimo")).unwrap();
}

#[test]
fn commands_need_a_project() {
    let mut session = AppSession::new(None);
    assert_eq!(tolerance::tables(&session), Err(CommandError::NoProject));
    assert!(matches!(
        tolerance::reinterpret(&mut session, &[]),
        Err(CommandError::NoProject)
    ));
    assert_eq!(
        tolerance::general_update_candidates(&session),
        Err(CommandError::NoProject)
    );
    assert!(matches!(
        tolerance::apply_general(&mut session),
        Err(CommandError::NoProject)
    ));
    assert!(matches!(
        session.characteristic_history(CharId::from_uuid(uuid::Uuid::nil())),
        Err(CommandError::NoProject)
    ));
    assert_eq!(
        tolerance::leg_unit(&session, CharId::from_uuid(uuid::Uuid::nil())),
        Err(CommandError::NoProject)
    );
}

/// MF F8: the detail panel labels the shorter leg field with the unit Rust names, the unit of
/// the characteristic's sheet (D-20), so the frontend never works out which sheet that is.
#[test]
fn shorter_leg_unit_follows_the_sheet_of_the_characteristic() {
    use dimo_core::Unit;
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let id = add(&mut session, "30°");
    assert_eq!(tolerance::leg_unit(&session, id), Ok(Unit::Mm));
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    session
        .execute(Command::UpdateSheet {
            sheet,
            values: vec![SheetField::Unit(Unit::In)],
        })
        .unwrap();
    assert_eq!(tolerance::leg_unit(&session, id), Ok(Unit::In));
    assert!(matches!(
        tolerance::leg_unit(&session, CharId::from_uuid(uuid::Uuid::nil())),
        Err(e) if e != CommandError::NoProject
    ));
}

/// T2.9a: the shorter leg set from the detail panel is parsed in the sheet unit and stored with
/// its unit. Regression: after the sheet unit changes, retyping the requirement text (the stored
/// leg is sent as `LegInput::Stored`) and re-interpreting keep the leg and the limits.
#[test]
fn shorter_leg_keeps_its_unit_when_the_sheet_unit_changes() {
    use dimo_core::{Length, Unit};
    use dimo_desktop::recognition::{LegInput, read_typed, resolve_leg};
    use dimo_desktop::tolerance::LegOutcome;
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    session.execute(general("m")).unwrap();
    let id = add(&mut session, "30°");
    assert_eq!(limits(&session, id), (None, None));

    let set = tolerance::apply_shorter_leg(&mut session, id, Some("40")).unwrap();
    assert_eq!(set.outcome, LegOutcome::Interpreted);
    let mm = Length {
        value: Decimal::from(40),
        unit: Unit::Mm,
    };
    let leg = |s: &AppSession| s.project().unwrap().characteristic(id).unwrap().shorter_leg;
    assert_eq!(leg(&session), Some(mm));
    assert_eq!(limits(&session, id), (d("30.5"), d("29.5")));

    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    session
        .execute(Command::UpdateSheet {
            sheet,
            values: vec![SheetField::Unit(Unit::In)],
        })
        .unwrap();
    // Retyping the text sends the stored leg; it keeps its unit on the inch sheet.
    let target = session.recognition_target(sheet).unwrap();
    assert_eq!(target.unit, Unit::In);
    let stored = resolve_leg(Some(&LegInput::Stored(mm)), target.unit).unwrap();
    let typed = read_typed(&target, "30°", stored, Language::En);
    session
        .execute(Command::UpdateFields {
            ids: vec![id],
            values: typed.values,
        })
        .unwrap();
    assert_eq!(leg(&session), Some(mm));
    assert_eq!(limits(&session, id), (d("30.5"), d("29.5")));
    // Re-interpretation uses the stored leg too.
    tolerance::reinterpret(&mut session, &[id]).unwrap();
    assert_eq!(leg(&session), Some(mm));
    assert_eq!(limits(&session, id), (d("30.5"), d("29.5")));

    // A newly typed leg is in the sheet unit: 0.3 in is 7.62 mm, so ±1°.
    tolerance::apply_shorter_leg(&mut session, id, Some("0,3")).unwrap();
    assert_eq!(
        leg(&session),
        Some(Length {
            value: Decimal::new(3, 1),
            unit: Unit::In
        })
    );
    assert_eq!(limits(&session, id), (d("31"), d("29")));
    // Text that is not a positive number is refused.
    assert!(matches!(
        tolerance::apply_shorter_leg(&mut session, id, Some("abc")),
        Err(CommandError::InvalidArgument {
            reason: ArgumentReason::ShorterLeg,
            ..
        })
    ));
    // Hand entered limits stay; only the leg changes.
    session
        .execute(Command::UpdateFields {
            ids: vec![id],
            values: vec![FieldValue::UpperLimit(d("30.2").into())],
        })
        .unwrap();
    let set = tolerance::apply_shorter_leg(&mut session, id, None).unwrap();
    assert_eq!(set.outcome, LegOutcome::ManualKept);
    assert_eq!(leg(&session), None);
    assert_eq!(limits(&session, id), (d("30.2"), d("29")));
}
