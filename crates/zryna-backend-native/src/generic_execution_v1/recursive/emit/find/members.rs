//! Exact enum tag and bidirectional Struct members; no heap indirection is followed.
use super::super::util::{c32, cmp, load};
use super::{Input, Refs, child, none};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
pub(super) fn emit(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    id: u32,
    i: Input,
) -> Result<(), Diagnostic> {
    let view = p.stored_type(id).ok_or_else(invariant)?;
    if view.category() == TypeCategory::Enum {
        let tag = load(b, types::I32, i.root, 0);
        for (ordinal, payload) in view.variants() {
            let yes = b.create_block();
            let next = b.create_block();
            let key = c32(b, ordinal);
            let matched = cmp(b, IntCC::Equal, tag, key);
            b.ins().brif(matched, yes, &[], next, &[]);
            b.switch_to_block(yes);
            if let Some(payload) = payload {
                let absent = b.create_block();
                child(
                    b,
                    r,
                    payload.index(),
                    u32::try_from(view.payload().ok_or_else(invariant)?.0).map_err(codegen)?,
                    i,
                    absent,
                );
                b.switch_to_block(absent);
            }
            none(b);
            b.switch_to_block(next);
        }
        b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
        return Ok(());
    }
    let mut fields = Vec::<(u32, u32)>::new();
    fields.try_reserve_exact(view.fields().len()).map_err(codegen)?;
    for (_, ty, offset) in view.fields() {
        fields.push((ty.index(), u32::try_from(offset).map_err(codegen)?));
    }
    let reverse = b.create_block();
    let forward = b.create_block();
    let zero = c32(b, 0);
    let backwards = cmp(b, IntCC::NotEqual, i.reverse, zero);
    b.ins().brif(backwards, reverse, &[], forward, &[]);
    b.switch_to_block(reverse);
    for &(ty, offset) in fields.iter().rev() {
        let next = b.create_block();
        child(b, r, ty, offset, i, next);
        b.switch_to_block(next);
    }
    none(b);
    b.switch_to_block(forward);
    for (ty, offset) in fields {
        let next = b.create_block();
        child(b, r, ty, offset, i, next);
        b.switch_to_block(next);
    }
    none(b);
    Ok(())
}
