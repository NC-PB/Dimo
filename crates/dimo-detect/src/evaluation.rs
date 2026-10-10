//! Box select evaluation (T2.9, M2 exit criterion, spec 08 Evaluation, D-42).
//!
//! Box select runs with the real tolerance engine on the region of every truth characteristic
//! and the proposal is compared with the truth: exactly one proposal, kind, nominal, upper and
//! lower limit and the inspect flag. The results are counted per [`Category`].
//!
//! # Common callouts
//!
//! The M2 exit criterion asks for correct limits on 99 percent of the "common callouts"
//! (docs/plan/M2.md T2.9). They are the categories with [`Category::is_common`]: linear,
//! diameter, radius, angle and chamfer sizes without a tolerance (general rule, or no limits
//! when the project has none), symmetric, asymmetric, one sided and limit tolerances, fits with
//! and without printed deviations, metric threads and reference dimensions. Decimal comma,
//! rotation and stacked text are [`Traits`] across the categories and are reported as extra
//! rows.
//!
//! An angle whose truth has limits from a general tolerance is a **known gap**: ISO 2768-1
//! selects the angular tolerance by the shorter leg of the angle, which is not written on the
//! sheet and which box select does not measure in M2 (decision log 2026-10-10, T2.5). The gate
//! checks such a case against what M2 can do: one proposal with kind, nominal and inspect flag
//! right, rule `no_tolerance_defined`, no limits, and the note that the shorter leg is unknown.
//! The strict rate, which wants the limits, is reported next to it.
//!
//! The category is derived from the truth fields and the requirement text, never from the
//! proposal, so a misread callout still counts in the category of what is printed.

use std::fmt::Write as _;

use dimo_core::characteristic::{CharacteristicKind, ToleranceRule, Unit};
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::id::SheetId;
use dimo_core::proposal::{BalloonPlacement, Proposal};
use dimo_core::truth::{TruthCharacteristic, TruthFile};
use dimo_core::{Environment as _, FixedEnvironment};
use dimo_pdf::Document;
use dimo_tolerance::{ContextError, TableSet, ToleranceContext};
use rust_decimal::Decimal;

use crate::box_select::{BoxSelectContext, Proposed, box_select_with_notes};
use crate::interpret::{Interpreter, ToleranceEngine};

/// Share of correct common callouts (known gaps left out) the M2 exit criterion asks for, in
/// percent.
pub const REQUIRED_PERCENT: u32 = 99;

/// How much the box around a truth region is grown on every side, in PDF user units: a user
/// draws the box a bit generously.
pub const BOX_MARGIN: f64 = 1.5;

/// The callout category of a truth characteristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    /// Linear size without a tolerance.
    LinearGeneral,
    /// Diameter without a tolerance.
    DiameterGeneral,
    /// Radius without a tolerance.
    RadiusGeneral,
    /// Angle without a tolerance.
    AngleGeneral,
    /// Chamfer without a tolerance.
    ChamferGeneral,
    /// `±` tolerance.
    Symmetric,
    /// Two deviations, both different from zero.
    Asymmetric,
    /// Two deviations, one of them zero.
    OneSided,
    /// Limit dimension: two limits without a nominal.
    Limit,
    /// Fit designation without printed deviations.
    Fit,
    /// Fit designation with printed deviations.
    FitPrinted,
    /// Metric thread.
    Thread,
    /// Reference dimension.
    Reference,
    /// Anything else (notes, basic dimensions, `MIN`, `MAX`, other kinds). Not gated.
    Other,
}

impl Category {
    /// Every category in report order.
    pub const ALL: [Category; 14] = [
        Category::LinearGeneral,
        Category::DiameterGeneral,
        Category::RadiusGeneral,
        Category::AngleGeneral,
        Category::ChamferGeneral,
        Category::Symmetric,
        Category::Asymmetric,
        Category::OneSided,
        Category::Limit,
        Category::Fit,
        Category::FitPrinted,
        Category::Thread,
        Category::Reference,
        Category::Other,
    ];

    /// True for the common callouts of the M2 exit criterion.
    pub fn is_common(self) -> bool {
        self != Category::Other
    }

    /// Short English label for the report.
    pub fn label(self) -> &'static str {
        match self {
            Category::LinearGeneral => "linear, no tolerance",
            Category::DiameterGeneral => "diameter, no tolerance",
            Category::RadiusGeneral => "radius, no tolerance",
            Category::AngleGeneral => "angle, no tolerance",
            Category::ChamferGeneral => "chamfer, no tolerance",
            Category::Symmetric => "symmetric tolerance",
            Category::Asymmetric => "asymmetric tolerance",
            Category::OneSided => "one sided tolerance",
            Category::Limit => "limit dimension",
            Category::Fit => "fit",
            Category::FitPrinted => "fit with printed deviations",
            Category::Thread => "metric thread",
            Category::Reference => "reference dimension",
            Category::Other => "other (not gated)",
        }
    }
}

/// Properties of the printed callout that cut across the categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Traits {
    /// Written with decimal commas.
    pub comma: bool,
    /// Rotated on the sheet (region angle not 0).
    pub rotated: bool,
    /// Text on two lines: a deviation pair or a limit pair.
    pub stacked: bool,
}

/// The category of a truth characteristic, from its fields and its requirement text.
pub fn classify(c: &TruthCharacteristic) -> Category {
    let text = c.requirement_text.as_str();
    let Some(nominal) = c.nominal else {
        return Category::Other;
    };
    if !c.inspect {
        let reference = text.starts_with('(') || text.contains("REF");
        return if reference {
            Category::Reference
        } else {
            Category::Other
        };
    }
    if c.kind == CharacteristicKind::Thread {
        return Category::Thread;
    }
    let rule = c.tolerance_rule;
    if c.fit.is_some() {
        return match rule {
            Some(ToleranceRule::Explicit) => Category::FitPrinted,
            _ => Category::Fit,
        };
    }
    if rule == Some(ToleranceRule::Explicit) {
        if text.contains('±') {
            return Category::Symmetric;
        }
        if text.contains("MIN") || text.contains("MAX") {
            return Category::Other;
        }
        let signed = text
            .split_whitespace()
            .skip(1)
            .any(|t| t.starts_with(['+', '-', '\u{2212}']) || t == "0");
        if !signed {
            return Category::Limit;
        }
        let (Some(upper), Some(lower)) = (c.upper_limit, c.lower_limit) else {
            return Category::Other;
        };
        return if upper == nominal || lower == nominal {
            Category::OneSided
        } else {
            Category::Asymmetric
        };
    }
    match c.kind {
        CharacteristicKind::Linear => Category::LinearGeneral,
        CharacteristicKind::Diameter => Category::DiameterGeneral,
        CharacteristicKind::Radius => Category::RadiusGeneral,
        CharacteristicKind::Angle => Category::AngleGeneral,
        CharacteristicKind::Chamfer => Category::ChamferGeneral,
        _ => Category::Other,
    }
}

/// The traits of a truth characteristic.
pub fn traits(c: &TruthCharacteristic) -> Traits {
    let chars: Vec<char> = c.requirement_text.chars().collect();
    let comma = chars
        .windows(3)
        .any(|w| w[0].is_ascii_digit() && w[1] == ',' && w[2].is_ascii_digit());
    let stacked = matches!(
        classify(c),
        Category::Asymmetric | Category::OneSided | Category::Limit | Category::FitPrinted
    );
    Traits {
        comma,
        rotated: c.region.angle.abs() > 1e-9,
        stacked,
    }
}

/// True when the truth expects limits that box select cannot derive in M2: an angle with a
/// general tolerance, which needs the shorter leg of the angle.
pub fn is_known_gap(c: &TruthCharacteristic) -> bool {
    classify(c) == Category::AngleGeneral
        && c.tolerance_rule.is_some_and(ToleranceRule::yields_limits)
}

/// The result of box select on one truth characteristic.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseResult {
    /// Name of the drawing (truth file name without extension).
    pub drawing: String,
    /// Truth id.
    pub id: String,
    /// Category of the truth entry.
    pub category: Category,
    /// Traits of the truth entry.
    pub traits: Traits,
    /// See [`is_known_gap`].
    pub known_gap: bool,
    /// Requirement text of the truth.
    pub text: String,
    /// What differs from the truth; `None` when the proposal matches.
    pub failure: Option<String>,
    /// For a known gap: what differs from the M2 expectation (no limits, shorter leg note);
    /// `None` when it matches. Always `None` for other cases.
    pub gap_failure: Option<String>,
}

impl CaseResult {
    /// True when the proposal matches the truth, limits included.
    pub fn correct(&self) -> bool {
        self.failure.is_none()
    }

    /// True when the proposal is what M2 must give: the truth, or for a known gap the truth
    /// without limits plus the shorter leg note.
    pub fn gated_correct(&self) -> bool {
        if self.known_gap {
            self.gap_failure.is_none()
        } else {
            self.correct()
        }
    }
}

/// The M2 expectation for a known gap: the truth without limits, rule `no_tolerance_defined`.
fn without_limits(c: &TruthCharacteristic) -> TruthCharacteristic {
    TruthCharacteristic {
        tolerance_rule: Some(ToleranceRule::NoToleranceDefined),
        upper_limit: None,
        lower_limit: None,
        ..c.clone()
    }
}

/// The tolerance engine with the settings a truth file assumes (a new project's settings when
/// it names none) and the shipped tables.
pub fn engine_for(truth: &TruthFile) -> Result<ToleranceEngine, ContextError> {
    let settings = truth.tolerance_settings.clone().unwrap_or_default();
    let context = ToleranceContext::for_project(&settings, TableSet::shipped()?, [])?;
    Ok(ToleranceEngine::new(context))
}

/// The box a user draws around a truth region: grown by [`BOX_MARGIN`] and axis aligned on
/// the sheet, as the box tool draws it.
pub fn user_box(region: &OrientedBox) -> OrientedBox {
    let turned = (region.angle.rem_euclid(180.0) - 90.0).abs() < 1.0;
    let (w, h) = if turned {
        (region.size.height, region.size.width)
    } else {
        (region.size.width, region.size.height)
    };
    OrientedBox {
        center: region.center,
        size: Size {
            width: w + 2.0 * BOX_MARGIN,
            height: h + 2.0 * BOX_MARGIN,
        },
        angle: 0.0,
    }
}

/// Box select on every characteristic of `truth` in `doc`, interpreted by `interpreter`.
/// `drawing` names the drawing in the results.
pub fn evaluate_drawing(
    doc: &Document,
    truth: &TruthFile,
    drawing: &str,
    interpreter: &dyn Interpreter,
) -> Vec<CaseResult> {
    // Proposals are compared field by field; the sheet id is never looked at.
    let sheet = SheetId::from_uuid(FixedEnvironment::new().new_uuid());
    truth
        .characteristics
        .iter()
        .map(|c| {
            let region = user_box(&c.region);
            let known_gap = is_known_gap(c);
            let (failure, gap_failure) = match doc.region_text(c.sheet as usize, region) {
                Ok(text) => {
                    let context = BoxSelectContext {
                        interpreter,
                        drawing_unit: Unit::Mm,
                        placement: BalloonPlacement {
                            position: Point {
                                x: region.center.x + 30.0,
                                y: region.center.y - 30.0,
                            },
                            anchor: region.center,
                        },
                        job_id: None,
                    };
                    let proposed = box_select_with_notes(sheet, &region, &text, &context);
                    let proposals: Vec<Proposal> =
                        proposed.iter().map(|p| p.proposal.clone()).collect();
                    let gap_failure = known_gap
                        .then(|| gap_check(&proposed, &proposals, c))
                        .flatten();
                    (compare(&proposals, c), gap_failure)
                }
                Err(e) => {
                    let message = format!("no region text: {e}");
                    (Some(message.clone()), known_gap.then_some(message))
                }
            };
            CaseResult {
                drawing: drawing.to_owned(),
                id: c.id.clone(),
                category: classify(c),
                traits: traits(c),
                known_gap,
                text: c.requirement_text.clone(),
                failure,
                gap_failure,
            }
        })
        .collect()
}

/// The M2 check of a known gap: [`without_limits`] and the shorter leg note.
fn gap_check(
    proposed: &[Proposed],
    proposals: &[Proposal],
    c: &TruthCharacteristic,
) -> Option<String> {
    let mut failure = compare(proposals, &without_limits(c));
    let noted = proposed
        .iter()
        .any(|p| p.notes.contains(&dimo_tolerance::Note::ShorterLegUnknown));
    if failure.is_none() && !noted {
        failure = Some("no note that the shorter leg is unknown".to_owned());
    }
    failure
}

/// What differs between the proposals and the truth entry; `None` when they match.
pub fn compare(proposals: &[Proposal], c: &TruthCharacteristic) -> Option<String> {
    let [p] = proposals else {
        let texts: Vec<&str> = proposals
            .iter()
            .map(|p| p.requirement_text.as_str())
            .collect();
        return Some(format!("{} proposals {texts:?}", proposals.len()));
    };
    let mut diffs = Vec::new();
    if p.kind != c.kind {
        diffs.push(format!("kind {:?}, want {:?}", p.kind, c.kind));
    }
    let show = |v: Option<Decimal>| v.map_or_else(|| "none".to_owned(), |v| v.to_string());
    let mut field = |name: &str, got: Option<Decimal>, want: Option<Decimal>| {
        if got != want {
            diffs.push(format!("{name} {}, want {}", show(got), show(want)));
        }
    };
    field("nominal", p.nominal, c.nominal);
    field("upper", p.upper_limit, c.upper_limit);
    field("lower", p.lower_limit, c.lower_limit);
    let rule = p.derivation.as_ref().and_then(|d| d.rule.kind());
    if rule != c.tolerance_rule {
        diffs.push(format!("rule {rule:?}, want {:?}", c.tolerance_rule));
    }
    if p.inspect != c.inspect {
        diffs.push(format!("inspect {}, want {}", p.inspect, c.inspect));
    }
    // Free text (no nominal in the truth) is expected not to parse.
    if let Some(e) = p.parse_error.as_ref().filter(|_| c.nominal.is_some()) {
        diffs.push(format!(
            "parse error at {}: expected {}",
            e.position, e.expected
        ));
    }
    if diffs.is_empty() {
        None
    } else {
        Some(format!(
            "read {:?}: {}",
            p.requirement_text,
            diffs.join("; ")
        ))
    }
}

/// A report row over one trait: label and test.
type TraitRow = (&'static str, fn(&Traits) -> bool);

/// Count and correct cases of one report row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tally {
    /// Cases.
    pub count: usize,
    /// Correct cases.
    pub correct: usize,
}

impl Tally {
    fn of<'a>(cases: impl IntoIterator<Item = &'a CaseResult>) -> Self {
        Self::by(cases, CaseResult::gated_correct)
    }

    fn by<'a>(
        cases: impl IntoIterator<Item = &'a CaseResult>,
        correct: fn(&CaseResult) -> bool,
    ) -> Self {
        let mut t = Tally::default();
        for case in cases {
            t.count += 1;
            t.correct += usize::from(correct(case));
        }
        t
    }

    /// Correct share in percent with one decimal, `-` without cases.
    pub fn percent(&self) -> String {
        if self.count == 0 {
            return "-".to_owned();
        }
        let tenths = (self.correct * 1000 + self.count / 2) / self.count;
        format!("{}.{}", tenths / 10, tenths % 10)
    }

    /// True when at least `percent` percent are correct (exact integer comparison). An empty
    /// tally does not pass.
    pub fn reaches(&self, percent: u32) -> bool {
        self.count > 0 && self.correct * 100 >= self.count * percent as usize
    }
}

/// All results of one evaluation run.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Evaluation {
    /// One result per truth characteristic, in run order.
    pub cases: Vec<CaseResult>,
}

impl Evaluation {
    /// Tally of one category.
    pub fn category(&self, category: Category) -> Tally {
        Tally::of(self.cases.iter().filter(|c| c.category == category))
    }

    /// Tally of all common callouts with the truth limits required, known gaps included.
    pub fn strict(&self) -> Tally {
        Tally::by(
            self.cases.iter().filter(|c| c.category.is_common()),
            CaseResult::correct,
        )
    }

    /// Tally the exit criterion is checked on: all common callouts, known gaps with the M2
    /// expectation ([`CaseResult::gated_correct`]).
    pub fn gated(&self) -> Tally {
        Tally::of(self.cases.iter().filter(|c| c.category.is_common()))
    }

    /// True when the gated tally reaches [`REQUIRED_PERCENT`].
    pub fn passes(&self) -> bool {
        self.gated().reaches(REQUIRED_PERCENT)
    }

    /// The report as plain text: one row per category, rows per trait, totals, failures.
    pub fn report(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "Box select evaluation (M2 exit criterion: at least {REQUIRED_PERCENT} percent correct on common callouts)"
        );
        let _ = writeln!(
            out,
            "Compared: one proposal per truth region, kind, nominal, upper and lower limit, tolerance rule, inspect flag."
        );
        let _ = writeln!(
            out,
            "Known gap: an angle under a general tolerance needs the shorter leg of the angle (ISO 2768-1 table 3), which is not on the sheet; box select gives no limits for it in M2. Gated on that (no limits, rule no_tolerance_defined, shorter leg note); the strict row wants the limits."
        );
        let _ = writeln!(
            out,
            "Caveats: the test_drawing_1 truth values are not yet reviewed by the owner, and every tolerance table (shipped ISO 2768-1 and ISO 286, synth tables) is a draft (D-43)."
        );
        let mut drawings: Vec<&str> = self.cases.iter().map(|c| c.drawing.as_str()).collect();
        drawings.dedup();
        let _ = writeln!(out, "Drawings: {}", drawings.len());
        let _ = writeln!(out);
        let row = |out: &mut String, label: &str, t: Tally| {
            let _ = writeln!(
                out,
                "{label:<34} {:>6} {:>8} {:>8}",
                t.count,
                t.correct,
                t.percent()
            );
        };
        let _ = writeln!(
            out,
            "{:<34} {:>6} {:>8} {:>8}",
            "category", "count", "correct", "percent"
        );
        for category in Category::ALL {
            row(&mut out, category.label(), self.category(category));
        }
        let _ = writeln!(out);
        let traits: [TraitRow; 3] = [
            ("decimal comma", |t| t.comma),
            ("rotated 90 degrees", |t| t.rotated),
            ("stacked", |t| t.stacked),
        ];
        for (label, has) in traits {
            let tally = Tally::of(
                self.cases
                    .iter()
                    .filter(|c| c.category.is_common() && has(&c.traits)),
            );
            row(&mut out, &format!("{label} (gated)"), tally);
        }
        let gaps = self.cases.iter().filter(|c| c.known_gap);
        row(
            &mut out,
            "known gap, M2 expectation",
            Tally::of(gaps.clone()),
        );
        row(
            &mut out,
            "known gap, with limits",
            Tally::by(gaps, CaseResult::correct),
        );
        let _ = writeln!(out);
        row(&mut out, "common, strict (limits required)", self.strict());
        row(&mut out, "common, gated", self.gated());
        let _ = writeln!(
            out,
            "Result: {}",
            if self.passes() { "PASS" } else { "FAIL" }
        );
        let failures: Vec<&CaseResult> = self.cases.iter().filter(|c| !c.correct()).collect();
        if !failures.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "Failures against the truth ({}, known gaps marked):",
                failures.len()
            );
            for c in failures {
                let gap = match (c.known_gap, &c.gap_failure) {
                    (false, _) => String::new(),
                    (true, None) => " [known gap, M2 expectation met]".to_owned(),
                    (true, Some(f)) => format!(" [known gap, M2 expectation failed: {f}]"),
                };
                let _ = writeln!(
                    out,
                    "  {} {} [{}]{gap} {:?}: {}",
                    c.drawing,
                    c.id,
                    c.category.label(),
                    c.text,
                    c.failure.as_deref().unwrap_or_default()
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dimo_core::geometry::{OrientedBox, Point, Size};

    fn d(text: &str) -> Option<Decimal> {
        text.parse().ok()
    }

    fn truth(text: &str, kind: CharacteristicKind) -> TruthCharacteristic {
        TruthCharacteristic {
            id: "c01".into(),
            sheet: 0,
            kind,
            requirement_text: text.into(),
            region: OrientedBox {
                center: Point { x: 10.0, y: 10.0 },
                size: Size {
                    width: 10.0,
                    height: 5.0,
                },
                angle: 0.0,
            },
            nominal: None,
            unit: None,
            fit: None,
            tolerance_rule: None,
            upper_limit: None,
            lower_limit: None,
            inspect: true,
            review_note: None,
        }
    }

    fn sized(
        text: &str,
        kind: CharacteristicKind,
        nominal: &str,
        rule: ToleranceRule,
        limits: Option<(&str, &str)>,
    ) -> TruthCharacteristic {
        let mut c = truth(text, kind);
        c.nominal = d(nominal);
        c.unit = Some(Unit::Mm);
        c.tolerance_rule = Some(rule);
        c.upper_limit = limits.and_then(|l| d(l.0));
        c.lower_limit = limits.and_then(|l| d(l.1));
        c
    }

    // T2.9: the common callout categories, from the truth fields and the printed text.
    #[test]
    fn categories_from_truth() {
        use CharacteristicKind as K;
        use ToleranceRule as R;
        let cases = [
            (
                sized("50", K::Linear, "50", R::NoToleranceDefined, None),
                Category::LinearGeneral,
            ),
            (
                sized("Ø20", K::Diameter, "20", R::General, Some(("20.2", "19.8"))),
                Category::DiameterGeneral,
            ),
            (
                sized("R5", K::Radius, "5", R::General, Some(("5.5", "4.5"))),
                Category::RadiusGeneral,
            ),
            (
                sized("C1", K::Chamfer, "1", R::General, Some(("1.2", "0.8"))),
                Category::ChamferGeneral,
            ),
            (
                sized(
                    "12,5±0,1",
                    K::Linear,
                    "12.5",
                    R::Explicit,
                    Some(("12.6", "12.4")),
                ),
                Category::Symmetric,
            ),
            (
                sized(
                    "100 +0 −0.6",
                    K::Linear,
                    "100",
                    R::Explicit,
                    Some(("100", "99.4")),
                ),
                Category::OneSided,
            ),
            (
                sized(
                    "100 0 −0.6",
                    K::Linear,
                    "100",
                    R::Explicit,
                    Some(("100", "99.4")),
                ),
                Category::OneSided,
            ),
            (
                sized(
                    "100 +0.2 −0.1",
                    K::Linear,
                    "100",
                    R::Explicit,
                    Some(("100.2", "99.9")),
                ),
                Category::Asymmetric,
            ),
            (
                sized(
                    "12.02 11.98",
                    K::Linear,
                    "12.02",
                    R::Explicit,
                    Some(("12.02", "11.98")),
                ),
                Category::Limit,
            ),
            (
                sized("M8x1.25", K::Thread, "8", R::NoToleranceDefined, None),
                Category::Thread,
            ),
            (
                sized("10 MIN", K::Linear, "10", R::Explicit, Some(("11", "10"))),
                Category::Other,
            ),
        ];
        for (c, want) in cases {
            assert_eq!(classify(&c), want, "{}", c.requirement_text);
        }
        let mut fit = sized("Ø8 f7", K::Diameter, "8", R::Fit, Some(("7.987", "7.972")));
        fit.fit = Some("f7".into());
        assert_eq!(classify(&fit), Category::Fit);
        fit.tolerance_rule = Some(R::Explicit);
        assert_eq!(classify(&fit), Category::FitPrinted);
        let mut reference = sized("(42)", K::Linear, "42", R::NoToleranceDefined, None);
        reference.inspect = false;
        assert_eq!(classify(&reference), Category::Reference);
        assert_eq!(classify(&truth("BREAK EDGES", K::Note)), Category::Other);
    }

    #[test]
    fn traits_and_known_gaps() {
        use CharacteristicKind as K;
        let mut c = sized(
            "12,02 11,98",
            K::Linear,
            "12.02",
            ToleranceRule::Explicit,
            Some(("12.02", "11.98")),
        );
        c.region.angle = 90.0;
        assert_eq!(
            traits(&c),
            Traits {
                comma: true,
                rotated: true,
                stacked: true
            }
        );
        let mut angle = sized(
            "30°",
            K::Angle,
            "30",
            ToleranceRule::General,
            Some(("30.5", "29.5")),
        );
        angle.unit = Some(Unit::Deg);
        assert!(is_known_gap(&angle));
        let untoleranced = sized(
            "90.0°",
            K::Angle,
            "90.0",
            ToleranceRule::NoToleranceDefined,
            None,
        );
        assert!(!is_known_gap(&untoleranced));
    }

    #[test]
    fn tally_percent_and_gate() {
        let t = Tally {
            count: 200,
            correct: 198,
        };
        assert_eq!(t.percent(), "99.0");
        assert!(t.reaches(99));
        assert!(
            !Tally {
                count: 200,
                correct: 197
            }
            .reaches(99)
        );
        assert!(!Tally::default().reaches(99));
        assert_eq!(
            Tally {
                count: 3,
                correct: 2
            }
            .percent(),
            "66.7"
        );
    }

    #[test]
    fn user_box_is_axis_aligned_and_grown() {
        let r = OrientedBox {
            center: Point { x: 50.0, y: 50.0 },
            size: Size {
                width: 30.0,
                height: 10.0,
            },
            angle: 90.0,
        };
        let b = user_box(&r);
        assert_eq!(b.angle, 0.0);
        assert_eq!((b.size.width, b.size.height), (13.0, 33.0));
    }
}
