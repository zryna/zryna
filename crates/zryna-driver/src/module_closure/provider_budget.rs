//! Physical provider work accounting and final authentication reserves.

use super::{
    MAX_MODULE_PROVIDER_CALLS, MAX_MODULE_PROVIDER_SOURCE_BYTES, ModuleClosureError,
    budget_rejection, checked_add,
};

#[derive(Clone, Copy)]
pub(super) enum ProviderCallPhase {
    Discovery,
    Final,
}

#[cfg(test)]
pub(super) fn account_provider_call(
    calls: &mut usize,
    phase: ProviderCallPhase,
) -> Result<(), ModuleClosureError> {
    account_provider_call_units(calls, phase, 1)
}

pub(super) fn account_provider_call_units(
    calls: &mut usize,
    phase: ProviderCallPhase,
    work_units: usize,
) -> Result<(), ModuleClosureError> {
    if work_units == 0 {
        return Err(budget_rejection("module provider analysis work must be positive"));
    }
    *calls = checked_add(*calls, work_units)?;
    let exhausted = match phase {
        ProviderCallPhase::Discovery => {
            checked_add(*calls, work_units)? > MAX_MODULE_PROVIDER_CALLS
        }
        ProviderCallPhase::Final => *calls > MAX_MODULE_PROVIDER_CALLS,
    };
    if exhausted {
        let message = match phase {
            ProviderCallPhase::Discovery if work_units == 1 => {
                "module discovery left no provider call for final authentication"
            }
            ProviderCallPhase::Discovery => {
                "module discovery left too few provider calls for final authentication"
            }
            ProviderCallPhase::Final => "module discovery exceeded the fixed provider-call budget",
        };
        return Err(budget_rejection(message));
    }
    Ok(())
}

pub(super) fn account_provider_work(
    calls: &mut usize,
    bytes: &mut usize,
    input_bytes: usize,
    aggregate_bytes: usize,
    work_units: usize,
    phase: ProviderCallPhase,
) -> Result<(), ModuleClosureError> {
    let mut next_calls = *calls;
    account_provider_call_units(&mut next_calls, phase, work_units)?;
    let charge = input_bytes.checked_mul(work_units).ok_or_else(|| {
        budget_rejection(
            "module provider source-byte accounting exceeded the supported integer range",
        )
    })?;
    let mut next_bytes = *bytes;
    account_provider_bytes(&mut next_bytes, charge)?;
    if matches!(phase, ProviderCallPhase::Discovery) {
        let reserve = aggregate_bytes.checked_mul(work_units).ok_or_else(|| {
            budget_rejection(
                "module final provider source-byte reserve exceeded the supported integer range",
            )
        })?;
        let mut reserved_bytes = next_bytes;
        account_provider_bytes(&mut reserved_bytes, reserve)?;
    }
    *calls = next_calls;
    *bytes = next_bytes;
    Ok(())
}

pub(super) fn account_provider_bytes(
    total: &mut usize,
    input: usize,
) -> Result<(), ModuleClosureError> {
    *total = checked_add(*total, input)?;
    if *total > MAX_MODULE_PROVIDER_SOURCE_BYTES {
        return Err(budget_rejection(
            "module discovery exceeded the cumulative provider source-byte budget",
        ));
    }
    Ok(())
}
