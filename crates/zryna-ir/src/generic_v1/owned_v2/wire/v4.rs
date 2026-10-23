//! Private nominal Struct transport; decoding grants bounded raw claims only.
use super::{Domain, Failure, raw};

/// Distinct private domain for Struct construction and Copy field observation.
pub const HEADER: &[u8] = b"ZRYNA-GENERIC-OWNED-IR-V4\0";
/// Private nominal Struct wire version.
pub const VERSION: u32 = 4;

/// Opaque bounded v4 claims, distinct from every older decoded carrier.
/// Only the owning issuer can consume the retained raw claims directly.
#[derive(Debug)]
pub struct DecodedOwnedV4(pub(in crate::generic_v1::owned_v2) raw::Program);

impl DecodedOwnedV4 {
    /// Immutable untrusted claims; neither source nor executable authority is exposed.
    #[must_use]
    pub const fn claims(&self) -> &raw::Program {
        &self.0
    }
}

/// Encodes raw v4 claims without granting any verification or execution authority.
/// # Errors
/// Rejects inherited core, count, field, child and complete-message byte ceilings.
pub fn encode(claim: &raw::Program) -> Result<Vec<u8>, Failure> {
    super::encode_for(claim, Domain::V4)
}

/// Decodes the exact v4 envelope into its separate opaque untrusted carrier.
/// # Errors
/// Rejects foreign domains/versions, truncation, unknown tags, amplification and suffixes.
pub fn decode(bytes: &[u8]) -> Result<DecodedOwnedV4, Failure> {
    super::decode_for(bytes, Domain::V4).map(DecodedOwnedV4)
}
