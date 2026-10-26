//! Precharged physical provider work, final-pass reserve and finite source/edge census.
use super::{
    EdgeIdentity, HashSet, Import, ModuleClosureError, NormalizedSourcePath,
    support::{add, budget, diagnostic, rejected},
};
use crate::{
    MAX_MODULE_EDGE_MANIFEST_BYTES, MAX_MODULE_IMPORT_DECLARATIONS, MAX_MODULE_IMPORT_EDGES,
    MAX_MODULE_PROVIDER_CALLS, MAX_MODULE_PROVIDER_SOURCE_BYTES, MAX_MODULE_SOURCE_BYTES,
};
#[derive(Default)]
pub(super) struct Ledger {
    pub bytes: usize,
    calls: usize,
    provider_bytes: usize,
    imports: usize,
    edge_bytes: usize,
}
impl Ledger {
    pub(super) fn source(&mut self, bytes: usize) -> Result<(), ModuleClosureError> {
        let next = add(self.bytes, bytes)?;
        if next > MAX_MODULE_SOURCE_BYTES {
            return Err(budget("generic source closure byte limit exceeded"));
        }
        self.bytes = next;
        Ok(())
    }
    pub(super) fn provider(
        &mut self,
        input: usize,
        units: usize,
        final_pass: bool,
    ) -> Result<(), ModuleClosureError> {
        if units == 0 {
            return Err(budget("physical provider work must be positive"));
        }
        let calls = add(self.calls, units)?;
        let charge =
            input.checked_mul(units).ok_or_else(|| budget("provider byte charge overflow"))?;
        let bytes = add(self.provider_bytes, charge)?;
        let reserved_calls = if final_pass { calls } else { add(calls, units)? };
        let reserved_bytes = if final_pass {
            bytes
        } else {
            add(
                bytes,
                self.bytes
                    .checked_mul(units)
                    .ok_or_else(|| budget("final provider byte reserve overflow"))?,
            )?
        };
        if reserved_calls > MAX_MODULE_PROVIDER_CALLS
            || reserved_bytes > MAX_MODULE_PROVIDER_SOURCE_BYTES
        {
            return Err(budget("generic physical provider work limit exceeded"));
        }
        self.calls = calls;
        self.provider_bytes = bytes;
        Ok(())
    }
    pub(super) fn import(
        &mut self,
        path: &NormalizedSourcePath,
        import: &Import,
        target: &NormalizedSourcePath,
        identities: &mut HashSet<EdgeIdentity>,
    ) -> Result<(), ModuleClosureError> {
        self.imports = add(self.imports, 1)?;
        if self.imports > MAX_MODULE_IMPORT_DECLARATIONS {
            return Err(budget("generic import declaration limit exceeded"));
        }
        for binding in &import.bindings {
            let identity = EdgeIdentity {
                importer: path.clone(),
                specifier: import.specifier.clone(),
                imported: binding.imported.clone(),
                local: binding.local.clone(),
            };
            if identities.contains(&identity) {
                return Err(rejected(diagnostic(
                    "ZRYNA-D3006",
                    Some(path),
                    "duplicate named import edge",
                )));
            }
            if identities.len() >= MAX_MODULE_IMPORT_EDGES {
                return Err(budget("generic import edge limit exceeded"));
            }
            let raw = [
                path.as_str(),
                target.as_str(),
                &import.specifier,
                &binding.imported,
                &binding.local,
            ]
            .into_iter()
            .try_fold(0, |sum, text| add(sum, text.len()))?;
            let escaped =
                raw.checked_mul(6).ok_or_else(|| budget("edge manifest byte overflow"))?;
            let next = add(self.edge_bytes, add(escaped, 128)?)?;
            if next > MAX_MODULE_EDGE_MANIFEST_BYTES {
                return Err(budget("generic edge manifest byte limit exceeded"));
            }
            self.edge_bytes = next;
            identities.insert(identity);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;
