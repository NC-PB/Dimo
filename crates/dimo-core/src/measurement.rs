//! Samples and measurement results (data model 07 "Sample and Result", FR-MEA-01, FR-MEA-06,
//! M3 decisions 1, 2, 4, 6, 7 and 8).
//!
//! - A [`Sample`] is one measured part. The order of `Project::samples` is the column order.
//! - A [`MeasurementResult`] is the value entered for one sample and one characteristic. There is
//!   at most one per pair. `Project::results` is kept sorted by sample order, then by placement
//!   order of the characteristics, so saves are byte identical (rule 11).
//! - The verdict is computed from the result and the current limits when it is needed, never
//!   stored (M3 decision 2).
//! - Text typed in the measurement grid is parsed here ([`parse_result`], decision 8), so the
//!   frontend never interprets numbers (rule 2).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::decimal::decimal_schema;
use crate::id::{CharId, SampleId};

/// Most samples one project holds (M3 decision 6).
pub const MAX_SAMPLES: usize = 500;

/// Most samples one "add samples" command adds (M3 decision 6).
pub const MAX_SAMPLES_PER_ADD: u32 = 100;

/// One measured part (data model `Sample`, FR-MEA-01). Empty text means not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Sample {
    /// Stable ID, never changes.
    pub id: SampleId,
    /// Serial number or other part identifier: trimmed, not empty, unique within the project
    /// (case sensitive, decision 6).
    pub serial: String,
    /// Measuring date as `YYYY-MM-DD`, or empty.
    pub date: String,
    /// Who measured the part (D-27).
    pub inspector: String,
    /// Gauges and instruments used, free text.
    pub gauges: String,
    /// Free text.
    pub comment: String,
}

/// One field of a sample, used by
/// [`Command::UpdateSample`](crate::command::Command::UpdateSample) (MF F4).
///
/// Serialized as `{ "field": "serial", "value": "7" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum SampleField {
    /// Serial number; trimmed, not empty, unique.
    Serial(String),
    /// Measuring date as `YYYY-MM-DD`, or empty.
    Date(String),
    /// Who measured the part.
    Inspector(String),
    /// Gauges and instruments used.
    Gauges(String),
    /// Free text.
    Comment(String),
}

impl Sample {
    /// Sets one field. Serial and date are not checked here.
    pub fn set(&mut self, value: &SampleField) {
        let (slot, text) = match value {
            SampleField::Serial(t) => (&mut self.serial, t),
            SampleField::Date(t) => (&mut self.date, t),
            SampleField::Inspector(t) => (&mut self.inspector, t),
            SampleField::Gauges(t) => (&mut self.gauges, t),
            SampleField::Comment(t) => (&mut self.comment, t),
        };
        slot.clone_from(text);
    }
}

/// Result of a go/no go or visual check (FR-MEA-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum AttributeValue {
    /// The requirement is met.
    Ok,
    /// The requirement is not met.
    NotOk,
}

/// What was measured (FR-MEA-02, FR-MEA-06). Serialized as
/// `{ "type": "variable", "value": "12.10" }` or `{ "type": "attribute", "value": "ok" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultValue {
    /// A measured value in the unit of the characteristic, with the decimals as typed
    /// (decision 8, rule 5).
    Variable {
        /// The value; its scale is kept (`12.10` stays `12.10`).
        #[serde(with = "crate::decimal::serde_str")]
        #[schemars(schema_with = "decimal_schema")]
        #[cfg_attr(feature = "specta", specta(type = String))]
        value: Decimal,
    },
    /// An attribute check.
    Attribute {
        /// OK or not OK.
        value: AttributeValue,
    },
}

impl ResultValue {
    /// The result type this value belongs to: [`ResultType::Variable`] or
    /// [`ResultType::Attribute`].
    pub fn result_type(&self) -> ResultType {
        match self {
            Self::Variable { .. } => ResultType::Variable,
            Self::Attribute { .. } => ResultType::Attribute,
        }
    }

    /// True if two values are equal and written with the same digits (`8.0` and `8.00`
    /// differ for the user, FR-EXP-11).
    pub fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Variable { value: a }, Self::Variable { value: b }) => {
                a == b && a.scale() == b.scale()
            }
            (a, b) => a == b,
        }
    }
}

/// Where a result came from (data model "Result source"). Serialized as `{ "type": "manual" }`
/// so the import source of M7 (file and row) can be added without a format change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultSource {
    /// Typed or pasted by the user.
    Manual,
}

/// The result of one sample for one characteristic (data model `Result`, M3 decision 1).
///
/// Holds only what the user entered. The verdict is computed from the value and the current
/// limits of the characteristic (M3 decision 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct MeasurementResult {
    /// The measured part.
    pub sample: SampleId,
    /// The characteristic.
    pub characteristic: CharId,
    /// The value.
    pub value: ResultValue,
    /// Where the value came from.
    pub source: ResultSource,
    /// Free text.
    pub comment: String,
}

impl MeasurementResult {
    /// The cell of this result.
    pub fn cell(&self) -> ResultCell {
        ResultCell {
            sample: self.sample,
            characteristic: self.characteristic,
        }
    }
}

/// One cell of the measurement grid: a sample and a characteristic.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ResultCell {
    /// The sample (column).
    pub sample: SampleId,
    /// The characteristic (row).
    pub characteristic: CharId,
}

/// One cell to set with [`Command::SetResults`](crate::command::Command::SetResults).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ResultEntry {
    /// The sample (column).
    pub sample: SampleId,
    /// The characteristic (row).
    pub characteristic: CharId,
    /// Text as typed or pasted, parsed by the effective result type of the characteristic
    /// ([`parse_result`]). Empty or white space only clears the cell.
    pub text: String,
    /// New comment of the result; `null` keeps the current one (empty for a new result). Not
    /// used when the text clears the cell: the comment goes with the result.
    pub comment: Option<String>,
}

/// How a characteristic is measured (FR-MEA-06, M3 decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    /// Variable if the characteristic has a limit, otherwise attribute (default).
    #[default]
    Auto,
    /// A measured value compared with the limits.
    Variable,
    /// OK or not OK, for go/no go and visual checks.
    Attribute,
}

/// Why a typed result was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResultParseError {
    /// Not a number: digits with one decimal point or comma and an optional minus sign.
    #[error("not a number; type digits with a decimal point or comma, e.g. 12.05 or -0,3")]
    NotANumber,
    /// Not an attribute result.
    #[error("not an attribute result; type ok, o or + for OK and not ok, n or x for not OK")]
    NotAnAttribute,
}

/// Parses a measured value (M3 decision 8): an optional minus sign as hyphen or U+2212, digits,
/// and an optional fraction after a decimal point or comma. A missing leading zero (`.5`) is
/// accepted. White space around the text is ignored. The scale is kept as typed: `12,10` gives
/// `12.10`. No exponent, no plus sign, no thousands separators.
pub fn parse_variable(text: &str) -> Result<Decimal, ResultParseError> {
    let text = text.trim();
    let (negative, digits) = match text
        .strip_prefix('-')
        .or_else(|| text.strip_prefix('\u{2212}'))
    {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    if digits.contains('.') && digits.contains(',') {
        return Err(ResultParseError::NotANumber);
    }
    let digits = digits.replace(',', ".");
    let digits = if digits.starts_with('.') {
        format!("0{digits}")
    } else {
        digits
    };
    let canonical = if negative {
        format!("-{digits}")
    } else {
        digits
    };
    crate::decimal::parse_decimal(&canonical).ok_or(ResultParseError::NotANumber)
}

/// Parses an attribute result (M3 decision 7), ignoring case and surrounding white space.
///
/// | OK | not OK |
/// |---|---|
/// | `ok`, `o`, `+`, `io`, `i.o.` | `not ok`, `not_ok`, `nok`, `n`, `x`, `nio`, `n.i.o.` |
pub fn parse_attribute(text: &str) -> Result<AttributeValue, ResultParseError> {
    match text.trim().to_lowercase().as_str() {
        "ok" | "o" | "+" | "io" | "i.o." => Ok(AttributeValue::Ok),
        "not ok" | "not_ok" | "nok" | "n" | "x" | "nio" | "n.i.o." => Ok(AttributeValue::NotOk),
        _ => Err(ResultParseError::NotAnAttribute),
    }
}

/// Parses text typed for a characteristic measured as `result_type`. [`ResultType::Auto`] is
/// resolved by the caller with
/// [`Characteristic::effective_result_type`](crate::characteristic::Characteristic::effective_result_type);
/// here it reads a number if the text is one, else an attribute.
pub fn parse_result(text: &str, result_type: ResultType) -> Result<ResultValue, ResultParseError> {
    let variable = |text| parse_variable(text).map(|value| ResultValue::Variable { value });
    let attribute = |text| parse_attribute(text).map(|value| ResultValue::Attribute { value });
    match result_type {
        ResultType::Variable => variable(text),
        ResultType::Attribute => attribute(text),
        ResultType::Auto => variable(text).or_else(|_| attribute(text)),
    }
}

/// Why a serial is not accepted (M3 decision 6).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SerialError {
    /// Empty or only white space.
    #[error("the serial must not be empty")]
    Empty,
    /// Another sample has this serial.
    #[error("serial {0:?} is used by another sample")]
    Duplicate(String),
}

/// The serial as stored: `text` trimmed, refused if empty or used by another sample than
/// `except` (case sensitive, M3 decision 6).
pub fn check_serial(
    samples: &[Sample],
    text: &str,
    except: Option<SampleId>,
) -> Result<String, SerialError> {
    let serial = text.trim();
    if serial.is_empty() {
        return Err(SerialError::Empty);
    }
    if samples
        .iter()
        .any(|s| Some(s.id) != except && s.serial == serial)
    {
        return Err(SerialError::Duplicate(serial.to_owned()));
    }
    Ok(serial.to_owned())
}

/// Serials for `count` new samples: plain numbers continuing the highest plain number of the
/// existing serials (`1`, `2`, `3`; M3 decision 6). A plain number is a serial of ASCII digits
/// only; others (`A-17`) are ignored. The result never repeats an existing serial.
pub fn next_serials(samples: &[Sample], count: usize) -> Vec<String> {
    let highest = samples
        .iter()
        .filter(|s| !s.serial.is_empty() && s.serial.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|s| s.serial.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    let mut out = Vec::with_capacity(count);
    let mut next = highest;
    while out.len() < count {
        next = next.saturating_add(1);
        let serial = next.to_string();
        if !samples.iter().any(|s| s.serial == serial) && !out.contains(&serial) {
            out.push(serial);
        }
        if next == u64::MAX {
            break;
        }
    }
    out
}

/// True for an empty text or a calendar date `YYYY-MM-DD` (years 0001 to 9999).
pub fn is_date_or_empty(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let b = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| b[range].iter().all(u8::is_ascii_digit);
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' || !digits(0..4) || !digits(5..7) {
        return false;
    }
    if !digits(8..10) {
        return false;
    }
    let number = |range: std::ops::Range<usize>| {
        b[range]
            .iter()
            .fold(0u32, |n, d| n * 10 + u32::from(d - b'0'))
    };
    let (year, month, day) = (number(0..4), number(5..7), number(8..10));
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year >= 1 && (1..=days).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn dec(text: &str) -> Decimal {
        crate::decimal::parse_decimal(text).unwrap()
    }

    /// Decision 8: point or comma, hyphen or U+2212, scale kept.
    #[test]
    fn variable_results_parse_as_typed() {
        let accepted = [
            ("12.10", "12.10"),
            ("12,10", "12.10"),
            ("  7 ", "7"),
            ("-0.02", "-0.02"),
            ("\u{2212}0,02", "-0.02"),
            (".5", "0.5"),
            (",5", "0.5"),
            ("-.5", "-0.5"),
            ("\u{2212},25", "-0.25"),
            ("0", "0"),
            ("-0", "0"),
            ("007.50", "7.50"),
            ("30.0000", "30.0000"),
        ];
        for (text, want) in accepted {
            let got = parse_variable(text).unwrap_or_else(|e| panic!("{text:?}: {e}"));
            assert_eq!(got, dec(want), "{text:?}");
            assert_eq!(got.scale(), dec(want).scale(), "{text:?} keeps its scale");
        }
        let refused = [
            "",
            " ",
            "-",
            "\u{2212}",
            ".",
            ",",
            "5.",
            "5,",
            "+5",
            "1e3",
            "1.234,5",
            "1,234.5",
            "1.2.3",
            "1,2,3",
            "1 000",
            "12mm",
            "ok",
            "--1",
            "- 1",
            "0x10",
            "∞",
            "NaN",
            "\u{2013}1",
        ];
        for text in refused {
            assert_eq!(
                parse_variable(text),
                Err(ResultParseError::NotANumber),
                "{text:?}"
            );
        }
    }

    /// Decision 7: attribute words and keys.
    #[test]
    fn attribute_results_parse_words_and_keys() {
        let cases = [
            ("ok", Some(AttributeValue::Ok)),
            ("OK", Some(AttributeValue::Ok)),
            (" o ", Some(AttributeValue::Ok)),
            ("O", Some(AttributeValue::Ok)),
            ("+", Some(AttributeValue::Ok)),
            ("i.O.", Some(AttributeValue::Ok)),
            ("iO", Some(AttributeValue::Ok)),
            ("not ok", Some(AttributeValue::NotOk)),
            ("Not OK", Some(AttributeValue::NotOk)),
            ("not_ok", Some(AttributeValue::NotOk)),
            ("NOK", Some(AttributeValue::NotOk)),
            ("n", Some(AttributeValue::NotOk)),
            ("x", Some(AttributeValue::NotOk)),
            ("X", Some(AttributeValue::NotOk)),
            ("n.i.O.", Some(AttributeValue::NotOk)),
            ("niO", Some(AttributeValue::NotOk)),
            ("", None),
            ("-", None),
            ("yes", None),
            ("1", None),
            ("okay", None),
            ("not", None),
        ];
        for (text, want) in cases {
            assert_eq!(parse_attribute(text).ok(), want, "{text:?}");
        }
    }

    #[test]
    fn results_parse_by_type() {
        let cases = [
            ("12,5", ResultType::Variable, Some("v12.5")),
            ("ok", ResultType::Variable, None),
            ("x", ResultType::Attribute, Some("not_ok")),
            ("12", ResultType::Attribute, None),
            ("3", ResultType::Auto, Some("v3")),
            ("o", ResultType::Auto, Some("ok")),
            ("?", ResultType::Auto, None),
        ];
        for (text, result_type, want) in cases {
            let got = parse_result(text, result_type).ok().map(|v| match v {
                ResultValue::Variable { value } => format!("v{value}"),
                ResultValue::Attribute {
                    value: AttributeValue::Ok,
                } => "ok".into(),
                ResultValue::Attribute {
                    value: AttributeValue::NotOk,
                } => "not_ok".into(),
            });
            assert_eq!(got.as_deref(), want, "{text:?} {result_type:?}");
        }
        assert_eq!(
            parse_result("ok", ResultType::Variable),
            Err(ResultParseError::NotANumber)
        );
        assert_eq!(
            parse_result("1", ResultType::Attribute),
            Err(ResultParseError::NotAnAttribute)
        );
    }

    fn sample(n: u128, serial: &str) -> Sample {
        Sample {
            id: SampleId::from_uuid(Uuid::from_u128(n)),
            serial: serial.into(),
            date: String::new(),
            inspector: String::new(),
            gauges: String::new(),
            comment: String::new(),
        }
    }

    /// Decision 6: trimmed, not empty, unique (case sensitive).
    #[test]
    fn serial_rules() {
        let samples = [sample(1, "1"), sample(2, "A-17"), sample(3, "x")];
        let cases: [(&str, Option<u128>, Result<&str, SerialError>); 9] = [
            ("  42 ", None, Ok("42")),
            ("", None, Err(SerialError::Empty)),
            (" \t ", None, Err(SerialError::Empty)),
            ("1", None, Err(SerialError::Duplicate("1".into()))),
            (" A-17", None, Err(SerialError::Duplicate("A-17".into()))),
            ("a-17", None, Ok("a-17")),
            ("X", None, Ok("X")),
            // A sample keeps its own serial.
            ("A-17", Some(2), Ok("A-17")),
            ("x", Some(2), Err(SerialError::Duplicate("x".into()))),
        ];
        for (text, except, want) in cases {
            let except = except.map(|n| SampleId::from_uuid(Uuid::from_u128(n)));
            assert_eq!(
                check_serial(&samples, text, except),
                want.map(str::to_owned),
                "{text:?}"
            );
        }
    }

    /// Decision 6: new serials continue the highest plain number.
    #[test]
    fn next_serials_continue_the_highest_plain_number() {
        let cases: [(&[&str], usize, &[&str]); 7] = [
            (&[], 3, &["1", "2", "3"]),
            (&["1", "2"], 1, &["3"]),
            (&["5", "2"], 2, &["6", "7"]),
            (&["A-17", "B"], 2, &["1", "2"]),
            (&["007", "S12"], 1, &["8"]),
            (&["3", "4a", "-9", "1.5"], 1, &["4"]),
            (&["99999999999999999999", "2"], 1, &["3"]),
        ];
        for (existing, count, want) in cases {
            let samples: Vec<Sample> = existing
                .iter()
                .enumerate()
                .map(|(i, s)| sample(u128::try_from(i).unwrap() + 1, s))
                .collect();
            assert_eq!(next_serials(&samples, count), want, "{existing:?}");
        }
    }

    #[test]
    fn dates_are_calendar_dates_or_empty() {
        for text in [
            "",
            "2026-10-12",
            "2024-02-29",
            "2000-02-29",
            "0001-01-01",
            "9999-12-31",
        ] {
            assert!(is_date_or_empty(text), "{text:?}");
        }
        for text in [
            "2026-1-12",
            "2026-13-01",
            "2026-00-10",
            "2026-04-31",
            "2025-02-29",
            "1900-02-29",
            "0000-01-01",
            "2026-10-12T00:00:00Z",
            "12.10.2026",
            " 2026-10-12",
            "2026/10/12",
            "2026-10-1x",
        ] {
            assert!(!is_date_or_empty(text), "{text:?}");
        }
    }

    #[test]
    fn result_values_serialize_tagged_with_decimal_strings() {
        let value = ResultValue::Variable {
            value: dec("12.10"),
        };
        let json = serde_json::to_value(value).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "variable", "value": "12.10" })
        );
        assert_eq!(serde_json::from_value::<ResultValue>(json).unwrap(), value);
        let attribute = ResultValue::Attribute {
            value: AttributeValue::NotOk,
        };
        assert_eq!(
            serde_json::to_value(attribute).unwrap(),
            serde_json::json!({ "type": "attribute", "value": "not_ok" })
        );
        let float = serde_json::json!({ "type": "variable", "value": 12.1 });
        assert!(serde_json::from_value::<ResultValue>(float).is_err());
        assert_eq!(
            serde_json::to_value(ResultSource::Manual).unwrap(),
            serde_json::json!({ "type": "manual" })
        );
        assert!(
            ResultValue::Variable { value: dec("8.0") }
                .same_as(&ResultValue::Variable { value: dec("8.0") })
        );
        assert!(
            !ResultValue::Variable { value: dec("8.0") }
                .same_as(&ResultValue::Variable { value: dec("8.00") })
        );
    }
}
