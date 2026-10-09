//! Exact decimal numbers as JSON strings (AGENTS.md rule 5).
//!
//! Nominals and limits are `rust_decimal::Decimal` and travel as strings such as `"30.0203"`,
//! never as JSON numbers, so no float ever touches them. The accepted text is strict: an optional
//! `-`, digits, and an optional fraction with `.` (no exponent, no `+`, no separators).
//! Leading zeros and `-0` are accepted and mean the same value as `0`.
//! The scale is kept, so `"90.0"` stays `"90.0"`.

use std::str::FromStr;

use rust_decimal::Decimal;
use schemars::{Schema, SchemaGenerator, json_schema};

/// Regular expression for decimal strings, used in the JSON schema.
pub const DECIMAL_PATTERN: &str = r"^-?[0-9]+(\.[0-9]+)?$";

/// Parse a strict decimal string (see module docs).
pub fn parse_decimal(text: &str) -> Option<Decimal> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (int, frac) = match digits.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (digits, None),
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(int) || frac.is_some_and(|f| !all_digits(f)) {
        return None;
    }
    Decimal::from_str(text).ok()
}

/// JSON schema of a decimal string.
pub fn decimal_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "string",
        "pattern": DECIMAL_PATTERN,
        "description": "Exact decimal number as a string, e.g. \"30.0203\" or \"-0.6\"."
    })
}

/// An optional exact decimal that serializes as a decimal string or `null`.
///
/// Used where a `#[serde(with)]` field attribute is not possible, such as the content of an
/// adjacently tagged enum variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct OptionalDecimal(
    #[cfg_attr(feature = "specta", specta(type = Option<String>))] pub Option<Decimal>,
);

impl From<Option<Decimal>> for OptionalDecimal {
    fn from(value: Option<Decimal>) -> Self {
        Self(value)
    }
}

impl From<Decimal> for OptionalDecimal {
    fn from(value: Decimal) -> Self {
        Self(Some(value))
    }
}

impl serde::Serialize for OptionalDecimal {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_str_nullable::serialize(&self.0, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for OptionalDecimal {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde_str_nullable::deserialize(deserializer).map(Self)
    }
}

impl schemars::JsonSchema for OptionalDecimal {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "OptionalDecimal".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        decimal_option_schema(generator)
    }
}

/// `#[serde(with = "crate::decimal::serde_str")]` for `Decimal` fields.
pub mod serde_str {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    /// Serialize as a decimal string with the stored scale.
    pub fn serialize<S: Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(value)
    }

    /// Deserialize from a strict decimal string.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        super::parse_decimal(&text).ok_or_else(|| {
            D::Error::custom(format!(
                "invalid decimal string {text:?}, expected digits with an optional '-' and '.'"
            ))
        })
    }
}

/// JSON schema of an optional decimal string: a decimal string or `null`.
pub fn decimal_option_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": ["string", "null"],
        "pattern": DECIMAL_PATTERN,
        "description": "Exact decimal number as a string, e.g. \"30.0203\", or null if absent."
    })
}

/// `#[serde(with = "crate::decimal::serde_str_option")]` for `Option<Decimal>` fields whose
/// absent value is an absent key: combine with
/// `#[serde(default, skip_serializing_if = "Option::is_none")]`. A present key must be a
/// decimal string; `null` is rejected (truth format).
pub mod serde_str_option {
    use rust_decimal::Decimal;
    use serde::{Deserializer, Serializer};

    /// Serialize `Some` as a decimal string, `None` as `null`.
    pub fn serialize<S: Serializer>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => super::serde_str::serialize(value, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize a present key as a strict decimal string.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Decimal>, D::Error> {
        super::serde_str::deserialize(deserializer).map(Some)
    }
}

/// `#[serde(with = "crate::decimal::serde_str_nullable")]` for `Option<Decimal>` fields that are
/// always present: `None` is written as `null`, and `null` or a decimal string are accepted
/// (project file and IPC, see [`decimal_option_schema`]).
pub mod serde_str_nullable {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    struct Text(#[serde(with = "super::serde_str")] Decimal);

    /// Serialize `Some` as a decimal string, `None` as `null`.
    pub fn serialize<S: Serializer>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => super::serde_str::serialize(value, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize `null` as `None` and a strict decimal string as `Some`.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Decimal>, D::Error> {
        Ok(Option::<Text>::deserialize(deserializer)?.map(|text| text.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_decimals_and_keeps_the_scale() {
        for text in ["0", "30", "30.0203", "-0.6", "90.0", "7.987", "007"] {
            let value = parse_decimal(text).unwrap();
            if text != "007" {
                assert_eq!(value.to_string(), text);
            }
        }
    }

    #[test]
    fn rejects_everything_else() {
        for text in [
            "", "-", ".5", "5.", "+1", "1e3", "1,5", " 1", "1 ", "1_000", "−0.6", "0x10", "NaN",
        ] {
            assert_eq!(parse_decimal(text), None, "{text:?} must be rejected");
        }
    }
}
