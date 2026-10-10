//! Reading one callout text: parse, then interpret (spec 08 stages 6 and 7, FR-REC-08).
//!
//! Box select and text typed into the value field (M2 decision 5) both go through
//! [`read_callout`], so a typed `Ø30 H7 +0.0203 -0` gives the same values as a box around the
//! printed one.

use dimo_core::characteristic::{CharacteristicKind, FieldValue, Unit};
use dimo_core::derivation::ToleranceDerivation;
use dimo_core::proposal::{EngineVersion, ParseHint, ParseIssue};
use dimo_notation::{Callout, parse_callout};
use rust_decimal::Decimal;

use crate::interpret::Interpreter;

/// Engine name of the callout parser, stored on proposals.
pub const PARSER_ENGINE: &str = "dimo-notation";

/// What one callout text says: the characteristic fields of a proposal (data model
/// `Proposal`) without source and placement.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// What the characteristic describes. Text that does not parse is a `note` when it holds a
    /// word of three or more letters, else `other`.
    pub kind: CharacteristicKind,
    /// The text as read, trimmed.
    pub requirement_text: String,
    /// Nominal value.
    pub nominal: Option<Decimal>,
    /// Unit of nominal, deviations and limits.
    pub unit: Option<Unit>,
    /// Upper deviation.
    pub upper_dev: Option<Decimal>,
    /// Lower deviation.
    pub lower_dev: Option<Decimal>,
    /// Upper limit.
    pub upper_limit: Option<Decimal>,
    /// Lower limit.
    pub lower_limit: Option<Decimal>,
    /// Fit designation such as `H7`.
    pub fit: Option<String>,
    /// Which rule produced the limits.
    pub derivation: Option<ToleranceDerivation>,
    /// Number of features (`4X`), at least 1.
    pub quantity: u32,
    /// False for reference and basic dimensions (D-25, FR-CHR-08).
    pub inspect: bool,
    /// Why the text could not be parsed. Then there are no values besides the text.
    pub parse_error: Option<ParseIssue>,
    /// Assumptions of the parser to check.
    pub parse_hints: Vec<ParseHint>,
    /// Parser and interpreter that made this reading.
    pub engines: Vec<EngineVersion>,
}

/// Parses and interprets one callout text (FR-REC-02, ADR 0006: a reading changes nothing in
/// the project). `basic_frame` is true when the PDF draws a basic dimension frame around the
/// text (M2 decision 6). `drawing_unit` is the unit of the sheet (D-20).
pub fn read_callout(
    text: &str,
    basic_frame: bool,
    drawing_unit: Unit,
    interpreter: &dyn Interpreter,
) -> Reading {
    let text = text.trim();
    let mut engines = vec![EngineVersion::new(PARSER_ENGINE, env!("CARGO_PKG_VERSION"))];
    let mut callout = match parse_callout(text) {
        Ok(callout) => callout,
        Err(error) => {
            return Reading {
                kind: if has_word(text) {
                    CharacteristicKind::Note
                } else {
                    CharacteristicKind::Other
                },
                requirement_text: text.to_owned(),
                nominal: None,
                unit: None,
                upper_dev: None,
                lower_dev: None,
                upper_limit: None,
                lower_limit: None,
                fit: None,
                derivation: None,
                quantity: 1,
                inspect: true,
                parse_error: Some(ParseIssue {
                    position: u32::try_from(error.position).unwrap_or(u32::MAX),
                    expected: error.expected,
                }),
                parse_hints: Vec::new(),
                engines,
            };
        }
    };
    callout.basic |= basic_frame;
    let interpretation = interpreter.interpret(&callout, drawing_unit);
    engines.push(interpreter.engine());
    Reading {
        kind: callout.characteristic_kind(),
        requirement_text: text.to_owned(),
        nominal: interpretation.nominal,
        unit: interpretation.unit,
        upper_dev: interpretation.upper_dev,
        lower_dev: interpretation.lower_dev,
        upper_limit: interpretation.upper_limit,
        lower_limit: interpretation.lower_limit,
        fit: callout.fit.as_ref().map(ToString::to_string),
        derivation: interpretation.derivation,
        quantity: callout.quantity.unwrap_or(1).max(1),
        inspect: !(callout.reference || callout.basic),
        parse_error: None,
        parse_hints: hints(text, &callout),
        engines,
    }
}

impl Reading {
    /// The fields to set on a characteristic for this reading, as one `update_fields` or
    /// `add_characteristic` command (M2 decision 5). A text that does not parse sets only the
    /// requirement text, as before T2.6.
    pub fn field_values(&self) -> Vec<FieldValue> {
        let mut values = vec![FieldValue::RequirementText(self.requirement_text.clone())];
        if self.parse_error.is_some() {
            return values;
        }
        values.extend([
            FieldValue::Kind(self.kind),
            FieldValue::Nominal(self.nominal.into()),
            FieldValue::Unit(self.unit),
            FieldValue::UpperDev(self.upper_dev.into()),
            FieldValue::LowerDev(self.lower_dev.into()),
            FieldValue::UpperLimit(self.upper_limit.into()),
            FieldValue::LowerLimit(self.lower_limit.into()),
            FieldValue::Fit(self.fit.clone()),
            FieldValue::Quantity(self.quantity),
            FieldValue::Inspect(self.inspect),
            FieldValue::Derivation(self.derivation.clone()),
        ]);
        values
    }
}

/// True if the text holds a word of at least three letters (free text, not a callout).
fn has_word(text: &str) -> bool {
    text.split(|c: char| !c.is_alphabetic())
        .any(|word| word.chars().count() >= 3)
}

/// Parser assumptions visible in the text and the callout.
fn hints(text: &str, callout: &Callout) -> Vec<ParseHint> {
    let mut hints = Vec::new();
    if callout.is_inch() && callout.unit != Some(Unit::In) {
        hints.push(ParseHint::InchFromNotation);
    }
    let chars: Vec<char> = text.chars().collect();
    if chars
        .windows(3)
        .any(|w| w[0].is_ascii_digit() && w[1] == ',' && w[2].is_ascii_digit())
    {
        hints.push(ParseHint::DecimalComma);
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::CalloutOnly;
    use dimo_core::derivation::DerivationRule;

    fn read(text: &str) -> Reading {
        read_callout(text, false, Unit::Mm, &CalloutOnly)
    }

    fn d(text: &str) -> Option<Decimal> {
        text.parse().ok()
    }

    // FR-REC-02: the fields of a proposal from the printed text.
    #[test]
    fn a_fit_with_printed_deviations() {
        let r = read(" Ø30 H7 +0.0203 -0 ");
        assert_eq!(r.kind, CharacteristicKind::Diameter);
        assert_eq!(r.requirement_text, "Ø30 H7 +0.0203 -0");
        assert_eq!(r.nominal, d("30"));
        assert_eq!(r.fit.as_deref(), Some("H7"));
        assert_eq!((r.upper_limit, r.lower_limit), (d("30.0203"), d("30")));
        assert_eq!(r.derivation.map(|d| d.rule), Some(DerivationRule::Explicit));
        assert_eq!(r.unit, Some(Unit::Mm));
        assert!(r.inspect && r.parse_error.is_none());
        let names: Vec<&str> = r.engines.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [PARSER_ENGINE, crate::interpret::CALLOUT_ONLY_ENGINE]
        );
    }

    #[test]
    fn quantity_reference_and_basic() {
        let r = read("4X Ø6.6 THRU");
        assert_eq!((r.quantity, r.inspect), (4, true));
        assert!(!read("(42)").inspect);
        let framed = read_callout("25", true, Unit::Mm, &CalloutOnly);
        assert!(!framed.inspect);
        assert!(read("25").inspect);
    }

    // Spec 08 stage 6: a parse error keeps the text, has no limits and tells the position.
    #[test]
    fn parse_errors_keep_the_text() {
        let r = read("BREAK ALL SHARP EDGES");
        assert_eq!(r.kind, CharacteristicKind::Note);
        assert_eq!(r.requirement_text, "BREAK ALL SHARP EDGES");
        assert_eq!(
            (r.nominal, r.upper_limit, r.lower_limit),
            (None, None, None)
        );
        assert_eq!(r.parse_error.as_ref().map(|e| e.position), Some(0));
        assert_eq!(r.field_values().len(), 1);
        let r = read("Ø30 +");
        assert_eq!(r.kind, CharacteristicKind::Other);
        assert!(r.parse_error.is_some_and(|e| e.position > 0));
    }

    #[test]
    fn hints_for_comma_and_inch_notation() {
        assert_eq!(read("12,5±0,1").parse_hints, [ParseHint::DecimalComma]);
        assert_eq!(read("Ø.250").parse_hints, [ParseHint::InchFromNotation]);
        assert_eq!(read("Ø.250\"").parse_hints, Vec::<ParseHint>::new());
    }

    // M2 decision 5: typed text sets the same fields in one command.
    #[test]
    fn field_values_set_every_interpreted_field() {
        let values = read("Ø30 H7 +0.0203 -0").field_values();
        assert_eq!(values.len(), 12);
        assert_eq!(
            values[0],
            FieldValue::RequirementText("Ø30 H7 +0.0203 -0".into())
        );
        assert!(values.contains(&FieldValue::UpperLimit(d("30.0203").into())));
        assert!(values.contains(&FieldValue::Fit(Some("H7".into()))));
    }
}
