//! Independent full-profile wire admission; a decoded carrier cannot execute.
use super::raw;
use crate::generic_v1::{Failure, reserve};
mod decode_call_operations;
mod decode_construct_operations;
mod decode_control;
mod decode_loan_operations;
mod decode_model;
mod decode_operations;
mod decode_place_operations;
mod decode_scalar_operations;
mod decoding;
mod encode_call_operations;
mod encode_construct_operations;
mod encode_control;
mod encode_loan_operations;
mod encode_model;
mod encode_operations;
mod encode_place_operations;
mod encode_scalar_operations;
mod encoding;
mod limits;
mod preflight;
mod reader;
mod writer;
/// Exact independent execution envelope, disjoint from owned v1-v5.
pub const HEADER: &[u8] = b"ZRYNA-GENERIC-EXECUTION-V1\0";
/// First full-profile schema version.
pub const VERSION: u32 = 1;
/// Existing generic complete-message ceiling.
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
/// Existing generic cumulative vector-child ceiling.
pub const MAX_CHILDREN: usize = 1_048_576;
/// Opaque bounded wire carrier; claims remain untrusted and have no issuer seal.
/// ```
/// use zryna_ir::generic_v1::{Failure, execution_v1::wire};
/// let _: fn(&[u8]) -> Result<wire::DecodedExecutionV1, Failure> = wire::decode;
/// ```
/// Raw claims cannot construct this carrier.
/// ```compile_fail
/// use zryna_ir::generic_v1::execution_v1::{raw, wire};
/// fn forge(raw: raw::Program) { let _ = wire::DecodedExecutionV1(raw); }
/// ```
/// Claimed rows remain immutable through the carrier.
/// ```compile_fail
/// use zryna_ir::generic_v1::execution_v1::wire;
/// fn mutate(carrier: wire::DecodedExecutionV1) { carrier.claims().functions.clear(); }
/// ```
/// No old admission carrier converts to full-profile admission.
/// ```compile_fail
/// use zryna_ir::generic_v1::{execution_v1, owned_v2};
/// fn promote(old: owned_v2::wire::v5::DecodedOwnedV5) {
///     let _: execution_v1::wire::DecodedExecutionV1 = old.into();
/// }
/// ```
/// Full-profile raw claims cannot become old admission.
/// ```compile_fail
/// use zryna_ir::generic_v1::{execution_v1, wire};
/// fn launder(carrier: execution_v1::wire::DecodedExecutionV1) {
///     let _: wire::DecodedProgram = carrier.claims().clone().into();
/// }
/// ```
#[derive(Debug)]
pub struct DecodedExecutionV1(pub(in crate::generic_v1::execution_v1) raw::Program);
impl DecodedExecutionV1 {
    /// Immutable claims for independent source/type/state verification.
    #[must_use]
    pub const fn claims(&self) -> &raw::Program {
        &self.0
    }
}
/// Encodes complete raw claims in the separately versioned execution domain.
/// # Errors
/// Rejects malformed structural inventories, keys and inherited resource ceilings.
pub fn encode(program: &raw::Program) -> Result<Vec<u8>, Failure> {
    encode_checked(program, preflight::check)
}
// A callback keeps the actual quota-before-preflight ordering directly testable.
fn encode_checked(
    program: &raw::Program,
    check: impl FnOnce(&raw::Program) -> Result<(), Failure>,
) -> Result<Vec<u8>, Failure> {
    encoding::census(program)?;
    check(program)?;
    encoding::encode(program)
}
/// Decodes exact complete claims with bounded, fallible allocation.
/// # Errors
/// Rejects unknown tags, malformed fields, noncanonical encodings, excess budgets or suffixes.
pub fn decode(bytes: &[u8]) -> Result<DecodedExecutionV1, Failure> {
    limits::bound(bytes.len(), MAX_BYTES)?;
    let program = decoding::decode(bytes)?;
    preflight::check(&program)?;
    Ok(DecodedExecutionV1(program))
}
fn invalid() -> Failure {
    crate::generic_v1::reject("malformed separately versioned execution wire claim")
}
#[cfg(test)]
mod hostile_tests;
#[cfg(test)]
mod tag_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod limit_tests;

#[cfg(test)]
mod census_tests;

#[cfg(test)]
mod compatibility_tests;

#[cfg(test)]
mod resource_admission_tests;
