//! Original concrete Copy transport; closed IR still checks independent sealed layout ownership.
use super::{Budget, Closed, Failure, facts::Facts, owners};
use crate::generic_v1::source::Originals;

pub(super) fn key<'k>(
    ty: &'k Closed,
    originals: &Originals<'_>,
    facts: &mut Facts<'_, '_>,
    credit: &mut Budget,
) -> Result<Option<&'k [u8]>, Failure> {
    let Closed::Stored(key) = ty else { return Ok(None) };
    if facts.admission() == super::Admission::StructV4 || matches!(key.first(), Some(0x10..=0x13)) {
        // The original frame checks every substituted member, including inactive variants.
        // This is source Copy qualification; typed IR independently requires sealed drop_kind 0.
        return Ok((!facts.affine(originals, ty, credit)?).then_some(key.as_slice()));
    }
    // Original checking precedes layout and must include unused opaque templates. The
    // admitted source universe is scalar/String/Option/Result; its existing recursive
    // affinity calculation checks every possible payload and treats opaque slots as owned.
    // Thus this grants no capability from a later Copy specialization or inactive variant.
    owners::admitted(ty)?;
    Ok((!owners::affine(ty)?).then_some(key.as_slice()))
}
