//! Independent whole-field transfer and nonconsuming live-base observation.
use super::{Failure, State, charge, graph, owned, reject};

pub(super) fn construct(
    state: &mut State,
    fields: &[u32],
    types: &[graph::Type],
    ownership: &[bool],
    used: &mut usize,
) -> Result<(), Failure> {
    charge(used, fields.len())?;
    for id in fields {
        let field_type = types
            .get(*id as usize)
            .copied()
            .ok_or_else(|| reject("unknown Struct field operand"))?;
        if !matches!(field_type, graph::Type::Stored(_)) {
            return Err(reject("Struct field cannot store a loan or unit"));
        }
        let affine = owned(field_type, ownership);
        state.read(*id, field_type, affine)?;
        state.consume(*id, affine)?;
    }
    Ok(())
}

pub(super) fn observe(
    state: &State,
    value: u32,
    types: &[graph::Type],
    ownership: &[bool],
) -> Result<(), Failure> {
    let base_type = types
        .get(value as usize)
        .copied()
        .ok_or_else(|| reject("unknown Struct observation operand"))?;
    if !matches!(base_type, graph::Type::Stored(_)) {
        return Err(reject("Copy field requires stored live Struct root"));
    }
    state.read(value, base_type, owned(base_type, ownership))
}
