//! Whole value moves rename their pending obligation; subobject moves acquire a new one.
use super::{Context, Failure, State, raw};
pub(super) fn place(
    c: &Context<'_>,
    state: &mut State,
    id: u32,
    mode: raw::ReadMode,
) -> Result<(Option<u32>, Option<usize>), Failure> {
    state.place(c, id, mode == raw::ReadMode::Move)?;
    let tag = state.tag(c, id);
    let slot = if mode == raw::ReadMode::Move {
        let slot = state.move_slot(c, id)?;
        state.take_place(c, id)?;
        slot
    } else {
        None
    };
    Ok((tag, slot))
}
