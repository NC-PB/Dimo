//! Expansion of ISO 286 tolerance classes to deviations (T2.3, FR-TOL-04, D-43).
//!
//! The values come from a fit table (`data/tolerances/iso-286.toml`); this module holds only
//! the rules of the ISO system that combine them:
//!
//! - shafts a to h: es from the table, ei = es - IT;
//! - shafts j (by grade), k, m to zc: ei from the table, es = ei + IT; k uses column `k` for
//!   IT4 to IT7 and `k_other` for all other grades;
//! - holes A to H: EI from the table, ES = EI + IT;
//! - holes J (by grade): ES from the table, EI = ES - IT;
//! - holes K, M, N: up to IT8 ES = table value + delta, above IT8 the `*_above_IT8` column;
//! - holes P to ZC: up to IT7 ES = table value + delta, above IT7 the table value;
//! - js and JS: +IT/2 and -IT/2; for IT7 to IT11 an odd IT value in micrometres is first
//!   reduced to the even value below;
//! - an `override` part replaces the fundamental deviation of one tolerance class in a range
//!   (M6 from 250 to 315 mm);
//! - a size exactly on a range bound belongs to the row whose bound is inclusive ("over 18 up to
//!   and including 30" holds 30, not 18).

use rust_decimal::Decimal;

use crate::designation::{Fit, Grade};
use crate::format::{Deviation, Feature, PartKind, TableKind};
use crate::table::{CellValue, LookupError, Range, Table, TablePart};

/// The deviations of a tolerance class at a nominal size (FR-TOL-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitLimits {
    /// The tolerance class.
    pub fit: Fit,
    /// Table id.
    pub table: String,
    /// Table version.
    pub version: u32,
    /// Whether the table is still a draft (D-43 badge).
    pub draft: bool,
    /// Upper deviation in mm (es or ES).
    pub upper: Decimal,
    /// Lower deviation in mm (ei or EI).
    pub lower: Decimal,
    /// The standard tolerance IT in mm.
    pub standard_tolerance: Decimal,
    /// The fundamental deviation in mm, including delta; `None` for js and JS.
    pub fundamental_deviation: Option<Decimal>,
    /// The delta value in mm when one was added.
    pub delta: Option<Decimal>,
    /// The size range in which all table values used here are constant: the step of the
    /// standard that contains the nominal size, for explanations.
    pub range: Range,
}

impl FitLimits {
    /// Upper and lower limit of a nominal size: nominal plus the deviations.
    pub fn limits(&self, nominal: Decimal) -> (Decimal, Decimal) {
        (nominal + self.upper, nominal + self.lower)
    }
}

/// Why a tolerance class could not be expanded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FitError {
    /// The text is not a tolerance class such as `H7` or `js6`.
    #[error("{0:?} is not a tolerance class such as H7 or g6")]
    Designation(String),
    /// The table is not a fit table.
    #[error("table {0} is not a fit table")]
    NotAFitTable(String),
    /// The table has no value for this class and size.
    #[error("{fit}: {source}")]
    Lookup {
        /// The tolerance class.
        fit: String,
        /// What is missing.
        source: Box<LookupError>,
    },
    /// The class needs a delta value that the table does not define for the grade.
    #[error("{fit}: no delta value for {grade}")]
    NoDelta {
        /// The tolerance class.
        fit: String,
        /// The grade.
        grade: String,
    },
}

/// One micrometre in mm.
const MICRON: Decimal = Decimal::from_parts(1, 0, 0, false, 3);

impl Table {
    /// Expand a tolerance class such as `H7` at a nominal size in mm to its deviations
    /// (FR-TOL-04). Exact decimals throughout.
    pub fn fit_limits(&self, fit: &str, nominal: Decimal) -> Result<FitLimits, FitError> {
        let fit = Fit::parse(fit).ok_or_else(|| FitError::Designation(fit.to_owned()))?;
        self.expand(&fit, nominal)
    }

    /// [`Table::fit_limits`] for a parsed tolerance class.
    pub fn expand(&self, fit: &Fit, nominal: Decimal) -> Result<FitLimits, FitError> {
        if self.header().kind != TableKind::Fit {
            return Err(FitError::NotAFitTable(self.id().to_owned()));
        }
        let name = fit.to_string();
        let lookup = |source: LookupError| FitError::Lookup {
            fit: name.clone(),
            source: Box::new(source),
        };
        let grade = fit.grade();
        let it = self
            .fit_part(PartKind::StandardTolerance, None, None)
            .lookup_span(self.id(), &grade.column(), nominal)
            .map_err(lookup)?;
        let mut range = it.range;
        let mut delta = None;

        // Fundamental deviation and whether it is the upper one.
        let fundamental: Option<(Decimal, bool)> = match (fit.feature(), fit.letter()) {
            (_, "JS") => None,
            (Feature::Shaft, letter) if a_to_h(letter) => {
                let found = self.deviation(
                    Feature::Shaft,
                    Deviation::Upper,
                    &fit.letter_as_written(),
                    nominal,
                );
                let found = found.map_err(lookup)?;
                range = range.intersect(&found.range);
                Some((found.value, true))
            }
            (Feature::Shaft, letter) => {
                let column = match letter {
                    "J" => fit.to_string(),
                    "K" if (4..=7).contains(&grade.number()) => "k".to_owned(),
                    "K" => "k_other".to_owned(),
                    _ => fit.letter_as_written(),
                };
                let found = self.deviation(Feature::Shaft, Deviation::Lower, &column, nominal);
                let found = found.map_err(lookup)?;
                range = range.intersect(&found.range);
                Some((found.value, false))
            }
            (Feature::Hole, letter) if a_to_h(letter) => {
                let found = self.deviation(Feature::Hole, Deviation::Lower, letter, nominal);
                let found = found.map_err(lookup)?;
                range = range.intersect(&found.range);
                Some((found.value, false))
            }
            (Feature::Hole, letter) => {
                // Highest grade that still gets delta: IT8 for K, M, N, IT7 for P to ZC.
                let delta_up_to = match letter {
                    "J" => None,
                    "K" | "M" | "N" => Some(8),
                    _ => Some(7),
                };
                let with_delta = delta_up_to.is_some_and(|g| grade.number() <= g);
                let column = match letter {
                    "J" => fit.to_string(),
                    "K" | "M" | "N" if !with_delta => format!("{letter}_above_IT8"),
                    _ => letter.to_owned(),
                };
                let found = self.deviation(Feature::Hole, Deviation::Upper, &column, nominal);
                let found = found.map_err(lookup)?;
                range = range.intersect(&found.range);
                let mut value = found.value;
                if with_delta {
                    let d = self.delta(&name, grade, nominal)?;
                    range = range.intersect(&d.range);
                    value += d.value;
                    delta = Some(d.value);
                }
                Some((value, true))
            }
        };

        // Special cases replace the computed fundamental deviation.
        let fundamental = match (fundamental, self.special_case(&name, nominal)) {
            (Some((_, is_upper)), Some(special)) => {
                range = range.intersect(&special.range);
                delta = None;
                Some((special.value, is_upper))
            }
            (other, _) => other,
        };

        let (upper, lower) = match fundamental {
            None => {
                let half = js_half(it.value, grade);
                (half, -half)
            }
            Some((value, true)) => (value, value - it.value),
            Some((value, false)) => (value + it.value, value),
        };
        Ok(FitLimits {
            fit: fit.clone(),
            table: self.id().to_owned(),
            version: self.header().version,
            draft: self.is_draft(),
            upper,
            lower,
            standard_tolerance: it.value,
            fundamental_deviation: fundamental.map(|(v, _)| v),
            delta,
            range,
        })
    }

    fn fit_part(
        &self,
        kind: PartKind,
        feature: Option<Feature>,
        deviation: Option<Deviation>,
    ) -> &TablePart {
        // Table::parse refuses fit tables without these parts.
        #[allow(clippy::expect_used, reason = "invariant checked by Table::parse")]
        self.part_by_role(kind, feature, deviation)
            .expect("fit tables have all parts")
    }

    fn deviation(
        &self,
        feature: Feature,
        deviation: Deviation,
        column: &str,
        nominal: Decimal,
    ) -> Result<CellValue, LookupError> {
        self.fit_part(
            PartKind::FundamentalDeviation,
            Some(feature),
            Some(deviation),
        )
        .lookup_span(self.id(), column, nominal)
    }

    fn delta(&self, fit: &str, grade: Grade, nominal: Decimal) -> Result<CellValue, FitError> {
        let part = self.fit_part(PartKind::Delta, None, None);
        match part.lookup_span(self.id(), &grade.column(), nominal) {
            Ok(found) => Ok(found),
            Err(LookupError::UnknownColumn { .. } | LookupError::NotDefined { .. }) => {
                Err(FitError::NoDelta {
                    fit: fit.to_owned(),
                    grade: grade.column(),
                })
            }
            Err(source) => Err(FitError::Lookup {
                fit: fit.to_owned(),
                source: Box::new(source),
            }),
        }
    }

    fn special_case(&self, fit: &str, nominal: Decimal) -> Option<CellValue> {
        self.part_by_role(PartKind::Override, None, None)?
            .lookup_span(self.id(), fit, nominal)
            .ok()
    }
}

/// Half the standard tolerance for js and JS. For IT7 to IT11 an odd number of micrometres is
/// reduced to the even number below first, so the deviations stay whole micrometres.
fn js_half(it: Decimal, grade: Grade) -> Decimal {
    let microns = it / MICRON;
    let odd = microns.fract().is_zero() && !(microns % Decimal::TWO).is_zero();
    let it = if (7..=11).contains(&grade.number()) && odd {
        it - MICRON
    } else {
        it
    };
    it / Decimal::TWO
}

/// Letters A to H (a to h), for which the table holds EI of holes and es of shafts.
fn a_to_h(letter: &str) -> bool {
    matches!(letter, "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    #[test]
    fn js_half_rounds_odd_microns_for_it7_to_it11() {
        let g = |n| Grade::it(n).unwrap();
        assert_eq!(js_half(d("0.015"), g(7)), d("0.007"));
        assert_eq!(js_half(d("0.021"), g(7)), d("0.010"));
        assert_eq!(js_half(d("0.018"), g(7)), d("0.009"));
        assert_eq!(js_half(d("0.009"), g(6)), d("0.0045"));
        assert_eq!(js_half(d("0.025"), g(12)), d("0.0125"));
        assert_eq!(js_half(d("0.0025"), g(7)), d("0.00125"));
    }
}
