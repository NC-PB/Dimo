//! Characteristic list exports (T1.4, FR-EXP-09, FR-EXP-11): snapshots and determinism.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_wraps)]

use std::fmt::Write as _;
use std::io::Read;

use dimo_core::Language;
use dimo_core::{
    Balloon, BalloonStyleOverride, CharId, Characteristic, CharacteristicKind,
    CharacteristicStatus, Classification, DisplayNumber, DrawingRevision, Environment,
    FixedEnvironment, OrientedBox, Point, Project, ProjectInfo, RevisionId, Sha256Hex, Sheet,
    SheetId, SheetKind, Size, SourceRegion, TextSource, Timestamp, Unit,
};
use dimo_core::{DerivationHint, DerivationRule, TableRef, ToleranceDerivation};
use dimo_io::export::{COLUMNS, CsvListExporter, ExportOptions, Exporter, XlsxListExporter};
use rust_decimal::Decimal;

fn dec(text: &str) -> Option<Decimal> {
    Some(dimo_core::decimal::parse_decimal(text).expect("test decimal"))
}

/// A project with two sheets and characteristics in a non display order, covering quoting,
/// scales, long decimals, a balloon only sheet link, a rejected one and empty values.
#[allow(clippy::too_many_lines, reason = "one fixture with every column case")]
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
        source: dimo_core::DrawingSource::Pdf,
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

    let new = |env: &mut FixedEnvironment, number: u32| {
        Characteristic::manual(
            CharId::from_uuid(env.new_uuid()),
            DisplayNumber::plain(number),
        )
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
    c2.derivation = Some(ToleranceDerivation::new(DerivationRule::Explicit));

    let mut c1 = new(&mut env, 1);
    c1.kind = CharacteristicKind::Diameter;
    c1.requirement_text = "\u{2300}30 H7".into();
    c1.nominal = dec("30.0");
    c1.upper_limit = dec("30.021");
    c1.lower_limit = dec("30");
    c1.fit = Some("H7".into());
    c1.unit = Some(Unit::Mm);
    c1.sources.push(region(sheet1));
    let mut fit = ToleranceDerivation::new(DerivationRule::Fit {
        table: TableRef {
            id: "iso-286".into(),
            version: 2,
        },
        fit: "H7".into(),
        range: None,
    });
    fit.draft = true;
    c1.derivation = Some(fit);

    // Sheet only known from the balloon.
    let mut c3 = new(&mut env, 3);
    c3.kind = CharacteristicKind::Angle;
    c3.requirement_text = "45\u{b0}".into();
    c3.nominal = dec("45");
    c3.unit = Some(Unit::Deg);
    c3.inspect = false;
    let mut reference = ToleranceDerivation::new(DerivationRule::NoToleranceDefined);
    reference.hints.push(DerivationHint::ReferenceDimension);
    c3.derivation = Some(reference);
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
    c4.derivation = Some(ToleranceDerivation::manual());

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
    let bytes = export(&CsvListExporter, &project(), Language::En);
    insta::assert_snapshot!(String::from_utf8(bytes).unwrap());
}

#[test]
fn csv_german_headers() {
    let bytes = export(&CsvListExporter, &project(), Language::De);
    let text = String::from_utf8(bytes).unwrap();
    insta::assert_snapshot!(text.lines().next().unwrap());
}

#[test]
fn csv_has_one_field_per_column_and_numbers_ascend() {
    let bytes = export(&CsvListExporter, &project(), Language::En);
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
    let bytes = export(&XlsxListExporter, &project(), Language::En);
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
    let bytes = export(&XlsxListExporter, &project(), Language::De);
    let workbook = read_entry(&bytes, "xl/workbook.xml");
    assert!(workbook.contains("name=\"Merkmale\""), "{workbook}");
    let strings = read_entry(&bytes, "xl/sharedStrings.xml");
    assert!(strings.contains("Nennmaß"));
}

/// Committed exports of [`project`], one per exporter and language: file name, XLSX (else
/// CSV), language.
const GOLDEN: &[(&str, bool, Language)] = &[
    ("list_en.csv", false, Language::En),
    ("list_de.csv", false, Language::De),
    ("list_en.xlsx", true, Language::En),
    ("list_de.xlsx", true, Language::De),
];

fn golden_path(name: &str) -> std::path::PathBuf {
    // Resolved at run time so a shared target directory never points to a stale checkout.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    std::path::PathBuf::from(manifest)
        .join("tests/golden")
        .join(name)
}

/// Where a produced file that differs from its golden file is written. CI uploads this
/// directory as an artifact when a job fails, so a mismatch on one platform can be inspected.
fn mismatch_path(name: &str) -> std::path::PathBuf {
    golden_path(name)
        .ancestors()
        .nth(5)
        .expect("workspace root above crates/dimo-io/tests/golden/<name>")
        .join("target/golden-mismatch/dimo-io")
        .join(name)
}

/// First difference between two XLSX files: entry list and order, then per entry the stored
/// metadata (host system, flags, compression, time, attributes) and the content. Falls back to
/// the first differing byte when the entries and their content are equal.
fn zip_difference(expected: &[u8], actual: &[u8]) -> String {
    use dimo_core::zip_host::entry_metadata;
    let (Ok(want), Ok(got)) = (entry_metadata(expected), entry_metadata(actual)) else {
        return "one of the files is not a readable ZIP archive".into();
    };
    let names = |meta: &[dimo_core::zip_host::EntryMetadata]| {
        meta.iter().map(|m| m.name.clone()).collect::<Vec<_>>()
    };
    if names(&want) != names(&got) {
        return format!(
            "entry names or order differ: expected {:?}, got {:?}",
            names(&want),
            names(&got)
        );
    }
    let mut want_zip = entries(expected);
    let mut got_zip = entries(actual);
    for (w, g) in want.iter().zip(&got) {
        let mut fields = Vec::new();
        if w.host() != g.host() {
            fields.push(format!("host system {} != {}", w.host(), g.host()));
        }
        if w.version_made_by != g.version_made_by {
            fields.push(format!(
                "version made by {:#06x} != {:#06x}",
                w.version_made_by, g.version_made_by
            ));
        }
        if w.flags != g.flags {
            fields.push(format!("flags {:#06x} != {:#06x}", w.flags, g.flags));
        }
        if w.method != g.method {
            fields.push(format!("compression {} != {}", w.method, g.method));
        }
        if w.dos_time_date != g.dos_time_date {
            fields.push(format!(
                "time/date {:?} != {:?}",
                w.dos_time_date, g.dos_time_date
            ));
        }
        if w.external_attributes != g.external_attributes {
            fields.push(format!(
                "external attributes {:#010x} != {:#010x}",
                w.external_attributes, g.external_attributes
            ));
        }
        let content = |zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>| {
            let mut file = zip.by_name(&w.name).unwrap();
            let compressed = file.compressed_size();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            (data, compressed)
        };
        let (want_data, want_compressed) = content(&mut want_zip);
        let (got_data, got_compressed) = content(&mut got_zip);
        if want_data != got_data {
            fields.push(format!(
                "uncompressed content differs ({} vs {} bytes)",
                want_data.len(),
                got_data.len()
            ));
        } else if want_compressed != got_compressed {
            fields.push(format!(
                "same content, compressed size {want_compressed} != {got_compressed}"
            ));
        }
        if !fields.is_empty() {
            return format!(
                "first differing entry {}: {} (expected != got)",
                w.name,
                fields.join(", ")
            );
        }
    }
    let at = expected
        .iter()
        .zip(actual)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(actual.len()));
    format!(
        "entries and content equal; first differing byte at offset {at} (lengths {} and {})",
        expected.len(),
        actual.len()
    )
}

/// FR-EXP-11, NFR-PORT-03, rule 11: the exports equal files written at an earlier time on
/// another run and platform, so they depend on neither the clock, the process nor the
/// operating system. Byte for byte, also the ZIP container of the XLSX files. Regenerate after
/// an intended change with `DIMO_UPDATE_GOLDEN=1 cargo test -p dimo-io --test export_list` and
/// review the diff. On a mismatch the produced file is written to
/// `target/golden-mismatch/dimo-io/`.
#[test]
fn exports_match_the_golden_files() {
    let update = std::env::var_os("DIMO_UPDATE_GOLDEN").is_some();
    let mut failures = Vec::new();
    for &(name, xlsx, language) in GOLDEN {
        let exporter: &dyn Exporter = if xlsx {
            &XlsxListExporter
        } else {
            &CsvListExporter
        };
        let bytes = export(exporter, &project(), language);
        assert_eq!(bytes, export(exporter, &project(), language), "{name}");
        let path = golden_path(name);
        if update {
            std::fs::write(&path, &bytes).unwrap();
        }
        let committed = std::fs::read(&path).expect("golden file in tests/golden");
        if committed == bytes {
            continue;
        }
        let out = mismatch_path(name);
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        std::fs::write(&out, &bytes).unwrap();
        let detail = if xlsx {
            zip_difference(&committed, &bytes)
        } else {
            let line = committed
                .split(|&b| b == b'\n')
                .zip(bytes.split(|&b| b == b'\n'))
                .position(|(a, b)| a != b)
                .map_or_else(|| "line count".to_owned(), |i| format!("line {}", i + 1));
            format!("first difference at {line}")
        };
        failures.push(format!(
            "{name} differs from tests/golden/{name}: {detail}. Produced file: {}",
            out.display()
        ));
    }
    assert!(
        failures.is_empty(),
        "{}\nIf the change is intended, regenerate with \
         DIMO_UPDATE_GOLDEN=1 cargo test -p dimo-io --test export_list",
        failures.join("\n")
    );
}

/// FR-EXP-11, NFR-PORT-03: the ZIP metadata of the XLSX list does not depend on the platform
/// the test runs on: host Unix, Unix mode 0o100600 without DOS attribute bits, fixed time and
/// the entry order of the golden file.
#[test]
fn xlsx_zip_metadata_is_platform_independent() {
    for (name, language) in [
        ("list_en.xlsx", Language::En),
        ("list_de.xlsx", Language::De),
    ] {
        let bytes = export(&XlsxListExporter, &project(), language);
        let meta = dimo_core::zip_host::entry_metadata(&bytes).unwrap();
        let golden =
            dimo_core::zip_host::entry_metadata(&std::fs::read(golden_path(name)).unwrap())
                .unwrap();
        let order = |m: &[dimo_core::zip_host::EntryMetadata]| {
            m.iter().map(|e| e.name.clone()).collect::<Vec<_>>()
        };
        assert_eq!(order(&meta), order(&golden), "{name}: entry order");
        for entry in &meta {
            assert_eq!(entry.host(), 3, "{name} {}: host Unix", entry.name);
            assert_eq!(
                entry.external_attributes,
                0o100_600 << 16,
                "{name} {}: Unix mode, no DOS bits",
                entry.name
            );
            // 1980-01-01 00:00:00 in DOS format.
            assert_eq!(entry.dos_time_date, (0, 0x21), "{name} {}", entry.name);
            assert_eq!(entry.method, 8, "{name} {}: deflate", entry.name);
        }
    }
}

#[test]
fn xlsx_zip_entries_have_a_fixed_time() {
    let bytes = export(&XlsxListExporter, &project(), Language::En);
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

/// D-23, FR-BAL-11: sub-numbers and letters export as text in number order; plain numbers stay
/// numbers, so the plain snapshots above do not change.
#[test]
fn structured_numbers_export_as_text_in_number_order() {
    let mut project = project();
    for c in &mut project.characteristics {
        match c.number.as_plain() {
            Some(3) => c.number = DisplayNumber::parse("2A").unwrap(),
            Some(4) => c.number = DisplayNumber::parse("2.1").unwrap(),
            _ => {}
        }
    }
    let bytes = export(&CsvListExporter, &project, Language::En);
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let numbers: Vec<String> = reader.records().map(|r| r.unwrap()[0].to_owned()).collect();
    assert_eq!(numbers, ["1", "2", "2A", "2.1"]);

    let bytes = export(&XlsxListExporter, &project, Language::En);
    let strings = read_entry(&bytes, "xl/sharedStrings.xml");
    assert!(
        strings.contains(">2A<") && strings.contains(">2.1<"),
        "{strings}"
    );
}
