//! Fixture projects for report tests (T3.6). Uses `dimo-core` only, so `dimo-cli` tests can
//! include this file with `#[path]` for the PDF text snapshots.

#![allow(dead_code, reason = "each test crate uses a part of the fixtures")]

use dimo_core::{
    AttributeValue, CharId, Characteristic, CharacteristicKind, CharacteristicStatus,
    DerivationHint, DerivationRule, DisplayNumber, DrawingRevision, Environment, FixedEnvironment,
    MeasurementResult, Project, ProjectInfo, RangeBound, ReportInfo, ResultSource, ResultType,
    ResultValue, RevisionId, Sample, SampleId, Sha256Hex, Sheet, SheetId, SheetKind, Size,
    SizeRange, TableLookup, TableRef, Timestamp, ToleranceDerivation, Unit,
};

/// SHA-256 of the fixture drawing (invented).
pub const SHA256: &str = "3f9a1c0e5b7d24a8c6e1f0b9d8a7c6e5f4d3c2b1a09f8e7d6c5b4a3928170615";

fn revision(env: &mut FixedEnvironment) -> DrawingRevision {
    let sheet = SheetId::from_uuid(env.new_uuid());
    let size = Size {
        width: 1191.0,
        height: 842.0,
    };
    DrawingRevision {
        id: RevisionId::from_uuid(env.new_uuid()),
        label: "B".into(),
        file_name: "bracket_rev-B.pdf".into(),
        sha256: Sha256Hex::parse(SHA256).expect("hash"),
        source: dimo_core::DrawingSource::Pdf,
        imported_at: Timestamp::parse("2026-10-01T08:00:00Z").expect("time"),
        sheets: vec![Sheet::new(sheet, 0, size, SheetKind::VectorText)],
    }
}

fn info() -> ProjectInfo {
    ProjectInfo {
        part_number: "BR-2041".into(),
        part_name: "Bracket, left".into(),
        customer: "Example Machines Ltd".into(),
        order_reference: "PO 88-1203".into(),
    }
}

fn report_info() -> ReportInfo {
    ReportInfo {
        report_number: "IR-2026-031".into(),
        report_date: "2026-10-09".into(),
        company_name: "Small Shop GmbH".into(),
        inspected_by: "A. Weber".into(),
        approved_by: "P. Muster".into(),
        remarks: "Parts deburred and cleaned.\nThread checked with a plug gauge.".into(),
    }
}

fn table(id: &str, version: u32) -> TableRef {
    TableRef {
        id: id.into(),
        version,
    }
}

fn range(min: &str, max: &str) -> SizeRange {
    let bound = |v: &str, inclusive| RangeBound {
        value: dimo_core::decimal::parse_decimal(v).expect("bound"),
        inclusive,
    };
    SizeRange {
        min: Some(bound(min, false)),
        max: Some(bound(max, true)),
    }
}

/// The characteristics of the fixture in placement order (not number order): every rule,
/// variable and attribute results, one sided limits, no limits, a reference dimension and a
/// rejected characteristic that the reports leave out (decision 5).
#[allow(clippy::too_many_lines, reason = "one fixture with every report case")]
fn characteristics(env: &mut FixedEnvironment) -> Vec<Characteristic> {
    use CharacteristicKind as K;
    let dec = |t: &str| Some(dimo_core::decimal::parse_decimal(t).expect("test decimal"));
    let mut new = |number: u32, kind, text: &str| {
        let mut c = Characteristic::manual(
            CharId::from_uuid(env.new_uuid()),
            DisplayNumber::plain(number),
        );
        c.kind = kind;
        c.requirement_text = text.into();
        c
    };

    let mut c2 = new(2, K::Linear, "12.5 +0.1/-0.2");
    c2.nominal = dec("12.5");
    c2.upper_dev = dec("0.1");
    c2.lower_dev = dec("-0.2");
    c2.upper_limit = dec("12.6");
    c2.lower_limit = dec("12.3");
    c2.unit = Some(Unit::Mm);
    c2.inspection.method = "Caliper".into();
    c2.derivation = Some(ToleranceDerivation::new(DerivationRule::Explicit));

    let mut c1 = new(1, K::Diameter, "\u{2300}30 H7");
    c1.nominal = dec("30");
    c1.upper_dev = dec("0.021");
    c1.lower_dev = dec("0");
    c1.upper_limit = dec("30.021");
    c1.lower_limit = dec("30.000");
    c1.unit = Some(Unit::Mm);
    c1.fit = Some("H7".into());
    c1.inspection.method = "CMM".into();
    c1.inspection.gauge = "Bore gauge 18-35".into();
    let mut fit = ToleranceDerivation::new(DerivationRule::Fit {
        table: table("iso-286", 1),
        fit: "H7".into(),
        range: Some(range("18", "30")),
    });
    fit.draft = true;
    c1.derivation = Some(fit);

    let mut c3 = new(3, K::Angle, "45\u{b0}");
    c3.nominal = dec("45");
    c3.upper_limit = dec("45.5");
    c3.lower_limit = dec("44.5");
    c3.unit = Some(Unit::Deg);
    c3.derivation = Some(ToleranceDerivation::new(DerivationRule::General {
        lookup: TableLookup {
            table: table("iso-2768-1", 1),
            part: "angular".into(),
            class: "m".into(),
            range: range("10", "50"),
        },
    }));

    let mut c4 = new(4, K::Thread, "2X M8x1.25-6H");
    c4.quantity = 2;
    c4.inspection.gauge = "Thread plug gauge M8".into();
    c4.result_type = ResultType::Attribute;

    let mut c5 = new(5, K::SurfaceTexture, "Ra 1.6");
    c5.upper_limit = dec("1.6");
    c5.unit = Some(Unit::Mm);
    c5.derivation = Some(ToleranceDerivation::new(DerivationRule::Explicit));
    c5.inspection.method = "Roughness tester".into();

    let mut c6 = new(6, K::Linear, "80");
    c6.nominal = dec("80");
    c6.unit = Some(Unit::Mm);
    c6.result_type = ResultType::Variable;
    c6.derivation = Some(ToleranceDerivation::new(DerivationRule::NoToleranceDefined));

    let mut c7 = new(7, K::Linear, "(60)");
    c7.nominal = dec("60");
    c7.unit = Some(Unit::Mm);
    c7.inspect = false;
    let mut reference = ToleranceDerivation::new(DerivationRule::NoToleranceDefined);
    reference.hints.push(DerivationHint::ReferenceDimension);
    c7.derivation = Some(reference);

    let mut c8 = new(8, K::Linear, "20.00");
    c8.nominal = dec("20.00");
    c8.upper_limit = dec("20.05");
    c8.lower_limit = dec("19.95");
    c8.unit = Some(Unit::Mm);
    c8.derivation = Some(ToleranceDerivation::new(DerivationRule::DecimalRule {
        places: 2,
        tolerance: dimo_core::decimal::parse_decimal("0.05").expect("decimal"),
    }));

    let mut c9 = new(9, K::Note, "rejected note");
    c9.status = CharacteristicStatus::Rejected;

    let mut c10 = new(10, K::Linear, "-0.5 offset");
    c10.nominal = dec("-0.5");
    c10.upper_limit = dec("-0.4");
    c10.lower_limit = dec("-0.6");
    c10.unit = Some(Unit::In);
    c10.derivation = Some(ToleranceDerivation::manual());

    vec![c2, c1, c3, c4, c5, c6, c7, c8, c9, c10]
}

fn sample(env: &mut FixedEnvironment, serial: &str, date: &str) -> Sample {
    Sample {
        id: SampleId::from_uuid(env.new_uuid()),
        serial: serial.into(),
        date: date.into(),
        inspector: "A. Weber".into(),
        gauges: "Caliper 150, CMM".into(),
        comment: String::new(),
    }
}

/// The project without samples, for a check sheet with blank columns.
pub fn without_samples() -> Project {
    let mut env = FixedEnvironment::new();
    let mut project = Project::new(info(), revision(&mut env));
    project.characteristics = characteristics(&mut env);
    project.report_info = report_info();
    project
}

/// The project with three samples and results covering pass, fail, not evaluated, missing and
/// attribute results.
pub fn project() -> Project {
    let mut env = FixedEnvironment::new();
    let mut project = Project::new(info(), revision(&mut env));
    project.characteristics = characteristics(&mut env);
    project.report_info = report_info();
    let mut s1 = sample(&mut env, "S-001", "2026-10-08");
    s1.comment = "first part".into();
    let s2 = sample(&mut env, "S-002", "2026-10-08");
    let s3 = sample(&mut env, "S-003", "2026-10-09");

    // Values per sample in placement order of the characteristics (c2, c1, c3, c4, c5, c6, c7,
    // c8, c9, c10); empty means no result.
    let values: [[&str; 10]; 3] = [
        [
            "12.45", "30.010", "45.2", "ok", "0.8", "80.12", "", "20.00", "", "-0.50",
        ],
        [
            "12.61", "30.000", "44.5", "not_ok", "1.6", "79.95", "", "", "", "-0.45",
        ],
        [
            "12.30", "30.021", "", "ok", "1.7", "", "", "19.94", "", "-0.6",
        ],
    ];
    let ids: Vec<CharId> = project.characteristics.iter().map(|c| c.id).collect();
    for (sample, row) in [&s1, &s2, &s3].into_iter().zip(values) {
        for (characteristic, text) in ids.iter().zip(row) {
            let value = match text {
                "" => continue,
                "ok" => ResultValue::Attribute {
                    value: AttributeValue::Ok,
                },
                "not_ok" => ResultValue::Attribute {
                    value: AttributeValue::NotOk,
                },
                number => ResultValue::Variable {
                    value: dimo_core::decimal::parse_decimal(number).expect("value"),
                },
            };
            project.results.push(MeasurementResult {
                sample: sample.id,
                characteristic: *characteristic,
                value,
                source: ResultSource::Manual,
                comment: String::new(),
            });
        }
    }
    project.samples = vec![s1, s2, s3];
    project.check_measurements().expect("valid fixture");
    project
}

/// The fixture with `count` samples (serials `S-001` ...) and every variable result inside the
/// limits, for column blocks.
pub fn with_samples(count: usize) -> Project {
    let mut project = without_samples();
    let mut env = FixedEnvironment::new();
    // Distinct ids from those of the characteristics: skip ahead in the same sequence.
    for _ in 0..100 {
        env.new_uuid();
    }
    project.samples = (1..=count)
        .map(|i| sample(&mut env, &format!("S-{i:03}"), "2026-10-09"))
        .collect();
    project
}

/// A large project: `rows` linear characteristics with `samples` samples, every cell filled.
pub fn large(rows: u32, samples: usize) -> Project {
    let mut env = FixedEnvironment::new();
    let mut project = Project::new(info(), revision(&mut env));
    project.report_info = report_info();
    project.characteristics = (1..=rows)
        .map(|n| {
            let mut c =
                Characteristic::manual(CharId::from_uuid(env.new_uuid()), DisplayNumber::plain(n));
            c.kind = CharacteristicKind::Linear;
            c.requirement_text = format!("{}.5 \u{b1}0.05", 10 + n % 90);
            c.nominal = dimo_core::decimal::parse_decimal(&format!("{}.50", 10 + n % 90));
            c.upper_limit = dimo_core::decimal::parse_decimal(&format!("{}.55", 10 + n % 90));
            c.lower_limit = dimo_core::decimal::parse_decimal(&format!("{}.45", 10 + n % 90));
            c.unit = Some(Unit::Mm);
            c.derivation = Some(ToleranceDerivation::new(DerivationRule::Explicit));
            c
        })
        .collect();
    project.samples = (1..=samples)
        .map(|i| sample(&mut env, &format!("S-{i:03}"), "2026-10-09"))
        .collect();
    for s in &project.samples {
        for (i, c) in project.characteristics.iter().enumerate() {
            let n = 10 + (u32::try_from(i).expect("row index") + 1) % 90;
            let last = if i % 37 == 0 { "58" } else { "51" };
            project.results.push(MeasurementResult {
                sample: s.id,
                characteristic: c.id,
                value: ResultValue::Variable {
                    value: dimo_core::decimal::parse_decimal(&format!("{n}.{last}"))
                        .expect("value"),
                },
                source: ResultSource::Manual,
                comment: String::new(),
            });
        }
    }
    project
}
