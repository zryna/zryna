//! Eight outer control effects; raw allocation/release factoring stays unaccounted.
use super::super::{Body, Diagnostic, FunctionCode, I, Model, WORD};
use crate::generic_execution_v1::model::containers as C;
use wasm_encoder::{BlockType, MemArg};
mod birth;
mod common;
mod counts;
mod release;
pub(super) fn append(m: &Model<'_, '_, '_>, out: &mut Vec<FunctionCode>) -> Result<(), Diagnostic> {
    if !m.has_accounted_control() {
        return Ok(());
    }
    out.push(birth::emit(m)?);
    for role in [C::STRONG_CLONE, C::DOWNGRADE, C::WEAK_CLONE] {
        out.push(counts::emit(m, role)?);
    }
    for role in [C::STRONG_BEGIN, C::STRONG_FINISH, C::WEAK_RELEASE] {
        out.push(release::emit(m, role)?);
    }
    if m.continuation {
        out.push(counts::emit(m, C::WEAK_UPGRADE)?);
    }
    Ok(())
}
