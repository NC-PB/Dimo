//! Human readable explanation of a derivation in English and German (FR-TOL-08, M2 decision 3).
//!
//! The project stores only the structured [`ToleranceDerivation`]; this module renders it with
//! the values of the characteristic. The text is plain and written for this program, not taken
//! from any standard.

use dimo_core::characteristic::{Characteristic, Unit};
use dimo_core::derivation::{
    DerivationHint, DerivationRule, RangeBound, SizeRange, TableLookup, TableRef,
    ToleranceDerivation, UnitConversion,
};
use dimo_core::proposal::Proposal;
use rust_decimal::Decimal;

use crate::interpret::Interpretation;

/// Language of an explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// English.
    English,
    /// German.
    German,
}

/// The values an explanation talks about: nominal, unit and deviations as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExplainValues {
    /// Nominal value.
    pub nominal: Option<Decimal>,
    /// Unit of nominal and deviations.
    pub unit: Option<Unit>,
    /// Upper deviation.
    pub upper_dev: Option<Decimal>,
    /// Lower deviation.
    pub lower_dev: Option<Decimal>,
    /// Upper limit, used when a deviation is missing.
    pub upper_limit: Option<Decimal>,
    /// Lower limit, used when a deviation is missing.
    pub lower_limit: Option<Decimal>,
}

impl From<&Characteristic> for ExplainValues {
    fn from(c: &Characteristic) -> Self {
        Self {
            nominal: c.nominal,
            unit: c.unit,
            upper_dev: c.upper_dev,
            lower_dev: c.lower_dev,
            upper_limit: c.upper_limit,
            lower_limit: c.lower_limit,
        }
    }
}

impl From<&Proposal> for ExplainValues {
    fn from(p: &Proposal) -> Self {
        Self {
            nominal: p.nominal,
            unit: p.unit,
            upper_dev: p.upper_dev,
            lower_dev: p.lower_dev,
            upper_limit: p.upper_limit,
            lower_limit: p.lower_limit,
        }
    }
}

impl From<&Interpretation> for ExplainValues {
    fn from(i: &Interpretation) -> Self {
        Self {
            nominal: i.nominal,
            unit: i.unit,
            upper_dev: i.upper_dev,
            lower_dev: i.lower_dev,
            upper_limit: i.upper_limit,
            lower_limit: i.lower_limit,
        }
    }
}

/// The explanation of a derivation, e.g. "No tolerance on the drawing. General tolerance
/// iso-2768-1 version 1, class m, part linear. Nominal 45 mm is in the range over 30 up to and
/// including 120 mm, so ±0.3 mm." (FR-TOL-08). Sentences are separated by one space.
pub fn explain(
    derivation: &ToleranceDerivation,
    values: &ExplainValues,
    language: Language,
) -> String {
    let t = Text { language };
    let mut out: Vec<String> = Vec::new();
    let marked = derivation.hints.iter().any(|h| {
        matches!(
            h,
            DerivationHint::ReferenceDimension | DerivationHint::BasicDimension
        )
    });
    if !(marked && derivation.rule == DerivationRule::NoToleranceDefined) {
        let blocked = derivation.hints.iter().any(|h| {
            matches!(
                h,
                DerivationHint::UnknownFit { .. } | DerivationHint::SizeOutsideTable { .. }
            )
        });
        if blocked && derivation.rule == DerivationRule::NoToleranceDefined {
            out.push(t.pick(
                "No rule gives limits for this dimension, so no limits. Check this characteristic.",
                "Keine Regel ergibt Grenzmaße für dieses Maß, daher keine Grenzmaße. Dieses Merkmal prüfen.",
            ));
        } else {
            out.push(t.rule(&derivation.rule, values));
        }
    }
    for hint in &derivation.hints {
        out.push(t.hint(hint));
    }
    if derivation.draft {
        out.push(t.pick(
            "The table is a draft and not verified yet.",
            "Die Tabelle ist ein Entwurf und noch nicht geprüft.",
        ));
    }
    if let Some(conversion) = &derivation.conversion {
        out.push(t.conversion(*conversion));
    }
    out.join(" ")
}

struct Text {
    language: Language,
}

impl Text {
    fn pick(&self, en: &str, de: &str) -> String {
        match self.language {
            Language::English => en.to_owned(),
            Language::German => de.to_owned(),
        }
    }

    fn rule(&self, rule: &DerivationRule, v: &ExplainValues) -> String {
        let no_tol = self.pick(
            "No tolerance on the drawing.",
            "Keine Toleranz in der Zeichnung.",
        );
        match rule {
            DerivationRule::Explicit => match self.language {
                Language::English => format!("Tolerance written on the drawing: {}.", self.devs(v)),
                Language::German => {
                    format!("Toleranz in der Zeichnung angegeben: {}.", self.devs(v))
                }
            },
            DerivationRule::Fit { table, fit, range } => {
                let source = match self.language {
                    Language::English => format!("Fit {fit} from table {}.", self.table(table)),
                    Language::German => {
                        format!("Passung {fit} aus Tabelle {}.", self.table(table))
                    }
                };
                format!("{source} {}", self.in_range(v, range.as_ref(), false))
            }
            DerivationRule::DrawingRule { lookup } => {
                let source = match self.language {
                    Language::English => {
                        format!("Drawing rule from table {}.", self.lookup(lookup))
                    }
                    Language::German => {
                        format!("Zeichnungsregel aus Tabelle {}.", self.lookup(lookup))
                    }
                };
                format!("{no_tol} {source} {}", self.in_range(v, Some(&lookup.range), true))
            }
            DerivationRule::General { lookup } => {
                let source = match self.language {
                    Language::English => format!("General tolerance {}.", self.lookup(lookup)),
                    Language::German => format!("Allgemeintoleranz {}.", self.lookup(lookup)),
                };
                format!("{no_tol} {source} {}", self.in_range(v, Some(&lookup.range), true))
            }
            DerivationRule::CustomTable { lookup } => {
                let source = match self.language {
                    Language::English => format!("Custom table {}.", self.lookup(lookup)),
                    Language::German => format!("Eigene Tabelle {}.", self.lookup(lookup)),
                };
                format!("{no_tol} {source} {}", self.in_range(v, Some(&lookup.range), true))
            }
            DerivationRule::DecimalRule { places, tolerance } => {
                let tol = self.with_unit(&format!("±{}", self.num(*tolerance)), v.unit);
                match self.language {
                    Language::English => format!(
                        "{no_tol} The nominal is written with {places} decimal places, so the decimal place rule gives {tol}."
                    ),
                    Language::German => format!(
                        "{no_tol} Das Nennmaß hat {places} Nachkommastellen, daher gilt nach der Regel für Nachkommastellen {tol}."
                    ),
                }
            }
            DerivationRule::NoToleranceDefined => self.pick(
                "No tolerance on the drawing and no general tolerance defined, so no limits. Check this characteristic.",
                "Keine Toleranz in der Zeichnung und keine Allgemeintoleranz festgelegt, daher keine Grenzmaße. Dieses Merkmal prüfen.",
            ),
            DerivationRule::Manual => {
                self.pick("Limits entered by hand.", "Grenzmaße von Hand eingegeben.")
            }
        }
    }

    fn hint(&self, hint: &DerivationHint) -> String {
        match hint {
            DerivationHint::FitDeviationsDiffer {
                fit,
                table_upper_dev,
                table_lower_dev,
            } => {
                let table = self.with_unit(
                    &format!(
                        "{} / {}",
                        self.signed_opt(*table_upper_dev),
                        self.signed_opt(*table_lower_dev)
                    ),
                    Some(Unit::Mm),
                );
                match self.language {
                    Language::English => format!(
                        "The written deviations differ from fit {fit} in the table ({table}). The written values are kept."
                    ),
                    Language::German => format!(
                        "Die angegebenen Abmaße weichen von der Passung {fit} in der Tabelle ab ({table}). Die angegebenen Werte gelten."
                    ),
                }
            }
            DerivationHint::UnknownFit { fit } => match self.language {
                Language::English => {
                    format!("Fit {fit} is not in the fit table, so it gives no limits.")
                }
                Language::German => format!(
                    "Die Passung {fit} ist nicht in der Passungstabelle, daher keine Grenzmaße daraus."
                ),
            },
            DerivationHint::SizeOutsideTable { table } => match self.language {
                Language::English => format!(
                    "The size is outside the ranges of table {}.",
                    self.table(table)
                ),
                Language::German => format!(
                    "Das Maß liegt außerhalb der Bereiche von Tabelle {}.",
                    self.table(table)
                ),
            },
            DerivationHint::ReferenceDimension => self.pick(
                "Reference dimension: no tolerance and not inspected unless chosen.",
                "Hilfsmaß: keine Toleranz und nur auf Wunsch geprüft.",
            ),
            DerivationHint::BasicDimension => self.pick(
                "Theoretically exact dimension: no tolerance and not inspected unless chosen.",
                "Theoretisch genaues Maß: keine Toleranz und nur auf Wunsch geprüft.",
            ),
        }
    }

    fn conversion(&self, c: UnitConversion) -> String {
        let (from, to) = (self.unit_name(c.from), self.unit_name(c.to));
        let places = c.places;
        match self.language {
            Language::English => format!(
                "Values converted from {from} to {to} and rounded to {places} decimal places."
            ),
            Language::German => format!(
                "Werte von {from} in {to} umgerechnet und auf {places} Nachkommastellen gerundet."
            ),
        }
    }

    fn table(&self, table: &TableRef) -> String {
        match self.language {
            Language::English => format!("{} version {}", table.id, table.version),
            Language::German => format!("{} Version {}", table.id, table.version),
        }
    }

    fn lookup(&self, lookup: &TableLookup) -> String {
        match self.language {
            Language::English => format!(
                "{}, class {}, part {}",
                self.table(&lookup.table),
                lookup.class,
                lookup.part
            ),
            Language::German => format!(
                "{}, Klasse {}, Teil {}",
                self.table(&lookup.table),
                lookup.class,
                lookup.part
            ),
        }
    }

    /// "Nominal 45 mm is in the range over 30 up to and including 120 mm, so ±0.3 mm." For
    /// angles the range is of the shorter leg.
    fn in_range(&self, v: &ExplainValues, range: Option<&SizeRange>, symmetric: bool) -> String {
        let devs = if symmetric {
            self.devs(v)
        } else {
            self.devs_split(v)
        };
        let Some(range) = range else {
            return match self.language {
                Language::English => format!("Deviations {devs}."),
                Language::German => format!("Abmaße {devs}."),
            };
        };
        let range = self.range(range);
        if v.unit == Some(Unit::Deg) {
            return match self.language {
                Language::English => {
                    format!("The shorter leg is in the range {range} mm, so {devs}.")
                }
                Language::German => {
                    format!("Der kürzere Schenkel liegt im Bereich {range} mm, daher {devs}.")
                }
            };
        }
        let nominal = v
            .nominal
            .map(|n| self.with_unit(&self.num(n), v.unit))
            .unwrap_or_default();
        let table_unit = if v.unit == Some(Unit::In) { " mm" } else { "" };
        let range_unit = if table_unit.is_empty() {
            self.with_unit(&range, v.unit)
        } else {
            format!("{range}{table_unit}")
        };
        match self.language {
            Language::English => {
                format!("Nominal {nominal} is in the range {range_unit}, so {devs}.")
            }
            Language::German => {
                format!("Nennmaß {nominal} liegt im Bereich {range_unit}, daher {devs}.")
            }
        }
    }

    fn range(&self, range: &SizeRange) -> String {
        let mut parts = Vec::new();
        let (from, over, upto, below, any) = match self.language {
            Language::English => ("from", "over", "up to and including", "below", "any size"),
            Language::German => ("ab", "über", "bis einschließlich", "unter", "jede Größe"),
        };
        match range.min {
            Some(RangeBound {
                value,
                inclusive: true,
            }) => parts.push(format!("{from} {}", self.num(value))),
            Some(RangeBound { value, .. }) => parts.push(format!("{over} {}", self.num(value))),
            None => {}
        }
        match range.max {
            Some(RangeBound {
                value,
                inclusive: true,
            }) => parts.push(format!("{upto} {}", self.num(value))),
            Some(RangeBound { value, .. }) => parts.push(format!("{below} {}", self.num(value))),
            None => {}
        }
        if parts.is_empty() {
            parts.push(any.to_owned());
        }
        parts.join(" ")
    }

    /// `±0.1 mm` for symmetric deviations, else `+0.0203 / 0 mm`.
    fn devs(&self, v: &ExplainValues) -> String {
        if let (Some(u), Some(l)) = (v.upper_dev, v.lower_dev)
            && u > Decimal::ZERO
            && u == -l
        {
            return self.with_unit(&format!("±{}", self.num(u)), v.unit);
        }
        self.devs_split(v)
    }

    /// `+0.0203 / 0 mm`; one sided limits as `lower limit 10 mm` or `upper limit 10 mm`.
    #[allow(
        clippy::single_match_else,
        reason = "the two cases read better as a match"
    )]
    fn devs_split(&self, v: &ExplainValues) -> String {
        match (v.upper_dev, v.lower_dev) {
            (Some(u), Some(l)) => {
                self.with_unit(&format!("{} / {}", self.signed(u), self.signed(l)), v.unit)
            }
            _ => {
                let mut parts = Vec::new();
                let (upper, lower, none) = match self.language {
                    Language::English => ("upper limit", "lower limit", "no limits"),
                    Language::German => ("Höchstmaß", "Mindestmaß", "keine Grenzmaße"),
                };
                if let Some(l) = v.lower_limit {
                    parts.push(format!("{lower} {}", self.with_unit(&self.num(l), v.unit)));
                }
                if let Some(u) = v.upper_limit {
                    parts.push(format!("{upper} {}", self.with_unit(&self.num(u), v.unit)));
                }
                if parts.is_empty() {
                    none.to_owned()
                } else {
                    parts.join(", ")
                }
            }
        }
    }

    fn signed_opt(&self, value: Option<Decimal>) -> String {
        value.map_or_else(|| "?".to_owned(), |v| self.signed(v))
    }

    /// `+0.1`, `-0.1` or `0`.
    fn signed(&self, value: Decimal) -> String {
        if value.is_zero() {
            "0".to_owned()
        } else if value.is_sign_positive() {
            format!("+{}", self.num(value))
        } else {
            format!("-{}", self.num(value.abs()))
        }
    }

    /// A number with its stored digits; German uses a decimal comma.
    fn num(&self, value: Decimal) -> String {
        let text = if value.is_zero() {
            "0".to_owned()
        } else {
            value.to_string()
        };
        match self.language {
            Language::English => text,
            Language::German => text.replace('.', ","),
        }
    }

    #[allow(
        clippy::unused_self,
        reason = "kept a method so all formatting reads alike"
    )]
    /// Unit symbol; German writes inch as `Zoll`.
    fn unit_name(&self, unit: Unit) -> &'static str {
        match (self.language, unit) {
            (Language::German, Unit::In) => "Zoll",
            _ => unit_name_en(unit),
        }
    }

    fn with_unit(&self, text: &str, unit: Option<Unit>) -> String {
        match unit {
            Some(Unit::Deg) => format!("{text}°"),
            Some(unit) => format!("{text} {}", self.unit_name(unit)),
            None => text.to_owned(),
        }
    }
}

fn unit_name_en(unit: Unit) -> &'static str {
    match unit {
        Unit::Mm => "mm",
        Unit::In => "in",
        Unit::Deg => "°",
    }
}
