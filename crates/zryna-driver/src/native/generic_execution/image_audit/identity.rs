//! Compact receipt retains full checked inventory privately without publishing libc symbol rows.

use super::{ClosureInventory, Diagnostic, rejected};

#[derive(Debug, serde::Serialize)]
pub(crate) struct ImageIdentity {
    executable_sha256: [u8; 32],
    required_tls_bytes: u64,
    closure: ClosureReceipt,
    #[serde(skip_serializing)]
    #[cfg(test)]
    inventory: ClosureInventory,
}

#[derive(Debug, serde::Serialize)]
struct ClosureReceipt {
    executable_sha256: [u8; 32],
    interpreter: Option<String>,
    needed: Vec<String>,
    providers: [ProviderReceipt; 2],
    execution_authorized: bool,
    complete_input_use_attestation: bool,
    relocation_or_resolver_execution_attested: bool,
}

#[derive(Debug, serde::Serialize)]
struct ProviderReceipt {
    id: String,
    sha256: [u8; 32],
    soname: Option<String>,
    needed: Vec<String>,
}

impl ImageIdentity {
    pub(super) fn issue(
        executable_sha256: [u8; 32],
        required_tls_bytes: u64,
        inventory: ClosureInventory,
    ) -> Result<Self, Diagnostic> {
        let [first, second] = inventory.providers.as_slice() else {
            return Err(rejected());
        };
        let provider =
            |row: &(String, super::super::dependencies::ObjectInventory)| ProviderReceipt {
                id: row.0.clone(),
                sha256: row.1.sha256,
                soname: row.1.soname.clone(),
                needed: row.1.needed.clone(),
            };
        let closure = ClosureReceipt {
            executable_sha256: inventory.executable.sha256,
            interpreter: inventory.executable.interpreter.clone(),
            needed: inventory.executable.needed.clone(),
            providers: [provider(first), provider(second)],
            // These are byte-checker observation limits, never an execution grant.
            execution_authorized: false,
            complete_input_use_attestation: false,
            relocation_or_resolver_execution_attested: false,
        };
        // Production retains the compact receipt; release the consumed full audit workspace.
        #[cfg(not(test))]
        drop(inventory);
        Ok(Self {
            executable_sha256,
            required_tls_bytes,
            closure,
            #[cfg(test)]
            inventory,
        })
    }

    pub(crate) const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }
    pub(crate) const fn required_tls_bytes(&self) -> u64 {
        self.required_tls_bytes
    }
    #[cfg(test)]
    pub(crate) const fn closure(&self) -> &ClosureInventory {
        &self.inventory
    }
}
