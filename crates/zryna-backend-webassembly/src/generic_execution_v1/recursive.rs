//! Fixed reviewed traversal material plus independently source-bound data and relocations.
use super::model::containers as C;
use super::{
    Diagnostic, budget, invariant,
    model::{self, Model},
};
mod audit;
mod encode;
mod metadata;
mod template;
pub(super) const FUNCTIONS: u32 = 17;
pub(super) const FIND: u32 = 6;
pub(super) const INIT: u32 = 10;
pub(super) const DROP: u32 = 15;
pub(super) const CLONE: u32 = 16;
// Normative private-template schema: params, results, wide result, I32 local count.
pub(super) const SCHEMA: [(u32, u32, bool, u32); 17] = [
    (1, 1, false, 0),
    (1, 1, false, 0),
    (2, 1, true, 0),
    (4, 0, false, 0),
    (0, 1, false, 0),
    (0, 1, false, 0),
    (4, 1, true, 10),
    (2, 0, false, 1),
    (1, 0, false, 0),
    (1, 0, false, 0),
    (3, 0, false, 8),
    (6, 0, false, 1),
    (0, 1, false, 0),
    (0, 0, false, 0),
    (2, 0, false, 0),
    (3, 0, false, 12),
    (3, 1, false, 12),
];
impl Model<'_, '_, '_> {
    pub(in crate::generic_execution_v1) fn recursive_base(&self) -> u32 {
        self.accounting_base() + self.accounting_count()
    }
    pub(in crate::generic_execution_v1) fn recursive_count(&self) -> u32 {
        if self.memory.recursive_arena().is_some() { FUNCTIONS } else { 0 }
    }
    pub(in crate::generic_execution_v1) fn recursive_used_global(&self) -> u32 {
        if self.accounted() { 13 } else { 10 }
    }
    pub(in crate::generic_execution_v1) fn recursive_function(
        &self,
        index: usize,
    ) -> Option<usize> {
        if self.recursive_count() == 0 {
            return None;
        }
        let index = u32::try_from(index).ok()?;
        index.checked_sub(self.recursive_base()).filter(|n| *n < FUNCTIONS).map(|n| n as usize)
    }
}
fn call(m: &Model<'_, '_, '_>, index: u32) -> Result<u32, Diagnostic> {
    Ok(match index {
        0 => model::COPY,
        1 => m.string_call(model::STRING_CLONE),
        2 => m.string_call(model::STRING_RELEASE),
        3 => m.container(C::VEC_ALLOCATE),
        4 => m.container(C::VEC_RELEASE),
        5 => m.container(C::STRONG_CLONE),
        6 => m.container(C::WEAK_CLONE),
        7 => m.container(C::STRONG_BEGIN),
        8 => m.container(C::STRONG_FINISH),
        9 => m.container(C::WEAK_RELEASE),
        n if n < 10 + FUNCTIONS => m.recursive_base() + n - 10,
        _ => return Err(invariant()),
    })
}
fn constant(m: &Model<'_, '_, '_>, index: u32) -> Result<i32, Diagnostic> {
    let arena = m.memory.recursive_arena().ok_or_else(invariant)?;
    let value = match index {
        0 => arena.metadata_start,
        1 => arena.start,
        2 => arena.capacity,
        4 => m.memory.claims().scratch_start,
        _ => return Err(invariant()),
    };
    Ok(value.cast_signed())
}
pub(super) use audit::{check_body, check_wrapper};
pub(super) use encode::{append, clone_wrapper, drop_wrapper};
pub(super) use metadata::{check as check_metadata, encode as metadata_bytes};
