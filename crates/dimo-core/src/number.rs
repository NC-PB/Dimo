//! Display numbers of characteristics: `12`, `12.1`, `12A` (data model `Characteristic.number`,
//! FR-BAL-07, FR-BAL-11, D-22, D-23).
//!
//! A [`DisplayNumber`] is a base number, an optional sub-number and an optional letter suffix.
//! It is stored and sent as its canonical text (`"12.1"`), so `project.json` stays readable and
//! the frontend shows it as it is. Numbers compare by base, then sub-number, then letter:
//! `12 < 12A < 12.1 < 12.1A < 12.2 < 13`.

use std::fmt;
use std::str::FromStr;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

use crate::zones::letters;

/// Regular expression of a display number, used in the JSON schema.
pub const DISPLAY_NUMBER_PATTERN: &str = r"^[1-9][0-9]*(\.[1-9][0-9]*)?[A-Z]*$";

/// The number shown in a balloon and in the characteristic list.
///
/// All parts are at least 1. The letter suffix counts like spreadsheet columns: 1 is `A`, 26 is
/// `Z`, 27 is `AA`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DisplayNumber {
    base: u32,
    sub: Option<u32>,
    letter: Option<u32>,
}

/// Text that is not a display number.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid display number {0:?}, expected for example 12, 12.1 or 12A")]
pub struct DisplayNumberError(String);

impl DisplayNumber {
    /// A plain number such as `12`. `base` 0 is raised to 1.
    pub fn plain(base: u32) -> Self {
        Self {
            base: base.max(1),
            sub: None,
            letter: None,
        }
    }

    /// The same base and sub-number with letter suffix `letter` (1 is `A`), for example `12A`.
    #[must_use]
    pub fn with_letter(self, letter: u32) -> Self {
        Self {
            letter: Some(letter.max(1)),
            ..self
        }
    }

    /// The same base with sub-number `sub` and no letter, for example `12.1`.
    #[must_use]
    pub fn with_sub(self, sub: u32) -> Self {
        Self {
            base: self.base,
            sub: Some(sub.max(1)),
            letter: None,
        }
    }

    /// The base number.
    pub fn base(self) -> u32 {
        self.base
    }

    /// The sub-number, if any.
    pub fn sub(self) -> Option<u32> {
        self.sub
    }

    /// The letter suffix as a count (1 is `A`), if any.
    pub fn letter(self) -> Option<u32> {
        self.letter
    }

    /// The base number if this is a plain number without sub-number or letter.
    pub fn as_plain(self) -> Option<u32> {
        (self.sub.is_none() && self.letter.is_none()).then_some(self.base)
    }

    /// Parses the canonical text: no leading zeros, upper case letters.
    pub fn parse(text: &str) -> Result<Self, DisplayNumberError> {
        let error = || DisplayNumberError(text.to_owned());
        let letters_at = text
            .find(|c: char| c.is_ascii_uppercase())
            .unwrap_or(text.len());
        let (digits, letters) = text.split_at(letters_at);
        let (base, sub) = match digits.split_once('.') {
            Some((base, sub)) => (base, Some(sub)),
            None => (digits, None),
        };
        let number = |s: &str| -> Option<u32> {
            let valid =
                !s.is_empty() && !s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit());
            valid.then(|| s.parse().ok()).flatten()
        };
        let base = number(base).ok_or_else(error)?;
        let sub = sub.map(|s| number(s).ok_or_else(error)).transpose()?;
        let letter = if letters.is_empty() {
            None
        } else {
            let mut value: u32 = 0;
            for b in letters.bytes() {
                if !b.is_ascii_uppercase() {
                    return Err(error());
                }
                value = value
                    .checked_mul(26)
                    .and_then(|v| v.checked_add(u32::from(b - b'A') + 1))
                    .ok_or_else(error)?;
            }
            Some(value)
        };
        Ok(Self { base, sub, letter })
    }
}

impl fmt::Display for DisplayNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base)?;
        if let Some(sub) = self.sub {
            write!(f, ".{sub}")?;
        }
        if let Some(letter) = self.letter {
            f.write_str(&letters(letter))?;
        }
        Ok(())
    }
}

impl FromStr for DisplayNumber {
    type Err = DisplayNumberError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl TryFrom<String> for DisplayNumber {
    type Error = DisplayNumberError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<DisplayNumber> for String {
    fn from(number: DisplayNumber) -> Self {
        number.to_string()
    }
}

impl JsonSchema for DisplayNumber {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DisplayNumber".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": DISPLAY_NUMBER_PATTERN,
            "description": "Display number: base, optional sub-number and letter suffix, e.g. \"12\", \"12.1\" or \"12A\"."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_and_parses_canonical_text() {
        for (text, base, sub, letter) in [
            ("1", 1, None, None),
            ("12", 12, None, None),
            ("12.1", 12, Some(1), None),
            ("12A", 12, None, Some(1)),
            ("12Z", 12, None, Some(26)),
            ("12AA", 12, None, Some(27)),
            ("12.10B", 12, Some(10), Some(2)),
            ("4294967295", u32::MAX, None, None),
        ] {
            let number = DisplayNumber::parse(text).unwrap();
            assert_eq!(
                (number.base(), number.sub(), number.letter()),
                (base, sub, letter),
                "{text}"
            );
            assert_eq!(number.to_string(), text);
        }
    }

    #[test]
    fn rejects_other_text() {
        for text in [
            "",
            "0",
            "012",
            "1.",
            ".1",
            "1.0",
            "1.01",
            "1a",
            "A",
            "1 ",
            "-1",
            "1.1.1",
            "1A1",
            "4294967296",
        ] {
            assert!(
                DisplayNumber::parse(text).is_err(),
                "{text:?} must be rejected"
            );
        }
    }

    #[test]
    fn orders_by_base_then_sub_then_letter() {
        let parse = |t: &str| DisplayNumber::parse(t).unwrap();
        let mut numbers: Vec<DisplayNumber> = ["13", "12.1", "12A", "12", "2", "12.1A", "12.2"]
            .into_iter()
            .map(parse)
            .collect();
        numbers.sort();
        let texts: Vec<String> = numbers.iter().map(ToString::to_string).collect();
        assert_eq!(texts, ["2", "12", "12A", "12.1", "12.1A", "12.2", "13"]);
    }

    #[test]
    fn serializes_as_text() {
        let number = DisplayNumber::plain(12).with_sub(3);
        assert_eq!(serde_json::to_string(&number).unwrap(), "\"12.3\"");
        assert_eq!(
            serde_json::from_str::<DisplayNumber>("\"12.3\"").unwrap(),
            number
        );
        assert!(serde_json::from_str::<DisplayNumber>("12").is_err());
        assert_eq!(DisplayNumber::plain(0).to_string(), "1");
        assert_eq!(DisplayNumber::plain(7).as_plain(), Some(7));
        assert_eq!(DisplayNumber::plain(7).with_letter(1).as_plain(), None);
    }
}
