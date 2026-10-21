//! Physical provider work and import-edge budgets for ownership discovery.

use super::support::{budget, diagnostic, rejected};
use super::{
    EdgeIdentity, Import, MAX_MODULE_IMPORT_EDGES, MAX_MODULE_PROVIDER_CALLS,
    MAX_MODULE_PROVIDER_SOURCE_BYTES, ModuleClosureError, add,
};
use std::collections::HashSet;
use zryna_source::NormalizedSourcePath;

pub(super) fn account_provider(
    calls: &mut usize,
    bytes: &mut usize,
    input_bytes: usize,
    aggregate_bytes: usize,
    work_units: usize,
    final_call: bool,
) -> Result<(), ModuleClosureError> {
    if work_units == 0 {
        return Err(budget("ownership provider analysis work must be positive"));
    }
    let next_calls = add(*calls, work_units)?;
    let charge = input_bytes.checked_mul(work_units).ok_or_else(|| {
        budget("ownership provider source-byte accounting exceeded the supported integer range")
    })?;
    let next_bytes = add(*bytes, charge)?;
    let reserved_calls = if final_call { next_calls } else { add(next_calls, work_units)? };
    let reserved_bytes = if final_call {
        next_bytes
    } else {
        let reserve = aggregate_bytes.checked_mul(work_units).ok_or_else(|| {
            budget(
                "ownership final provider source-byte reserve exceeded the supported integer range",
            )
        })?;
        add(next_bytes, reserve)?
    };
    if reserved_bytes > MAX_MODULE_PROVIDER_SOURCE_BYTES
        || reserved_calls > MAX_MODULE_PROVIDER_CALLS
    {
        return Err(budget("ownership module discovery exceeded its provider budget"));
    }
    *calls = next_calls;
    *bytes = next_bytes;
    Ok(())
}

pub(super) fn register_import_edges(
    edges: &mut HashSet<EdgeIdentity>,
    path: &NormalizedSourcePath,
    import: &Import,
) -> Result<(), ModuleClosureError> {
    for binding in &import.bindings {
        let identity = EdgeIdentity {
            importer: path.clone(),
            specifier: import.specifier.clone(),
            imported: binding.imported.clone(),
            local: binding.local.clone(),
        };
        if edges.len() >= MAX_MODULE_IMPORT_EDGES || !edges.insert(identity) {
            return Err(rejected(diagnostic(
                "ZRYNA-D3301",
                Some(path),
                "duplicate or excessive ownership import edge",
            )));
        }
    }
    Ok(())
}
