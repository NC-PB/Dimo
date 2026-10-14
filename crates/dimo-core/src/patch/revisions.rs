//! Appliers for drawing revisions: sheets, revision labels and issued reports.

use super::{ChangeError, ensure};
use crate::id::RevisionId;
use crate::project::Project;
use crate::report::IssuedReport;
use crate::sheet::Sheet;

impl Project {
    pub(super) fn change_sheet(
        &mut self,
        before: &Sheet,
        after: &Sheet,
    ) -> Result<(), ChangeError> {
        ensure(before.id == after.id, "sheet ID changed")?;
        let slot = self
            .revisions
            .iter_mut()
            .flat_map(|r| &mut r.sheets)
            .find(|s| s.id == before.id)
            .ok_or(ChangeError("unknown sheet"))?;
        ensure(*slot == *before, "sheet differs from before")?;
        *slot = after.clone();
        Ok(())
    }

    pub(super) fn change_revision_label(
        &mut self,
        revision: RevisionId,
        before: &str,
        after: &str,
    ) -> Result<(), ChangeError> {
        let slot = self
            .revisions
            .iter_mut()
            .find(|r| r.id == revision)
            .ok_or(ChangeError("unknown revision"))?;
        ensure(slot.label == before, "revision label differs from before")?;
        after.clone_into(&mut slot.label);
        Ok(())
    }

    pub(super) fn insert_issued_report(
        &mut self,
        index: usize,
        report: &IssuedReport,
    ) -> Result<(), ChangeError> {
        ensure(
            index <= self.issued_reports.len(),
            "insert index out of range",
        )?;
        self.issued_reports.insert(index, report.clone());
        Ok(())
    }

    pub(super) fn remove_issued_report(
        &mut self,
        index: usize,
        report: &IssuedReport,
    ) -> Result<(), ChangeError> {
        ensure(
            self.issued_reports.get(index) == Some(report),
            "removed issued report differs",
        )?;
        self.issued_reports.remove(index);
        Ok(())
    }
}
