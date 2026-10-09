//! ISO 286 tolerance class designations such as `H7`, `js6` or `ZC11` (FR-TOL-04).

use std::fmt;

use crate::format::Feature;

/// Deviation letters supported by the fit tables, in uppercase. `CD`, `EF` and `FG` (only used
/// for small sizes) are not supported.
pub const LETTERS: [&str; 25] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "J", "JS", "K", "M", "N", "P", "R", "S", "T", "U", "V",
    "X", "Y", "Z", "ZA", "ZB", "ZC",
];

/// A standard tolerance grade. `IT01` is stored as -1, `IT0` as 0, `ITn` as n, so grades
/// compare in their natural order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Grade(i8);

impl Grade {
    /// Parse the digits after `IT` or after the letter of a tolerance class: `01`, `0`, `1` to
    /// `18`, without other leading zeros.
    pub fn parse(digits: &str) -> Option<Self> {
        if digits == "01" {
            return Some(Self(-1));
        }
        if digits.is_empty()
            || !digits.bytes().all(|b| b.is_ascii_digit())
            || (digits.len() > 1 && digits.starts_with('0'))
        {
            return None;
        }
        let n: i8 = digits.parse().ok()?;
        (0..=18).contains(&n).then_some(Self(n))
    }

    /// The grade number: -1 for `IT01`, 0 for `IT0`, n for `ITn`.
    pub fn number(self) -> i8 {
        self.0
    }

    /// Grade `ITn` for n in 0 to 18.
    pub fn it(n: i8) -> Option<Self> {
        (0..=18).contains(&n).then_some(Self(n))
    }

    /// Column name in standard tolerance and delta parts, e.g. `IT7`.
    pub fn column(self) -> String {
        format!("IT{}", self.digits())
    }

    fn digits(self) -> String {
        if self.0 < 0 {
            "01".to_owned()
        } else {
            self.0.to_string()
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.column())
    }
}

/// A tolerance class such as `H7` (hole) or `js6` (shaft).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fit {
    feature: Feature,
    letter: &'static str,
    grade: Grade,
}

impl Fit {
    /// Parse a tolerance class: deviation letters, all uppercase for holes or all lowercase
    /// for shafts, followed by the grade digits. Returns `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let split = text.find(|c: char| c.is_ascii_digit())?;
        let (letters, digits) = text.split_at(split);
        let feature = if !letters.is_empty() && letters.chars().all(|c| c.is_ascii_uppercase()) {
            Feature::Hole
        } else if !letters.is_empty() && letters.chars().all(|c| c.is_ascii_lowercase()) {
            Feature::Shaft
        } else {
            return None;
        };
        let upper = letters.to_ascii_uppercase();
        let letter = LETTERS.iter().copied().find(|l| *l == upper)?;
        let grade = Grade::parse(digits)?;
        Some(Self {
            feature,
            letter,
            grade,
        })
    }

    /// Hole or shaft.
    pub fn feature(&self) -> Feature {
        self.feature
    }

    /// The deviation letters in uppercase, e.g. `JS` for `js6`.
    pub fn letter(&self) -> &'static str {
        self.letter
    }

    /// The deviation letters as written for this feature, e.g. `js` for `js6`.
    pub fn letter_as_written(&self) -> String {
        match self.feature {
            Feature::Hole => self.letter.to_owned(),
            Feature::Shaft => self.letter.to_ascii_lowercase(),
        }
    }

    /// The tolerance grade.
    pub fn grade(&self) -> Grade {
        self.grade
    }
}

impl fmt::Display for Fit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.letter_as_written(), self.grade.digits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades_parse_and_order() {
        assert_eq!(Grade::parse("01"), Some(Grade(-1)));
        assert_eq!(Grade::parse("0"), Some(Grade(0)));
        assert_eq!(Grade::parse("18"), Some(Grade(18)));
        for bad in ["", "19", "07", "001", "1a", "-1"] {
            assert_eq!(Grade::parse(bad), None, "{bad:?}");
        }
        assert!(Grade::parse("01") < Grade::parse("0"));
        assert_eq!(Grade(-1).column(), "IT01");
        assert_eq!(Grade(7).column(), "IT7");
    }

    #[test]
    fn fits_parse_and_print() {
        for text in ["H7", "h6", "js6", "JS7", "ZC11", "za8", "c10", "H01", "h0"] {
            assert_eq!(Fit::parse(text).unwrap().to_string(), text);
        }
        assert_eq!(Fit::parse("js6").unwrap().letter(), "JS");
        assert_eq!(Fit::parse("H7").unwrap().feature(), Feature::Hole);
        for bad in [
            "", "H", "7", "Js6", "hH7", "I7", "W7", "H19", "H07", "cd6", "H7 ",
        ] {
            assert_eq!(Fit::parse(bad), None, "{bad:?}");
        }
    }
}
