//! Pure source-outcome transport; bytes never grant program or execution authority.
use super::raw::TrapIdentity;
use zryna_abi::{ScalarType, ScalarValue};

/// Exact private host record length, independent of target word size.
pub const RECORD_BYTES: usize = 20;
/// Opening little-endian word of the version-one invocation record.
pub const RECORD_MAGIC: u32 = 0x3149_475a;
/// Closing little-endian word; trailing bytes are rejected.
pub const RECORD_END: u32 = 0x3144_4e45;

/// Source-function status, separate from canonical ownership-runtime status numbers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceStatus {
    /// All owning cleanup and invocation completion checks succeeded.
    Success,
    /// An exact language failure after its owning cleanup.
    Trap(TrapIdentity),
    /// Internal failure; it must never become a controlled language trap.
    HostInvariant,
}
impl SourceStatus {
    /// Exact source-status code. Canonical runtime status is not an input.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::Success => 0,
            Self::Trap(TrapIdentity::Bounds) => 1,
            Self::Trap(TrapIdentity::Allocation) => 2,
            Self::Trap(TrapIdentity::Capacity) => 3,
            Self::Trap(TrapIdentity::Refcount) => 4,
            Self::Trap(TrapIdentity::Utf8) => 5,
            Self::HostInvariant => 255,
        }
    }
    /// Parses only the source-status namespace, never a signal or runtime status.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::Trap(TrapIdentity::Bounds)),
            2 => Some(Self::Trap(TrapIdentity::Allocation)),
            3 => Some(Self::Trap(TrapIdentity::Capacity)),
            4 => Some(Self::Trap(TrapIdentity::Refcount)),
            5 => Some(Self::Trap(TrapIdentity::Utf8)),
            255 => Some(Self::HostInvariant),
            _ => None,
        }
    }
}
/// A parsed host result; it authenticates no artifact, process or source owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Exact scalar value from a successfully completed source invocation.
    Value(ScalarValue),
    /// Exact controlled source failure, with no scalar result.
    Trap(TrapIdentity),
}
/// A malformed transport or host invariant, never a source-level trap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportError {
    /// Host execution did not complete normally, regardless of any record bytes.
    HostFailure,
    /// The record is truncated or contains any extra byte.
    Length,
    /// Either fixed framing word differs.
    Framing,
    /// The record type differs from the separately bound export result.
    Type,
    /// Status is unknown or denotes a host invariant.
    Status,
    /// Boolean is not zero/one, or a trap attempts to publish scalar bits.
    Scalar,
}
/// Exact stable wire tag for an already verified scalar result type.
#[must_use]
pub const fn scalar_tag(ty: ScalarType) -> u32 {
    match ty {
        ScalarType::Bool => 0,
        ScalarType::I32 => 1,
    }
}
/// Parses a complete host record against the caller's separately bound result type.
/// A non-normal host/engine completion must be rejected before calling this function.
/// # Errors
/// Rejects incomplete/extra records, wrong framing/type/status and invalid scalar bits.
pub fn decode(bytes: &[u8], expected: ScalarType) -> Result<Outcome, TransportError> {
    let exact: &[u8; RECORD_BYTES] = bytes.try_into().map_err(|_| TransportError::Length)?;
    let word = |offset: usize| {
        u32::from_le_bytes([exact[offset], exact[offset + 1], exact[offset + 2], exact[offset + 3]])
    };
    if word(0) != RECORD_MAGIC || word(16) != RECORD_END {
        return Err(TransportError::Framing);
    }
    if word(8) != scalar_tag(expected) {
        return Err(TransportError::Type);
    }
    decode_packed((u64::from(word(4)) << 32) | u64::from(word(12)), expected)
}
/// Parses all 64 result bits of a normally completed private Wasm invoker.
/// # Errors
/// Rejects unknown/invariant source status and invalid scalar or trap bits.
pub fn decode_packed(bits: u64, expected: ScalarType) -> Result<Outcome, TransportError> {
    let code = u32::try_from(bits >> 32).map_err(|_| TransportError::Status)?;
    let scalar = u32::try_from(bits & u64::from(u32::MAX)).map_err(|_| TransportError::Scalar)?;
    match SourceStatus::from_code(code) {
        Some(SourceStatus::Success) => match expected {
            ScalarType::Bool => match scalar {
                0 => Ok(Outcome::Value(ScalarValue::Bool(false))),
                1 => Ok(Outcome::Value(ScalarValue::Bool(true))),
                _ => Err(TransportError::Scalar),
            },
            ScalarType::I32 => Ok(Outcome::Value(ScalarValue::I32(scalar.cast_signed()))),
        },
        Some(SourceStatus::Trap(trap)) if scalar == 0 => Ok(Outcome::Trap(trap)),
        Some(SourceStatus::Trap(_)) => Err(TransportError::Scalar),
        Some(SourceStatus::HostInvariant) | None => Err(TransportError::Status),
    }
}

#[cfg(test)]
mod tests;
