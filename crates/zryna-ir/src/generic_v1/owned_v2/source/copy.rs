//! Original concrete Copy transport; closed IR still checks independent sealed layout ownership.
use super::{Closed, Failure, owners};

pub(super) fn key(ty: &Closed) -> Result<Option<&[u8]>, Failure> {
    let Closed::Stored(key) = ty else { return Ok(None) };
    // Original checking precedes layout and must include unused opaque templates. The
    // admitted source universe is scalar/String/Option/Result; its existing recursive
    // affinity calculation checks every possible payload and treats opaque slots as owned.
    // Thus this grants no capability from a later Copy specialization or inactive variant.
    owners::admitted(ty)?;
    Ok((!owners::affine(ty)?).then_some(key.as_slice()))
}
