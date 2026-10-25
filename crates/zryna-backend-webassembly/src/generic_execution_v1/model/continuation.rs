//! Binary lookups in independently checked original-site temporary inventories.
use super::{Diagnostic, Model, invariant};
use crate::generic_execution_v1::memory::raw::{Slot, UpgradeEdge};
use zryna_ir::generic_v1::execution_v1::OperationSite;
impl Model<'_, '_, '_> {
    pub(in crate::generic_execution_v1) fn pending(
        &self,
        site: OperationSite,
    ) -> Result<&Slot, Diagnostic> {
        let rows = &self.memory.claims().functions[site.function].pending_vecs;
        self.take(
            usize::try_from(rows.len().checked_ilog2().unwrap_or(0))
                .map_err(|_| super::budget())?
                + 1,
        )?;
        let at = rows
            .binary_search_by_key(&(site.block, site.instruction), |row| {
                (row.site.block, row.site.instruction)
            })
            .map_err(|_| invariant())?;
        Ok(&rows[at].slot)
    }
    pub(in crate::generic_execution_v1) fn snapshot(
        &self,
        function: usize,
        block: u32,
        success: bool,
    ) -> Result<&UpgradeEdge, Diagnostic> {
        let rows = &self.memory.claims().functions[function].upgrade_edges;
        self.take(
            usize::try_from(rows.len().checked_ilog2().unwrap_or(0))
                .map_err(|_| super::budget())?
                + 1,
        )?;
        let at = rows
            .binary_search_by_key(&(block, success), |row| (row.block, row.success))
            .map_err(|_| invariant())?;
        Ok(&rows[at])
    }
}
