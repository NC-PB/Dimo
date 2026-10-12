//! Pass/fail evaluation of measurement results (FR-MEA-03, FR-MEA-06, M3 decisions 2 to 5).
//!
//! Verdicts are computed from the entered value and the current limits whenever they are
//! needed and never stored (decision 2). The rules of decision 3:
//!
//! - Limits are inclusive and compared exactly on `Decimal`; the value is never rounded, so
//!   `12` equals `12.000`. No measurement uncertainty.
//! - Only an upper or only a lower limit: checked against that limit.
//! - A variable result without any limit is [`Verdict::NotEvaluated`].
//! - An attribute result passes when `ok` and fails when `not_ok`; limits are ignored.
//! - No result is [`Verdict::Missing`].
//! - Per sample, per characteristic and overall: [`OverallVerdict::Fail`] if any cell fails, else
//!   [`OverallVerdict::Pass`] if every cell passes, else [`OverallVerdict::Open`]. An empty set (no samples
//!   or no measured characteristic) is open: nothing was shown to pass.
//!
//! Limits from draft tables (D-43) evaluate like any other; [`RowState::draft`] lets the grid
//! and the reports mark them.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::characteristic::{Characteristic, CharacteristicStatus};
use crate::id::{CharId, SampleId};
use crate::measurement::{AttributeValue, MeasurementResult, ResultType, ResultValue};
use crate::project::Project;

/// Verdict of one cell: one sample and one characteristic (FR-MEA-03, decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Within the limits, or attribute OK.
    Pass,
    /// Outside a limit, or attribute not OK.
    Fail,
    /// A measured value for a characteristic without limits: neither pass nor fail.
    NotEvaluated,
    /// No result entered.
    Missing,
}

/// Verdict of a group of cells: a sample, a characteristic or the whole report (decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum OverallVerdict {
    /// Every cell passes.
    Pass,
    /// At least one cell fails.
    Fail,
    /// No cell fails, but some are missing or not evaluated, or there are no cells.
    Open,
}

/// Number of cells per verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct VerdictCounts {
    /// Cells that pass.
    pub pass: u32,
    /// Cells that fail.
    pub fail: u32,
    /// Cells with a value but no limits.
    pub not_evaluated: u32,
    /// Cells without a result.
    pub missing: u32,
}

impl VerdictCounts {
    /// Counts one more cell.
    pub fn add(&mut self, verdict: Verdict) {
        let n = match verdict {
            Verdict::Pass => &mut self.pass,
            Verdict::Fail => &mut self.fail,
            Verdict::NotEvaluated => &mut self.not_evaluated,
            Verdict::Missing => &mut self.missing,
        };
        *n = n.saturating_add(1);
    }

    /// Total number of cells counted.
    pub fn total(&self) -> u32 {
        self.pass
            .saturating_add(self.fail)
            .saturating_add(self.not_evaluated)
            .saturating_add(self.missing)
    }

    /// The summary of these cells: fail if any fails, pass if all pass (and there is at least
    /// one), else open (decision 3).
    pub fn verdict(&self) -> OverallVerdict {
        if self.fail > 0 {
            OverallVerdict::Fail
        } else if self.pass > 0 && self.pass == self.total() {
            OverallVerdict::Pass
        } else {
            OverallVerdict::Open
        }
    }
}

/// The verdict of `result` for `characteristic` with its current limits (decision 3).
///
/// The value decides how it is evaluated, not the result type: commands keep the two in step
/// (decision 4), and a value is never converted.
pub fn evaluate(characteristic: &Characteristic, result: Option<&MeasurementResult>) -> Verdict {
    match result.map(|r| r.value) {
        None => Verdict::Missing,
        Some(ResultValue::Attribute {
            value: AttributeValue::Ok,
        }) => Verdict::Pass,
        Some(ResultValue::Attribute {
            value: AttributeValue::NotOk,
        }) => Verdict::Fail,
        Some(ResultValue::Variable { value }) => {
            let (upper, lower) = (characteristic.upper_limit, characteristic.lower_limit);
            if upper.is_none() && lower.is_none() {
                return Verdict::NotEvaluated;
            }
            // Decimal comparison is by value, independent of scale: 12 == 12.000.
            let above = upper.is_some_and(|u| value > u);
            let below = lower.is_some_and(|l| value < l);
            if above || below {
                Verdict::Fail
            } else {
                Verdict::Pass
            }
        }
    }
}

/// True if the characteristic is measured: inspected and not rejected (decision 5).
pub fn is_measured(characteristic: &Characteristic) -> bool {
    characteristic.inspect && characteristic.status != CharacteristicStatus::Rejected
}

/// The measured characteristics in number order (decision 5): what the grid, the check sheet
/// and the inspection report list.
pub fn measured_characteristics(project: &Project) -> Vec<&Characteristic> {
    let mut chars: Vec<&Characteristic> = project
        .characteristics
        .iter()
        .filter(|c| is_measured(c))
        .collect();
    // Stable: equal numbers (not possible through commands) keep placement order.
    chars.sort_by_key(|c| c.number);
    chars
}

/// Evaluation of one measured characteristic over all samples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct RowState {
    /// The characteristic.
    pub characteristic: CharId,
    /// Effective result type: variable or attribute, never auto (decision 4).
    pub result_type: ResultType,
    /// True if the limits came from a draft tolerance table (D-43).
    pub draft: bool,
    /// One verdict per sample, in sample order.
    pub cells: Vec<Verdict>,
    /// Cells per verdict.
    pub counts: VerdictCounts,
    /// Fail, pass or open over all samples.
    pub verdict: OverallVerdict,
}

/// Evaluation of one sample over all measured characteristics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct SampleState {
    /// The sample.
    pub sample: SampleId,
    /// Cells per verdict.
    pub counts: VerdictCounts,
    /// Fail, pass or open for this part.
    pub verdict: OverallVerdict,
}

/// Verdicts and summaries of the whole project (FR-MEA-03): what the measurement grid shows
/// and the inspection report prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct MeasurementState {
    /// Measured characteristics in number order (decision 5).
    pub rows: Vec<RowState>,
    /// Samples in column order.
    pub samples: Vec<SampleState>,
    /// Cells per verdict over the whole grid.
    pub counts: VerdictCounts,
    /// Overall verdict of the report.
    pub verdict: OverallVerdict,
}

/// Evaluates every cell of the measurement grid of `project`.
pub fn measurement_state(project: &Project) -> MeasurementState {
    let results: HashMap<(SampleId, CharId), &MeasurementResult> = project
        .results
        .iter()
        .map(|r| ((r.sample, r.characteristic), r))
        .collect();
    let mut sample_counts = vec![VerdictCounts::default(); project.samples.len()];
    let mut counts = VerdictCounts::default();
    let rows = measured_characteristics(project)
        .into_iter()
        .map(|c| {
            let mut row_counts = VerdictCounts::default();
            let cells = project
                .samples
                .iter()
                .zip(sample_counts.iter_mut())
                .map(|(sample, sample_counts)| {
                    let verdict = evaluate(c, results.get(&(sample.id, c.id)).copied());
                    row_counts.add(verdict);
                    sample_counts.add(verdict);
                    counts.add(verdict);
                    verdict
                })
                .collect();
            RowState {
                characteristic: c.id,
                result_type: c.effective_result_type(),
                draft: c.derivation.as_ref().is_some_and(|d| d.draft),
                cells,
                counts: row_counts,
                verdict: row_counts.verdict(),
            }
        })
        .collect();
    let samples = project
        .samples
        .iter()
        .zip(sample_counts)
        .map(|(sample, counts)| SampleState {
            sample: sample.id,
            counts,
            verdict: counts.verdict(),
        })
        .collect();
    MeasurementState {
        rows,
        samples,
        counts,
        verdict: counts.verdict(),
    }
}
