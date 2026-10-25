//! Strict ordinary-runtime object structure; dependency names alone grant no provider access.

use crate::generic_runtime_v1::{RuntimeMaterialIdentity, RuntimeSource};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use zryna_diagnostics::Diagnostic;

// Shared plain byte readers have no legacy issuer or fixture dependency.
use crate::native::runtime_object_bytes::{Failure, inventory, relocations, tables};

/// Evidence from structural validation, never a constructor for a prepared executable.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct RuntimeObjectIdentity {
    material: RuntimeMaterialIdentity,
    runtime_object_sha256: [u8; 32],
}

impl RuntimeObjectIdentity {
    pub(crate) const fn material(&self) -> &RuntimeMaterialIdentity {
        &self.material
    }
    pub(crate) const fn runtime_object_sha256(&self) -> &[u8; 32] {
        &self.runtime_object_sha256
    }
}

pub(crate) fn audit_runtime_object(
    source: &RuntimeSource<'_, '_, '_>,
    bytes: &[u8],
) -> Result<RuntimeObjectIdentity, Diagnostic> {
    if bytes.len() > zryna_backend_native::MAX_NATIVE_OBJECT_BYTES {
        return Err(limit());
    }
    if <[u8; 32]>::from(Sha256::digest(source.bytes())) != *source.identity().rendered_sha256() {
        return Err(error());
    }
    let expected: BTreeSet<_> = source
        .program()
        .runtime()
        .declarations()
        .native_linux_x86_64
        .iter()
        .map(|entry| entry.symbol.as_str())
        .collect();
    if expected.len() != 17 {
        return Err(error());
    }
    let table = tables::read(bytes).map_err(structural_failure)?;
    let symbols = inventory::check(&table, &expected).map_err(structural_failure)?;
    relocations::check(&table, &symbols).map_err(structural_failure)?;
    storage(bytes, &table)?;
    Ok(RuntimeObjectIdentity {
        material: source.identity().clone(),
        runtime_object_sha256: Sha256::digest(bytes).into(),
    })
}

fn storage(bytes: &[u8], table: &tables::Tables<'_>) -> Result<(), Diagnostic> {
    let start = usize::try_from(tables::number(bytes, 40, 8).map_err(structural_failure)?)
        .map_err(|_| error())?;
    let mut registry = false;
    for (index, section) in table.sections.iter().enumerate() {
        // tables::read has already bounded and validated every section header/range.
        let header =
            start.checked_add(index.checked_mul(64).ok_or_else(error)?).ok_or_else(error)?;
        if section.name == ".bss" {
            registry = section.size == 4096 * 8
                && tables::number(bytes, header + 48, 8).map_err(structural_failure)? == 8;
        }
        if section.name == ".data" && section.size != 0 {
            return Err(error());
        }
    }
    if !registry {
        return Err(error());
    }
    // The exact ordinary template owns one local registry object and no observation/TLS state.
    let mut found = 0;
    for row in table.symbols.chunks_exact(24) {
        if tables::string(table.strings, tables::number(row, 0, 4).map_err(structural_failure)?)
            .map_err(structural_failure)?
            == "allocation_buckets"
        {
            let index = usize::try_from(tables::number(row, 6, 2).map_err(structural_failure)?)
                .map_err(|_| error())?;
            if row[4] != 1
                || table.sections.get(index).is_none_or(|s| s.name != ".bss")
                || tables::number(row, 8, 8).map_err(structural_failure)? != 0
                || tables::number(row, 16, 8).map_err(structural_failure)? != 4096 * 8
            {
                return Err(error());
            }
            found += 1;
        }
    }
    if found != 1 {
        return Err(error());
    }
    Ok(())
}

fn structural_failure(failure: Failure) -> Diagnostic {
    match failure {
        Failure::Inventory => error(),
        Failure::Limit => limit(),
    }
}

fn error() -> Diagnostic {
    super::super::native_error(
        "ZRYNA-C4162",
        "generic runtime object violates its exact ordinary ABI and storage inventory",
        "retain strong exact 17 definitions, four libc imports, bounded non-PIC relocations and the naturally aligned fixed registry without TLS or constructors",
    )
}

fn limit() -> Diagnostic {
    super::super::native_error(
        "ZRYNA-C4163",
        "generic runtime object exceeds its existing structural limits",
        "retain the existing object-byte, section, symbol and relocation limits",
    )
}
