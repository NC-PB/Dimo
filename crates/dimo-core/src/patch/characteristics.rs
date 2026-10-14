//! Appliers for characteristics, balloons and the placement order.

use super::{ChangeError, ensure, is_permutation};
use crate::balloon::Balloon;
use crate::characteristic::Characteristic;
use crate::id::CharId;
use crate::project::Project;

impl Project {
    pub(super) fn insert_characteristic(
        &mut self,
        index: usize,
        characteristic: &Characteristic,
    ) -> Result<(), ChangeError> {
        ensure(
            index <= self.characteristics.len(),
            "insert index out of range",
        )?;
        ensure(
            self.characteristic(characteristic.id).is_none(),
            "characteristic ID exists",
        )?;
        self.characteristics.insert(index, characteristic.clone());
        Ok(())
    }

    pub(super) fn remove_characteristic(
        &mut self,
        index: usize,
        characteristic: &Characteristic,
    ) -> Result<(), ChangeError> {
        ensure(
            self.characteristics.get(index) == Some(characteristic),
            "removed characteristic differs",
        )?;
        ensure(
            self.balloons_of(characteristic.id).next().is_none(),
            "characteristic still has balloons",
        )?;
        ensure(
            self.results_of(characteristic.id).next().is_none(),
            "characteristic still has results",
        )?;
        self.characteristics.remove(index);
        Ok(())
    }

    pub(super) fn change_characteristic(
        &mut self,
        before: &Characteristic,
        after: &Characteristic,
    ) -> Result<(), ChangeError> {
        ensure(before.id == after.id, "characteristic ID changed")?;
        let slot = self
            .characteristics
            .iter_mut()
            .find(|c| c.id == before.id)
            .ok_or(ChangeError("unknown characteristic"))?;
        ensure(*slot == *before, "characteristic differs from before")?;
        *slot = after.clone();
        Ok(())
    }

    pub(super) fn insert_balloon(
        &mut self,
        index: usize,
        balloon: &Balloon,
    ) -> Result<(), ChangeError> {
        ensure(index <= self.balloons.len(), "insert index out of range")?;
        ensure(self.balloon(balloon.id).is_none(), "balloon ID exists")?;
        self.check_balloon_refs(balloon)?;
        self.balloons.insert(index, balloon.clone());
        Ok(())
    }

    pub(super) fn remove_balloon(
        &mut self,
        index: usize,
        balloon: &Balloon,
    ) -> Result<(), ChangeError> {
        ensure(
            self.balloons.get(index) == Some(balloon),
            "removed balloon differs",
        )?;
        self.balloons.remove(index);
        Ok(())
    }

    pub(super) fn change_balloon(
        &mut self,
        before: &Balloon,
        after: &Balloon,
    ) -> Result<(), ChangeError> {
        ensure(before.id == after.id, "balloon ID changed")?;
        self.check_balloon_refs(after)?;
        let slot = self
            .balloons
            .iter_mut()
            .find(|b| b.id == before.id)
            .ok_or(ChangeError("unknown balloon"))?;
        ensure(*slot == *before, "balloon differs from before")?;
        *slot = after.clone();
        Ok(())
    }

    pub(super) fn reorder(
        &mut self,
        before: &[CharId],
        after: &[CharId],
    ) -> Result<(), ChangeError> {
        let current: Vec<CharId> = self.characteristics.iter().map(|c| c.id).collect();
        ensure(current == before, "order differs from before")?;
        ensure(is_permutation(before, after), "new order is no permutation")?;
        let mut old = std::mem::take(&mut self.characteristics);
        for id in after {
            // Present: `after` is a permutation of the current IDs.
            if let Some(i) = old.iter().position(|c| c.id == *id) {
                self.characteristics.push(old.swap_remove(i));
            }
        }
        self.sort_results();
        Ok(())
    }

    fn check_balloon_refs(&self, balloon: &Balloon) -> Result<(), ChangeError> {
        ensure(
            self.characteristic(balloon.characteristic).is_some(),
            "balloon of an unknown characteristic",
        )?;
        ensure(
            self.sheet(balloon.sheet).is_some(),
            "balloon on an unknown sheet",
        )
    }
}
