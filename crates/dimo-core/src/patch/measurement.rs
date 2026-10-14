//! Appliers for samples and results (data model version 3).

use std::collections::HashMap;

use super::{ChangeError, ensure, is_permutation};
use crate::id::{CharId, SampleId};
use crate::measurement::{MeasurementResult, ResultCell, Sample};
use crate::project::Project;

impl Project {
    pub(super) fn insert_sample(
        &mut self,
        index: usize,
        sample: &Sample,
    ) -> Result<(), ChangeError> {
        ensure(index <= self.samples.len(), "insert index out of range")?;
        ensure(self.sample(sample.id).is_none(), "sample ID exists")?;
        self.check_serial(sample)?;
        self.samples.insert(index, sample.clone());
        Ok(())
    }

    pub(super) fn remove_sample(
        &mut self,
        index: usize,
        sample: &Sample,
    ) -> Result<(), ChangeError> {
        ensure(
            self.samples.get(index) == Some(sample),
            "removed sample differs",
        )?;
        ensure(
            !self.results.iter().any(|r| r.sample == sample.id),
            "sample still has results",
        )?;
        self.samples.remove(index);
        Ok(())
    }

    pub(super) fn change_sample(
        &mut self,
        before: &Sample,
        after: &Sample,
    ) -> Result<(), ChangeError> {
        ensure(before.id == after.id, "sample ID changed")?;
        self.check_serial(after)?;
        let slot = self
            .samples
            .iter_mut()
            .find(|s| s.id == before.id)
            .ok_or(ChangeError("unknown sample"))?;
        ensure(*slot == *before, "sample differs from before")?;
        *slot = after.clone();
        Ok(())
    }

    pub(super) fn reorder_samples(
        &mut self,
        before: &[SampleId],
        after: &[SampleId],
    ) -> Result<(), ChangeError> {
        let current: Vec<SampleId> = self.samples.iter().map(|s| s.id).collect();
        ensure(current == before, "sample order differs from before")?;
        ensure(is_permutation(before, after), "new order is no permutation")?;
        let mut old = std::mem::take(&mut self.samples);
        for id in after {
            // Present: `after` is a permutation of the current IDs.
            if let Some(i) = old.iter().position(|s| s.id == *id) {
                self.samples.push(old.swap_remove(i));
            }
        }
        self.sort_results();
        Ok(())
    }

    /// Serials are trimmed, not empty and unique (M3 decision 6).
    fn check_serial(&self, sample: &Sample) -> Result<(), ChangeError> {
        ensure(
            !sample.serial.is_empty() && sample.serial.trim() == sample.serial,
            "serial is empty or not trimmed",
        )?;
        ensure(
            !self
                .samples
                .iter()
                .any(|s| s.id != sample.id && s.serial == sample.serial),
            "serial exists",
        )
    }

    /// Position of every sample and characteristic, the sort key of results.
    fn result_order(&self) -> (HashMap<SampleId, usize>, HashMap<CharId, usize>) {
        let samples = self.samples.iter().enumerate().map(|(i, s)| (s.id, i));
        let chars = self
            .characteristics
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i));
        (samples.collect(), chars.collect())
    }

    /// Restores the order of results after the sample or placement order changed.
    pub(super) fn sort_results(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let (samples, chars) = self.result_order();
        self.results.sort_by_key(|r| {
            (
                samples.get(&r.sample).copied().unwrap_or(usize::MAX),
                chars.get(&r.characteristic).copied().unwrap_or(usize::MAX),
            )
        });
    }

    pub(super) fn change_result(
        &mut self,
        cell: ResultCell,
        before: Option<&MeasurementResult>,
        after: Option<&MeasurementResult>,
    ) -> Result<(), ChangeError> {
        ensure(
            before.is_some() || after.is_some(),
            "result change without state",
        )?;
        ensure(
            before.into_iter().chain(after).all(|r| r.cell() == cell),
            "result belongs to another cell",
        )?;
        let (samples, chars) = self.result_order();
        let key = |c: ResultCell| {
            (
                samples.get(&c.sample).copied().unwrap_or(usize::MAX),
                chars.get(&c.characteristic).copied().unwrap_or(usize::MAX),
            )
        };
        let target = key(cell);
        ensure(
            target.0 != usize::MAX && target.1 != usize::MAX,
            "result of an unknown sample or characteristic",
        )?;
        let at = self.results.partition_point(|r| key(r.cell()) < target);
        let current = self.results.get(at).filter(|r| r.cell() == cell);
        ensure(current == before, "result differs from before")?;
        match (current.is_some(), after) {
            (true, Some(after)) => self.results[at] = after.clone(),
            (true, None) => {
                self.results.remove(at);
            }
            (false, Some(after)) => self.results.insert(at, after.clone()),
            (false, None) => {}
        }
        Ok(())
    }
}
