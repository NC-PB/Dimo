//! Characteristic list exports (T1.4, FR-EXP-09, FR-EXP-11): snapshots and determinism.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_wraps)]

use std::fmt::Write as _;
use std::io::Read;

use dimo_core::{
    Balloon, BalloonStyleOverride, CharId, Characteristic, CharacteristicKind,
    CharacteristicStatus, Classification, DrawingRevision, Environment, FixedEnvironment,
    OrientedBox, Point, Project, ProjectInfo, RevisionId, Sha256Hex, Sheet, SheetId, SheetKind,
    Size, SourceRegion, TextSource, Timestamp, Unit,
};
use dimo_io::export::{
    COLUMNS, CsvListExporter, ExportOptions, Exporter, Language, XlsxListExporter,
};
use rust_decimal::Decimal;

fn dec(text: &str) -> Option<Decimal> {
    Some(dimo_core::decimal::parse_decimal(text).expect("test decimal"))
}

/// A project with two sheets and characteristics in a non display order, covering quoting,
/// scales, long decimals, a balloon only sheet link, a rejected one and empty values.
fn project() -> Project {
    let mut env = FixedEnvironment::new();
    let sheet1 = SheetId::from_uuid(env.new_uuid());
    let sheet2 = SheetId::from_uuid(env.new_uuid());
    let size = Size {
        width: 842.0,
        height: 595.0,
    };
    let mut revision = DrawingRevision {
        id: RevisionId::from_uuid(env.new_uuid()),
        label: "A".into(),
        file_name: "bracket.pdf".into(),
        sha256: Sha256Hex::parse(&"0".repeat(64)).unwrap(),
        imported_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
        sheets: Vec::new(),
    };
    revision
        .sheets
        .push(Sheet::new(sheet1, 0, size, SheetKind::VectorText));
    revision
        .sheets
        .push(Sheet::new(sheet2, 1, size, SheetKind::VectorText));
    let mut project = Project::new(
        ProjectInfo {
            part_number: "P-100".into(),
            ..ProjectInfo::default()
        },
        revision,
    );

    let new = |env: &mut FixedEnvironment, number| {
        Characteristic::manual(CharId::from_uuid(env.new_uuid()), number)
    };
    let region = |sheet| SourceRegion {
        sheet,
        region: OrientedBox {
            center: Point { x: 10.0, y: 10.0 },
            size: Size {
                width: 20.0,
                height: 5.0,
            },
            angle: 0.0,
        },
        text_source: TextSource::PdfText,
        raw_text: None,
    };

    let mut c2 = new(&mut env, 2);
    c2.kind = CharacteristicKind::Linear;
    c2.requirement_text = "12.5 +0.1/-0.2, \"ream\"\nbore".into();
    c2.nominal = dec("12.5");
    c2.upper_dev = dec("0.1");
    c2.lower_dev = dec("-0.2");
    c2.upper_limit = dec("12.6");
    c2.lower_limit = dec("12.3");
    c2.unit = Some(Unit::Mm);
    c2.quantity = 4;
    c2.classification = Classification::Critical;
    c2.inspection.method = "CMM".into();
    c2.inspection.gauge = "Caliper, digital".into();
    c2.inspection.sampling = "100%".into();
    c2.comment = "check after coating; see note 3".into();
    c2.sources.push(region(sheet2));

    let mut c1 = new(&mut env, 1);
    c1.kind = CharacteristicKind::Diameter;
    c1.requirement_text = "\u{2300}30 H7".into();
    c1.nominal = dec("30.0");
    c1.upper_limit = dec("30.021");
    c1.lower_limit = dec("30");
    c1.fit = Some("H7".into());
    c1.unit = Some(Unit::Mm);
    c1.sources.push(region(sheet1));

    // Sheet only known from the balloon.
    let mut c3 = new(&mut env, 3);
    c3.kind = CharacteristicKind::Angle;
    c3.requirement_text = "45\u{b0}".into();
    c3.nominal = dec("45");
    c3.unit = Some(Unit::Deg);
    c3.inspect = false;
    let balloon = Balloon {
        id: dimo_core::BalloonId::from_uuid(env.new_uuid()),
        characteristic: c3.id,
        sheet: sheet2,
        position: Point { x: 1.0, y: 1.0 },
        anchor: Point { x: 2.0, y: 2.0 },
        style: BalloonStyleOverride::default(),
    };

    // More digits than a double holds.
    let mut c4 = new(&mut env, 4);
    c4.kind = CharacteristicKind::Linear;
    c4.nominal = dec("123456.7890123456789");
    c4.upper_limit = dec("0.000000000000001");
    c4.status = CharacteristicStatus::Proposed;

    let mut rejected = new(&mut env, 5);
    rejected.status = CharacteristicStatus::Rejected;
    rejected.requirement_text = "not exported".into();

    project.characteristics = vec![c2, c1, c3, c4, rejected];
    project.balloons.push(balloon);
    project
}

fn export(exporter: &dyn Exporter, project: &Project, language: Language) -> Vec<u8> {
    let mut out = Vec::new();
    exporter
        .export(project, &ExportOptions { language }, &mut out)
        .unwrap();
    out
}

fn entries(bytes: &[u8]) -> zip::ZipArchive<std::io::Cursor<&[u8]>> {
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap()
}

fn read_entry(bytes: &[u8], name: &str) -> String {
    let mut text = String::new();
    entries(bytes)
        .by_name(name)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

#[test]
fn csv_english() {
    let bytes = export(&CsvListExporter, &project(), Language::English);
    insta::assert_snapshot!(String::from_utf8(bytes).unwrap());
}

#[test]
fn csv_german_headers() {
    let bytes = export(&CsvListExporter, &project(), Language::German);
    let text = String::from_utf8(bytes).unwrap();
    insta::assert_snapshot!(text.lines().next().unwrap());
}

#[test]
fn csv_has_one_field_per_column_and_numbers_ascend() {
    let bytes = export(&CsvListExporter, &project(), Language::English);
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let mut numbers = Vec::new();
    for record in reader.records() {
        let record = record.unwrap();
        assert_eq!(record.len(), COLUMNS.len());
        numbers.push(record[0].parse::<u32>().unwrap());
    }
    assert_eq!(numbers, [1, 2, 3, 4]);
}

#[test]
fn xlsx_sheet_content() {
    let bytes = export(&XlsxListExporter, &project(), Language::English);
    let mut snapshot = String::new();
    for name in [
        "xl/workbook.xml",
        "xl/styles.xml",
        "xl/worksheets/sheet1.xml",
        "xl/sharedStrings.xml",
        "docProps/core.xml",
    ] {
        let _ = writeln!(snapshot, "=== {name}\n{}", read_entry(&bytes, name));
    }
    insta::assert_snapshot!(snapshot);
}

#[test]
fn xlsx_german_sheet_name() {
    let bytes = export(&XlsxListExporter, &project(), Language::German);
    let workbook = read_entry(&bytes, "xl/workbook.xml");
    assert!(workbook.contains("name=\"Merkmale\""), "{workbook}");
    let strings = read_entry(&bytes, "xl/sharedStrings.xml");
    assert!(strings.contains("Nennmaß"));
}

#[test]
fn exports_are_byte_identical() {
    for exporter in [
        &CsvListExporter as &dyn Exporter,
        &XlsxListExporter as &dyn Exporter,
    ] {
        for language in [Language::English, Language::German] {
            let a = export(exporter, &project(), language);
            // Let the wall clock move on, a clock dependent writer would differ.
            std::thread::sleep(std::time::Duration::from_millis(2100));
            let b = export(exporter, &project(), language);
            assert_eq!(a, b, "{} {language:?}", exporter.file_extension());
        }
    }
}

#[test]
fn xlsx_zip_entries_have_a_fixed_time() {
    let bytes = export(&XlsxListExporter, &project(), Language::English);
    let mut archive = entries(&bytes);
    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        let time = file.last_modified().expect("entry time");
        assert_eq!(
            (time.year(), time.month(), time.day()),
            (1980, 1, 1),
            "{}",
            file.name()
        );
        assert_eq!((time.hour(), time.minute(), time.second()), (0, 0, 0));
    }
}
