//! Samples and results (FR-MEA-01, FR-MEA-02, FR-MEA-06, M3 decisions 4 to 8 and 16).

use std::collections::BTreeSet;

use super::{CommandError, Tx, index_u32, known_chars};
use crate::env::Environment;
use crate::id::{CharId, SampleId};
use crate::measurement::{
    MAX_SAMPLES, MAX_SAMPLES_PER_ADD, MeasurementResult, ResultCell, ResultEntry, ResultSource,
    ResultType, Sample, SampleField, check_serial, is_date_or_empty, next_serials, parse_result,
};
use crate::patch::Change;
use crate::project::Project;

/// Clears the results of cells. Cells without a result are left as they are.
pub(super) fn clear_results(tx: &mut Tx<'_>, cells: &[ResultCell]) -> Result<(), CommandError> {
    cells.iter().try_for_each(|cell| {
        check_cell(tx.project, *cell)?;
        set_result(tx, *cell, None)
    })
}

/// Distinct sample IDs in first seen order, all checked to exist.
fn known_samples(project: &Project, ids: &[SampleId]) -> Result<Vec<SampleId>, CommandError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for &id in ids {
        if project.sample(id).is_none() {
            return Err(CommandError::UnknownSample(id));
        }
        if seen.insert(id) {
            out.push(id);
        }
    }
    Ok(out)
}

/// Checks that sample and characteristic of a cell exist.
fn check_cell(project: &Project, cell: ResultCell) -> Result<(), CommandError> {
    if project.sample(cell.sample).is_none() {
        return Err(CommandError::UnknownSample(cell.sample));
    }
    if project.characteristic(cell.characteristic).is_none() {
        return Err(CommandError::UnknownCharacteristic(cell.characteristic));
    }
    Ok(())
}

/// Sets or clears the result of one cell; records a change only if value, digits, source or
/// comment differ.
pub(super) fn set_result(
    tx: &mut Tx<'_>,
    cell: ResultCell,
    after: Option<MeasurementResult>,
) -> Result<(), CommandError> {
    let before = tx.project.result(cell).cloned();
    let same = match (&before, &after) {
        (Some(a), Some(b)) => {
            a.value.same_as(&b.value) && a.source == b.source && a.comment == b.comment
        }
        (None, None) => true,
        _ => false,
    };
    if !same {
        tx.push(Change::ResultChanged {
            cell,
            before,
            after,
        })?;
    }
    Ok(())
}

pub(super) fn add_samples(
    tx: &mut Tx<'_>,
    count: u32,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    if !(1..=MAX_SAMPLES_PER_ADD).contains(&count) {
        return Err(CommandError::InvalidSampleCount);
    }
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if tx.project.samples.len().saturating_add(count) > MAX_SAMPLES {
        return Err(CommandError::TooManySamples);
    }
    for serial in next_serials(&tx.project.samples, count) {
        let sample = Sample {
            id: SampleId::from_uuid(env.new_uuid()),
            serial,
            date: String::new(),
            inspector: env.user_name(),
            gauges: String::new(),
            comment: String::new(),
        };
        tx.push(Change::SampleInserted {
            index: index_u32(tx.project.samples.len()),
            sample,
        })?;
    }
    Ok(())
}

/// Sets fields of a sample (MF F4). The resulting serial is trimmed and checked, the date must
/// be a date or empty.
pub(super) fn update_sample(
    tx: &mut Tx<'_>,
    id: SampleId,
    values: &[SampleField],
) -> Result<(), CommandError> {
    let current = tx
        .project
        .sample(id)
        .ok_or(CommandError::UnknownSample(id))?;
    let mut after = current.clone();
    for v in values {
        after.set(v);
    }
    after.serial = check_serial(&tx.project.samples, &after.serial, Some(id))?;
    if !is_date_or_empty(&after.date) {
        return Err(CommandError::InvalidDate(after.date));
    }
    if *current != after {
        tx.push(Change::SampleChanged {
            before: current.clone(),
            after,
        })?;
    }
    Ok(())
}

pub(super) fn delete_samples(tx: &mut Tx<'_>, ids: &[SampleId]) -> Result<(), CommandError> {
    for id in known_samples(tx.project, ids)? {
        // M3 decision 16: results go with their sample, in the same undo step.
        let cells: Vec<ResultCell> = tx
            .project
            .results
            .iter()
            .filter(|r| r.sample == id)
            .map(MeasurementResult::cell)
            .collect();
        for cell in cells {
            set_result(tx, cell, None)?;
        }
        if let Some(index) = tx.project.samples.iter().position(|s| s.id == id) {
            let sample = tx.project.samples[index].clone();
            tx.push(Change::SampleRemoved {
                index: index_u32(index),
                sample,
            })?;
        }
    }
    Ok(())
}

pub(super) fn move_samples(
    tx: &mut Tx<'_>,
    ids: &[SampleId],
    before: Option<SampleId>,
) -> Result<(), CommandError> {
    let moved: BTreeSet<SampleId> = known_samples(tx.project, ids)?.into_iter().collect();
    if let Some(target) = before {
        if tx.project.sample(target).is_none() {
            return Err(CommandError::UnknownSample(target));
        }
        if moved.contains(&target) {
            return Err(CommandError::InvalidMoveTarget);
        }
    }
    let old: Vec<SampleId> = tx.project.samples.iter().map(|s| s.id).collect();
    let (picked, mut new): (Vec<SampleId>, Vec<SampleId>) =
        old.iter().copied().partition(|id| moved.contains(id));
    let at = before
        .and_then(|target| new.iter().position(|id| *id == target))
        .unwrap_or(new.len());
    new.splice(at..at, picked);
    if new != old {
        tx.push(Change::SampleOrderChanged {
            before: old,
            after: new,
        })?;
    }
    Ok(())
}

pub(super) fn set_results(tx: &mut Tx<'_>, entries: &[ResultEntry]) -> Result<(), CommandError> {
    for entry in entries {
        let cell = ResultCell {
            sample: entry.sample,
            characteristic: entry.characteristic,
        };
        check_cell(tx.project, cell)?;
        if entry.text.trim().is_empty() {
            set_result(tx, cell, None)?;
            continue;
        }
        let Some(characteristic) = tx.project.characteristic(cell.characteristic) else {
            return Err(CommandError::UnknownCharacteristic(cell.characteristic));
        };
        let effective = characteristic.effective_result_type();
        let value =
            parse_result(&entry.text, effective).map_err(|error| CommandError::InvalidResult {
                characteristic: cell.characteristic,
                error,
            })?;
        // M3 decision 4 (owner addition): a result pins an `auto` characteristic to its
        // effective type in the same undo step, so a later limit change never flips it.
        if characteristic.result_type == ResultType::Auto {
            // Only a project file written before pinning existed can hold results on an
            // `auto` characteristic; never mix types in one row.
            if tx
                .project
                .results_of(cell.characteristic)
                .any(|r| r.value.result_type() != effective)
            {
                return Err(CommandError::ResultTypeConflict(cell.characteristic));
            }
            let before = Box::new(characteristic.clone());
            let mut after = before.clone();
            after.result_type = effective;
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
        let comment = match &entry.comment {
            Some(comment) => comment.clone(),
            None => tx
                .project
                .result(cell)
                .map(|r| r.comment.clone())
                .unwrap_or_default(),
        };
        let result = MeasurementResult {
            sample: cell.sample,
            characteristic: cell.characteristic,
            value,
            source: ResultSource::Manual,
            comment,
        };
        set_result(tx, cell, Some(result))?;
    }
    Ok(())
}

pub(super) fn set_result_type(
    tx: &mut Tx<'_>,
    ids: &[CharId],
    result_type: ResultType,
) -> Result<(), CommandError> {
    for id in known_chars(tx.project, ids)? {
        let Some(current) = tx.project.characteristic(id) else {
            continue;
        };
        let before = Box::new(current.clone());
        let mut after = before.clone();
        after.result_type = result_type;
        let effective = after.effective_result_type();
        let has_results = tx.project.results_of(id).next().is_some();
        if tx
            .project
            .results_of(id)
            .any(|r| r.value.result_type() != effective)
        {
            return Err(CommandError::ResultTypeConflict(id));
        }
        // M3 decision 4: `auto` never holds while results exist, else a later limit change
        // would flip the type under them. Owner decision 2026-10-12: going back to `auto` is
        // refused instead of silently keeping the pinned type; clear the results first.
        if has_results && result_type == ResultType::Auto && current.result_type != ResultType::Auto
        {
            return Err(CommandError::ResultTypeConflict(id));
        }
        if after != before {
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
    }
    Ok(())
}
