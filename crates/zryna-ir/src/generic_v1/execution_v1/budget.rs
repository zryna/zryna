//! One inherited private IR owner-work ledger for the independent execution verifier.
//!
//! New execution source/world/owner/frontier work and retained execution indexes
//! share this ledger. It is not a transitive all-library/all-phase work counter:
//! independently verified scalar ABI, layouts and runtime ABI keep their existing
//! published owner caps. No new execution retention may be assigned to those owners.

use super::{Failure, resource_failure};

/// Existing owned-verifier work ceiling, shared across all new execution phases.
pub(super) const MAX_OWNER_UNITS: usize = 1_048_576;

/// Prepay one complete bounded operation before reserving or inspecting its work.
/// Failure retains the caller's exact completed debit, permitting a checked retry.
pub(super) fn charge(spent: &mut usize, units: usize) -> Result<(), Failure> {
    let next = spent
        .checked_add(units)
        .filter(|next| *next <= MAX_OWNER_UNITS)
        .ok_or_else(|| resource_failure("full-profile ownership verification work ceiling"))?;
    *spent = next;
    Ok(())
}

#[cfg(test)]
mod tests;
