//! ISO 286 table: internal consistency, expansion rules and the comparison with the
//! independent fit table of `tools/synth` (T2.3, FR-TOL-04, D-43).
//!
//! The consistency checks catch typing errors in the draft table: several columns of the
//! standard follow from others (delta from the grades, hole deviations from shaft deviations).
//! They do not replace the owner's check against the printed source.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use common::d;
use dimo_tolerance::designation::LETTERS;
use dimo_tolerance::table::TablePart;
use dimo_tolerance::{FitError, Grade, Table, TableSet};
use rust_decimal::Decimal;

fn iso_286() -> Table {
    TableSet::shipped().unwrap().get("iso-286").unwrap().clone()
}

/// Sizes that hit every row of every part: each bound and the middle of each fine step.
fn sizes() -> Vec<Decimal> {
    let bounds = [
        "1", "3", "6", "10", "14", "18", "24", "30", "40", "50", "65", "80", "100", "120", "140",
        "160", "180", "200", "225", "250", "280", "315", "355", "400", "450", "500",
    ];
    let mut out = vec![d("0.5")];
    let mut prev = Decimal::ZERO;
    for b in bounds {
        let b = d(b);
        out.push((prev + b) / Decimal::TWO);
        out.push(b);
        prev = b;
    }
    out
}

fn value(table: &Table, part: &str, column: &str, size: Decimal) -> Option<Decimal> {
    let part: &TablePart = table.part(part).unwrap();
    part.lookup(table.id(), column, size).ok().map(|c| c.value)
}

fn grade(n: i8) -> Grade {
    Grade::it(n).unwrap()
}

#[test]
fn delta_is_the_step_between_neighbouring_grades() {
    // Above 3 mm, delta for ITn equals ITn minus IT(n-1); up to 3 mm it is zero.
    let table = iso_286();
    for size in sizes() {
        for n in 3..=8 {
            let delta = value(&table, "delta", &grade(n).column(), size).unwrap();
            let expected = if size <= d("3") {
                Decimal::ZERO
            } else {
                let it = |g| value(&table, "standard_tolerance", &grade(g).column(), size).unwrap();
                it(n) - it(n - 1)
            };
            assert_eq!(delta, expected, "delta IT{n} at {size}");
        }
    }
}

#[test]
fn standard_tolerances_grow_with_grade_and_size() {
    let table = iso_286();
    let it = |g: Grade, size| value(&table, "standard_tolerance", &g.column(), size);
    let grades: Vec<Grade> = std::iter::once(Grade::parse("01").unwrap())
        .chain((0..=18).map(grade))
        .collect();
    let sizes = sizes();
    for size in &sizes {
        for pair in grades.windows(2) {
            if let (Some(a), Some(b)) = (it(pair[0], *size), it(pair[1], *size)) {
                assert!(a < b, "{} < {} at {size}", pair[0], pair[1]);
            }
        }
    }
    for g in &grades {
        for pair in sizes.windows(2) {
            if let (Some(a), Some(b)) = (it(*g, pair[0]), it(*g, pair[1])) {
                assert!(a <= b, "{g} at {} <= at {}", pair[0], pair[1]);
            }
        }
    }
}

#[test]
fn hole_deviations_mirror_shaft_deviations() {
    let table = iso_286();
    for size in sizes() {
        // A to H: EI = -es.
        for letter in ["A", "B", "C", "D", "E", "F", "G", "H"] {
            let shaft = value(&table, "shaft_upper", &letter.to_lowercase(), size);
            let hole = value(&table, "hole_lower", letter, size);
            assert_eq!(hole, shaft.map(|v| -v), "{letter} at {size}");
        }
        // K, M, N up to IT8 and P to ZC above IT7: ES = -ei (before delta).
        for letter in [
            "K", "M", "N", "P", "R", "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC",
        ] {
            let shaft = value(&table, "shaft_lower", &letter.to_lowercase(), size);
            let hole = value(&table, "hole_upper", letter, size);
            assert_eq!(hole, shaft.map(|v| -v), "{letter} at {size}");
        }
    }
}

#[test]
fn every_class_spans_its_standard_tolerance() {
    // For every letter and grade the zone is IT wide, except js and JS with odd micrometres
    // in IT7 to IT11, which are one micrometre narrower.
    let table = iso_286();
    let micron = d("0.001");
    let mut expanded = 0;
    for size in sizes() {
        for letter in LETTERS {
            for g in std::iter::once("01".to_owned()).chain((0..=18).map(|n| n.to_string())) {
                for text in [letter.to_owned(), letter.to_lowercase()] {
                    let fit = format!("{text}{g}");
                    match table.fit_limits(&fit, size) {
                        Ok(l) => {
                            expanded += 1;
                            let width = l.upper - l.lower;
                            let narrower = letter == "JS" && width == l.standard_tolerance - micron;
                            assert!(
                                width == l.standard_tolerance || narrower,
                                "{fit} at {size}: {} to {}",
                                l.upper,
                                l.lower
                            );
                            assert!(l.draft);
                            assert!(l.range.contains(size));
                        }
                        Err(FitError::Lookup { .. } | FitError::NoDelta { .. }) => {}
                        Err(e) => panic!("{fit}: {e}"),
                    }
                }
            }
        }
    }
    assert!(expanded > 10_000, "only {expanded} classes expanded");
}

#[test]
fn designation_and_size_errors() {
    let table = iso_286();
    assert!(matches!(
        table.fit_limits("Q7", d("10")),
        Err(FitError::Designation(_))
    ));
    assert!(matches!(
        table.fit_limits("H7", d("0")),
        Err(FitError::Lookup { .. })
    ));
    assert!(matches!(
        table.fit_limits("K2", d("18")),
        Err(FitError::NoDelta { .. })
    ));
    let general = TableSet::shipped()
        .unwrap()
        .get("iso-2768-1")
        .unwrap()
        .clone();
    assert!(matches!(
        general.fit_limits("H7", d("10")),
        Err(FitError::NotAFitTable(_))
    ));
}

#[test]
fn limits_and_explanation_data() {
    // FR-TOL-04 with exact decimals: Ø30 H7 is 30.000 to 30.021.
    let table = iso_286();
    let l = table.fit_limits("H7", d("30")).unwrap();
    assert_eq!(l.limits(d("30")), (d("30.021"), d("30")));
    assert_eq!(l.range.to_string(), "over 18 up to and including 30");
    assert_eq!(l.table, "iso-286");
    assert_eq!(l.fundamental_deviation, Some(Decimal::ZERO));
    let p6 = table.fit_limits("P6", d("18")).unwrap();
    assert_eq!(p6.delta, Some(d("0.003")));
    let u6 = table.fit_limits("u6", d("20")).unwrap();
    assert_eq!(u6.range.to_string(), "over 18 up to and including 24");
    // The range is the step of the standard, not the finer row grid of the file.
    let small = table.fit_limits("H7", d("0.5")).unwrap();
    assert_eq!(small.range.to_string(), "up to and including 3");
    let a11 = table.fit_limits("a11", d("2")).unwrap();
    assert_eq!(a11.range.to_string(), "over 1 up to and including 3");
    let m6 = table.fit_limits("M6", d("300")).unwrap();
    assert_eq!((m6.upper, m6.delta), (d("-0.009"), None));
}

/// Parse the `FITS` table of `tools/synth/src/lib.rs` as text, so no code is shared with the
/// generator (it stays an independent truth source): `(nominal, fit, upper µm, lower µm)`.
fn synth_fits() -> Vec<(i64, String, i64, i64)> {
    let text = std::fs::read_to_string(common::repo_root().join("tools/synth/src/lib.rs")).unwrap();
    let start = text.find("const FITS").unwrap();
    let body = &text[start..];
    let body = &body[body.find("= &[").unwrap()..body.find("];").unwrap()];
    body.lines()
        .filter_map(|line| {
            let inner = line.trim().strip_prefix('(')?.strip_suffix("),")?;
            let fields: Vec<&str> = inner.split(',').map(str::trim).collect();
            Some((
                fields[0].parse().unwrap(),
                fields[1].trim_matches('"').to_owned(),
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
            ))
        })
        .collect()
}

#[test]
#[allow(
    clippy::print_stderr,
    reason = "the report is for the owner, the test never fails on it"
)]
fn report_differences_to_synth_fits() {
    // T2.3: list differences between tools/synth FITS and the ISO 286 table for the owner.
    // Differences are reported, never resolved here.
    let table = iso_286();
    let fits = synth_fits();
    assert_eq!(
        fits.len(),
        15,
        "FITS table of tools/synth not found or changed"
    );
    let mut differences = Vec::new();
    for (nominal, fit, upper, lower) in &fits {
        let synth = (Decimal::new(*upper, 3), Decimal::new(*lower, 3));
        match table.fit_limits(fit, Decimal::from(*nominal)) {
            Ok(l) if (l.upper, l.lower) == synth => {}
            Ok(l) => differences.push(format!(
                "{fit} at {nominal}: synth {} / {}, table {} / {}",
                synth.0, synth.1, l.upper, l.lower
            )),
            Err(e) => differences.push(format!("{fit} at {nominal}: table has no value: {e}")),
        }
    }
    if differences.is_empty() {
        eprintln!(
            "tools/synth FITS: all {} entries equal the ISO 286 table",
            fits.len()
        );
    } else {
        eprintln!(
            "tools/synth FITS differs from the ISO 286 table (owner: decide which is right):\n{}",
            differences.join("\n")
        );
    }
}
