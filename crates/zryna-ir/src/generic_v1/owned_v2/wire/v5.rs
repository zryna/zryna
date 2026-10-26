//! Private complete Struct clone transport; physical decoding grants no Clone authority.
use super::{Domain, Failure, raw};

#[cfg(test)]
mod tests;

/// Distinct private domain for complete Struct cloning and inherited owned operations.
pub const HEADER: &[u8] = b"ZRYNA-GENERIC-OWNED-IR-V5\0";
/// Private complete Struct clone wire version.
pub const VERSION: u32 = 5;

/// Opaque bounded v5 claims, distinct from every older decoded carrier.
/// Only the owning issuer can consume the retained raw claims directly.
///
/// The positive control binds the actual public decode API.
/// ```
/// use zryna_ir::generic_v1::{Failure, owned_v2::wire};
/// let _: fn(&[u8]) -> Result<wire::v5::DecodedOwnedV5, Failure> = wire::v5::decode;
/// ```
/// Raw claims cannot construct a public carrier.
/// ```compile_fail
/// use zryna_ir::generic_v1::owned_v2::{raw, wire};
/// fn forge(claim: raw::Program) { let _ = wire::v5::DecodedOwnedV5(claim); }
/// ```
/// Claims access is immutable.
/// ```compile_fail
/// use zryna_ir::generic_v1::owned_v2::wire;
/// fn edit(carrier: wire::v5::DecodedOwnedV5) { carrier.claims().extensions.clear(); }
/// ```
/// A fresh v5 carrier cannot become the v2/v3 shared carrier or the distinct v4 carrier.
/// ```compile_fail
/// use zryna_ir::generic_v1::owned_v2::wire;
/// fn downgrade(carrier: wire::v5::DecodedOwnedV5) { let _: wire::DecodedProgram = carrier.into(); }
/// ```
/// ```compile_fail
/// use zryna_ir::generic_v1::owned_v2::wire;
/// fn downgrade(carrier: wire::v5::DecodedOwnedV5) { let _: wire::v4::DecodedOwnedV4 = carrier.into(); }
/// ```
/// Copying its raw claims cannot convert them into earlier admission.
/// ```compile_fail
/// use zryna_ir::generic_v1::owned_v2::wire;
/// fn launder(carrier: wire::v5::DecodedOwnedV5) {
///     let _: wire::DecodedProgram = carrier.claims().clone().into();
/// }
/// ```
/// An old carrier cannot become a v5 carrier.
/// ```compile_fail
/// use zryna_ir::generic_v1::owned_v2::wire;
/// fn promote(old: wire::v4::DecodedOwnedV4) { let _: wire::v5::DecodedOwnedV5 = old.into(); }
/// ```
#[derive(Debug)]
pub struct DecodedOwnedV5(pub(in crate::generic_v1::owned_v2) raw::Program);

impl DecodedOwnedV5 {
    /// Immutable untrusted claims; decoding proves no source, type, plan or Clone capability.
    #[must_use]
    pub const fn claims(&self) -> &raw::Program {
        &self.0
    }
}

/// Encodes raw v5 claims without granting verification or execution authority.
/// # Errors
/// Rejects inherited core, count, field, child and complete-message byte ceilings.
pub fn encode(claim: &raw::Program) -> Result<Vec<u8>, Failure> {
    super::encode_for(claim, Domain::V5)
}

/// Decodes only the exact v5 envelope into its separate immutable untrusted carrier.
/// # Errors
/// Rejects foreign domains/versions, truncation, unknown tags, amplification and suffixes.
pub fn decode(bytes: &[u8]) -> Result<DecodedOwnedV5, Failure> {
    super::decode_for(bytes, Domain::V5).map(DecodedOwnedV5)
}
