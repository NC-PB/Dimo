//! From a parsed callout to nominal, deviations and limits (spec 08 stage 7, FR-TOL-01).
//!
//! Precedence (FR-TOL-01), the first level that applies wins:
//!
//! 1. explicit tolerance on the callout (deviations, `±`, limit dimension, `MIN`, `MAX`);
//! 2. fit designation expanded with the fit table (FR-TOL-04);
//! 3. drawing rule: the custom table the project assigns as drawing rule (FR-TOL-07);
//! 4. general tolerance by size range (FR-TOL-02), from a general or a custom table;
//! 5. decimal place rule (FR-TOL-06).
//!
//! Without any of them the result is `no_tolerance_defined` without limits. A fit always ends
//! the search: a dimension with a fit that cannot be expanded gets no limits rather than a
//! general tolerance. A table level whose table does not cover the size adds the hint
//! `size_outside_table` and the search goes on. Reference and basic dimensions get no limits
//! and `inspect = false` (D-25, FR-CHR-08).

use dimo_core::characteristic::{CharacteristicKind, Unit};
use dimo_core::derivation::{
    DerivationHint, DerivationRule, RangeBound, SizeRange, TableLookup, TableRef,
    ToleranceDerivation, UnitConversion,
};
use dimo_core::project::{TableClass, UnitRounding};
use dimo_notation::{Callout, Kind, NumberForm, Tolerance};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::context::ToleranceContext;
use crate::designation::Fit;
use crate::fit::FitError;
use crate::format::{AppliesTo, TableKind, ValueUnit};
use crate::table::{LookupError, Range, Table};

/// Millimetres per inch, exact.
const MM_PER_INCH: Decimal = Decimal::from_parts(254, 0, 0, false, 1);

/// Decimal places of an angular general tolerance converted from minutes of arc to degrees.
/// Rounded toward zero, so the tolerance is never wider than the table.
pub const ANGLE_PLACES: u32 = 6;

/// Facts about the feature that the callout text does not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Measures {
    /// Length of the shorter leg of an angle in mm. Selects the row of angular general
    /// tolerances (ISO 2768-1); without it those tolerances cannot apply.
    pub shorter_leg: Option<Decimal>,
}

/// Something the engine could not decide and the user should resolve. Not stored in the
/// project; shown with the proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Note {
    /// An angle without a known shorter leg length, while an angular table part would apply.
    /// No limits were derived from it.
    ShorterLegUnknown,
    /// A fit pair such as `H7/g6` names two features. No limits; split it into the hole and
    /// the shaft.
    FitPair,
}

/// The meaning of one callout: what [`interpret`] returns (spec 08 stage 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpretation {
    /// What the callout dimensions.
    pub kind: CharacteristicKind,
    /// Nominal value in `unit`.
    pub nominal: Option<Decimal>,
    /// Unit of nominal, deviations and limits.
    pub unit: Option<Unit>,
    /// Upper deviation: upper limit minus nominal.
    pub upper_dev: Option<Decimal>,
    /// Lower deviation: lower limit minus nominal.
    pub lower_dev: Option<Decimal>,
    /// Upper limit.
    pub upper_limit: Option<Decimal>,
    /// Lower limit.
    pub lower_limit: Option<Decimal>,
    /// Fit designation as written, e.g. `H7`.
    pub fit: Option<String>,
    /// Number of features (`4X`), at least 1.
    pub quantity: u32,
    /// False for reference and basic dimensions (D-25, FR-CHR-08).
    pub inspect: bool,
    /// Rule, draft flag, hints and unit conversion (FR-TOL-08).
    pub derivation: ToleranceDerivation,
    /// What the engine could not decide.
    pub notes: Vec<Note>,
}

/// Interpret a callout without extra facts about the feature (FR-TOL-01).
pub fn interpret(callout: &Callout, context: &ToleranceContext) -> Interpretation {
    interpret_with(callout, context, &Measures::default())
}

/// Interpret a callout (FR-TOL-01, FR-TOL-02, FR-TOL-04, FR-TOL-06, FR-TOL-07, FR-TOL-09).
pub fn interpret_with(
    callout: &Callout,
    context: &ToleranceContext,
    measures: &Measures,
) -> Interpretation {
    let written = written_unit(callout, context);
    let output = match written {
        Unit::Deg => Unit::Deg,
        _ => context.output_unit().unwrap_or(written),
    };
    let mut state = State {
        hints: Vec::new(),
        notes: Vec::new(),
        draft: false,
    };
    let reference_or_basic = callout.reference || callout.basic;
    if callout.reference {
        state.hints.push(DerivationHint::ReferenceDimension);
    }
    if callout.basic {
        state.hints.push(DerivationHint::BasicDimension);
    }
    let found = if reference_or_basic || matches!(callout.kind, Kind::Thread { .. }) {
        None
    } else {
        derive(callout, context, measures, written, &mut state)
    };
    let nominal = callout.nominal.value;
    let rounding = context.settings().unit_rounding;
    let mut result = Interpretation {
        kind: callout.characteristic_kind(),
        nominal: Some(convert_nominal(nominal, written, output, rounding)),
        unit: Some(output),
        upper_dev: None,
        lower_dev: None,
        upper_limit: None,
        lower_limit: None,
        fit: callout.fit.as_ref().map(ToString::to_string),
        quantity: callout.quantity.unwrap_or(1).max(1),
        inspect: !reference_or_basic,
        derivation: ToleranceDerivation::new(DerivationRule::NoToleranceDefined),
        notes: Vec::new(),
    };
    let mut converted = written != output;
    if let Some(found) = found {
        let source = found.unit;
        converted |= source != output;
        let nominal_source = convert_exact(nominal, written, source);
        let limit = |dev: Option<Decimal>, inward: RoundingStrategy| {
            dev.map(|dev| {
                let value = nominal_source + dev;
                if converted {
                    round(
                        convert_exact(value, source, output),
                        places(output, rounding),
                        inward,
                    )
                } else {
                    value
                }
            })
        };
        result.upper_limit = limit(found.upper, RoundingStrategy::ToNegativeInfinity);
        result.lower_limit = limit(found.lower, RoundingStrategy::ToPositiveInfinity);
        if converted {
            result.upper_dev = result
                .upper_limit
                .zip(result.nominal)
                .map(|(l, n)| zero(l - n));
            result.lower_dev = result
                .lower_limit
                .zip(result.nominal)
                .map(|(l, n)| zero(l - n));
        } else {
            result.upper_dev = found.upper.map(zero);
            result.lower_dev = found.lower.map(zero);
        }
        result.derivation.rule = found.rule;
        result.derivation.conversion = converted.then(|| UnitConversion {
            from: if written == output { source } else { written },
            to: output,
            places: places(output, rounding),
        });
    } else if converted {
        result.derivation.conversion = Some(UnitConversion {
            from: written,
            to: output,
            places: places(output, rounding),
        });
    }
    result.derivation.draft = state.draft;
    result.derivation.hints = state.hints;
    result.notes = state.notes;
    result
}

/// Hints, notes and the draft flag collected while deriving.
struct State {
    hints: Vec<DerivationHint>,
    notes: Vec<Note>,
    draft: bool,
}

/// Deviations of one level, in `unit`, relative to the nominal converted to `unit`.
struct Found {
    rule: DerivationRule,
    unit: Unit,
    upper: Option<Decimal>,
    lower: Option<Decimal>,
}

/// The unit the callout is written in: degrees for angles, else the unit marker, inch
/// notation, or the drawing unit (D-20).
fn written_unit(callout: &Callout, context: &ToleranceContext) -> Unit {
    if matches!(callout.kind, Kind::Angle) {
        return Unit::Deg;
    }
    match callout.unit {
        Some(unit) if unit.is_length() => unit,
        _ if callout.is_inch() => Unit::In,
        _ => context.drawing_unit(),
    }
}

/// The precedence of FR-TOL-01. `None` means no level gives limits.
fn derive(
    callout: &Callout,
    context: &ToleranceContext,
    measures: &Measures,
    written: Unit,
    state: &mut State,
) -> Option<Found> {
    if let Some(tolerance) = &callout.tolerance {
        return Some(explicit(callout, tolerance, context, written, state));
    }
    if let Some(fit) = &callout.fit {
        return fit_level(callout, fit, context, written, state);
    }
    let applies_to = match callout.kind {
        Kind::Linear | Kind::Diameter | Kind::SphericalDiameter | Kind::Depth => AppliesTo::Linear,
        Kind::Radius | Kind::SphericalRadius | Kind::Chamfer { .. } => AppliesTo::RadiusChamfer,
        Kind::Angle => AppliesTo::Angular,
        Kind::Thread { .. } => return None,
    };
    let settings = context.settings();
    let levels = [
        (settings.drawing_rule.as_ref(), true),
        (settings.general.as_ref(), false),
    ];
    for (setting, drawing_rule) in levels {
        let Some(setting) = setting else { continue };
        let Some(table) = context.table(setting) else {
            continue;
        };
        let size = if applies_to == AppliesTo::Angular {
            if !has_part(table, applies_to) {
                continue;
            }
            let Some(leg) = measures.shorter_leg else {
                state.notes.push(Note::ShorterLegUnknown);
                return None;
            };
            leg
        } else {
            convert_exact(callout.nominal.value.abs(), written, Unit::Mm)
        };
        match table.general(applies_to, &setting.class, size) {
            Ok(general) => {
                state.draft |= general.draft;
                let lookup = table_lookup(table, setting, &general.part, &general.range);
                let rule = if drawing_rule {
                    DerivationRule::DrawingRule { lookup }
                } else if table.header().kind == TableKind::Custom {
                    DerivationRule::CustomTable { lookup }
                } else {
                    DerivationRule::General { lookup }
                };
                let (value, unit) = match general.unit {
                    ValueUnit::Mm => (general.value, Unit::Mm),
                    ValueUnit::Arcmin => (
                        (general.value / Decimal::from(60))
                            .round_dp_with_strategy(ANGLE_PLACES, RoundingStrategy::ToZero)
                            .normalize(),
                        Unit::Deg,
                    ),
                };
                return Some(Found {
                    rule,
                    unit,
                    upper: Some(value),
                    lower: Some(-value),
                });
            }
            Err(LookupError::NoPart { .. }) => {}
            Err(_) => state.hints.push(DerivationHint::SizeOutsideTable {
                table: table_ref(table),
            }),
        }
    }
    decimal_rule(callout, context, written)
}

/// Level 1: the tolerance written on the callout. Deviations are sorted here: the parser keeps
/// the written order. A difference from the fit table is a hint, never a correction.
fn explicit(
    callout: &Callout,
    tolerance: &Tolerance,
    context: &ToleranceContext,
    written: Unit,
    state: &mut State,
) -> Found {
    let nominal = callout.nominal.value;
    let (upper, lower, compare) = match tolerance {
        Tolerance::Symmetric(t) => {
            let t = t.value.abs();
            (Some(t), Some(-t), true)
        }
        Tolerance::Deviations { upper, lower } => (
            Some(upper.value.max(lower.value)),
            Some(upper.value.min(lower.value)),
            true,
        ),
        Tolerance::Limits { other } => {
            let other = other.value - nominal;
            (
                Some(other.max(Decimal::ZERO)),
                Some(other.min(Decimal::ZERO)),
                false,
            )
        }
        Tolerance::Min => (None, Some(Decimal::ZERO), false),
        Tolerance::Max => (Some(Decimal::ZERO), None, false),
    };
    if let (true, Some(fit), Some(u), Some(l)) = (compare, &callout.fit, upper, lower)
        && fit.second.is_none()
        && written.is_length()
    {
        let fit_text = fit.to_string();
        let nominal_mm = convert_exact(nominal.abs(), written, Unit::Mm);
        match expand(context, &fit_text, nominal_mm) {
            Ok((tu, tl, _)) => {
                if !same_deviation(u, tu, written) || !same_deviation(l, tl, written) {
                    state.hints.push(DerivationHint::FitDeviationsDiffer {
                        fit: fit_text,
                        table_upper_dev: Some(tu),
                        table_lower_dev: Some(tl),
                    });
                }
            }
            Err(hint) => state.hints.push(hint),
        }
    }
    Found {
        rule: DerivationRule::Explicit,
        unit: written,
        upper,
        lower,
    }
}

/// Whether a written deviation equals a fit table deviation in mm. Inch values are compared at
/// the written number of decimal places, so the rounding of a conversion is no difference.
fn same_deviation(written: Decimal, table_mm: Decimal, unit: Unit) -> bool {
    match unit {
        Unit::In => {
            let table = (table_mm / MM_PER_INCH)
                .round_dp_with_strategy(written.scale(), RoundingStrategy::MidpointAwayFromZero);
            table == written
        }
        _ => table_mm == written,
    }
}

/// Level 2: the fit designation (FR-TOL-04).
fn fit_level(
    callout: &Callout,
    fit: &dimo_notation::Fit,
    context: &ToleranceContext,
    written: Unit,
    state: &mut State,
) -> Option<Found> {
    let fit_text = fit.to_string();
    if fit.second.is_some() {
        state
            .hints
            .push(DerivationHint::UnknownFit { fit: fit_text });
        state.notes.push(Note::FitPair);
        return None;
    }
    let nominal_mm = convert_exact(callout.nominal.value.abs(), written, Unit::Mm);
    match expand(context, &fit_text, nominal_mm) {
        Ok((upper, lower, found)) => {
            state.draft |= found.draft;
            Some(Found {
                rule: DerivationRule::Fit {
                    table: TableRef {
                        id: found.table,
                        version: found.version,
                    },
                    fit: fit_text,
                    range: Some(size_range(&found.range)),
                },
                unit: Unit::Mm,
                upper: Some(upper),
                lower: Some(lower),
            })
        }
        Err(hint) => {
            state.hints.push(hint);
            None
        }
    }
}

/// Expand a single tolerance class with the fit table. The error is the hint to show.
fn expand(
    context: &ToleranceContext,
    fit: &str,
    nominal_mm: Decimal,
) -> Result<(Decimal, Decimal, crate::fit::FitLimits), DerivationHint> {
    let unknown = || DerivationHint::UnknownFit {
        fit: fit.to_owned(),
    };
    let table = context.fit_table().ok_or_else(unknown)?;
    let parsed = Fit::parse(fit).ok_or_else(unknown)?;
    match table.expand(&parsed, nominal_mm) {
        Ok(found) => Ok((found.upper, found.lower, found)),
        Err(FitError::Lookup { source, .. })
            if matches!(
                *source,
                LookupError::OutOfRange { .. }
                    | LookupError::SizeNotPositive { .. }
                    | LookupError::NotDefined { .. }
            ) =>
        {
            Err(DerivationHint::SizeOutsideTable {
                table: table_ref(table),
            })
        }
        Err(_) => Err(unknown()),
    }
}

/// Level 5: decimal place rule (FR-TOL-06). Lengths written as decimals only; the tolerance is
/// in the unit the dimension is written in.
fn decimal_rule(callout: &Callout, context: &ToleranceContext, written: Unit) -> Option<Found> {
    if !written.is_length()
        || !matches!(
            callout.nominal.form,
            NumberForm::Decimal | NumberForm::NoLeadingZero
        )
    {
        return None;
    }
    let places = callout.nominal.value.scale();
    let rule = context
        .settings()
        .decimal_rules
        .iter()
        .find(|r| r.places == places)?;
    Some(Found {
        rule: DerivationRule::DecimalRule {
            places,
            tolerance: rule.tolerance,
        },
        unit: written,
        upper: Some(rule.tolerance),
        lower: Some(-rule.tolerance),
    })
}

fn has_part(table: &Table, applies_to: AppliesTo) -> bool {
    table
        .parts()
        .iter()
        .any(|p| p.data().applies_to == Some(applies_to))
}

fn table_ref(table: &Table) -> TableRef {
    TableRef {
        id: table.id().to_owned(),
        version: table.header().version,
    }
}

fn table_lookup(table: &Table, setting: &TableClass, part: &str, range: &Range) -> TableLookup {
    TableLookup {
        table: table_ref(table),
        part: part.to_owned(),
        class: setting.class.clone(),
        range: size_range(range),
    }
}

/// The range of a table row as stored in the derivation.
pub fn size_range(range: &Range) -> SizeRange {
    let bound = |b: crate::table::Bound| RangeBound {
        value: b.value,
        inclusive: b.inclusive,
    };
    SizeRange {
        min: range.min.map(bound),
        max: range.max.map(bound),
    }
}

/// Convert a value between length units. Inch to mm is exact; mm to inch is a division with
/// up to 28 significant digits, rounded afterwards by the caller.
fn convert_exact(value: Decimal, from: Unit, to: Unit) -> Decimal {
    match (from, to) {
        (Unit::In, Unit::Mm) => value * MM_PER_INCH,
        (Unit::Mm, Unit::In) => value / MM_PER_INCH,
        _ => value,
    }
}

/// The nominal in the output unit, rounded half away from zero when converted (FR-TOL-09).
fn convert_nominal(value: Decimal, from: Unit, to: Unit, rounding: UnitRounding) -> Decimal {
    if from == to {
        value
    } else {
        round(
            convert_exact(value, from, to),
            places(to, rounding),
            RoundingStrategy::MidpointAwayFromZero,
        )
    }
}

/// Round to `places` and keep exactly that many digits, so a converted value shows its
/// precision (`25.400`).
fn round(value: Decimal, places: u32, strategy: RoundingStrategy) -> Decimal {
    let mut rounded = zero(value.round_dp_with_strategy(places, strategy));
    rounded.rescale(places);
    rounded
}

/// Decimal places of converted values in `unit` (FR-TOL-09).
fn places(unit: Unit, rounding: UnitRounding) -> u32 {
    match unit {
        Unit::In => rounding.inch_places,
        _ => rounding.mm_places,
    }
}

/// Negative zero (from `-0` on the drawing) as positive zero.
fn zero(value: Decimal) -> Decimal {
    if value.is_zero() {
        let mut v = value;
        v.set_sign_positive(true);
        v
    } else {
        value
    }
}
