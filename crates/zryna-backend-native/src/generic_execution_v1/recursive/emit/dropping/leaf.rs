//! Typed leaf effects and distinct positive-owner suspensions.
use super::super::super::calls::Refs;
use super::super::util::{self as u, c32, c64, call, cmp, load, trap};
use super::{State, again, frames};
use crate::generic_execution_v1::{codegen, invariant, runtime};
use cranelift_codegen::ir::{FuncRef, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
pub(super) fn emit(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    rt: &[FuncRef],
    s: State,
    leaf: Value,
    offset: Value,
) -> Result<(), Diagnostic> {
    for id in 0..p.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        let view = p.stored_type(id).ok_or_else(invariant)?;
        if !matches!(
            view.category(),
            TypeCategory::String | TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak
        ) {
            continue;
        }
        let yes = b.create_block();
        let no = b.create_block();
        let key = c32(b, id);
        let matched = cmp(b, IntCC::Equal, leaf, key);
        b.ins().brif(matched, yes, &[], no, &[]);
        b.switch_to_block(yes);
        let address = u::offset(b, s.root, offset);
        match view.category() {
            TypeCategory::String => {
                let _effect = p.string_leaf(id, false)?;
                runtime::string_release(p, rt, b, s.ctx, address)?;
                again(b, s, s.ty, s.root, offset);
            }
            TypeCategory::Weak => {
                let control = load(b, types::I64, address, 0);
                runtime::control_release(
                    p,
                    rt,
                    b,
                    s.ctx,
                    p.control_drop(id, false)?,
                    &[control, s.flag],
                )?;
                again(b, s, s.ty, s.root, offset);
            }
            TypeCategory::Shared => shared(p, b, rt, s, id, offset, address)?,
            TypeCategory::Vec => vector(p, b, r, s, id, offset, address)?,
            _ => return Err(invariant()),
        }
        b.switch_to_block(no);
    }
    b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
fn vector(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    s: State,
    id: u32,
    offset: Value,
    address: Value,
) -> Result<(), Diagnostic> {
    let meta = p.containers().and_then(|c| c.element(id)).ok_or_else(invariant)?;
    let length = load(b, types::I64, address, 8);
    let capacity = load(b, types::I64, address, 16);
    let max = c64(b, zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS);
    let excess = cmp(b, IntCC::UnsignedGreaterThan, length, max);
    trap(b, excess);
    let malformed = cmp(b, IntCC::UnsignedGreaterThan, length, capacity);
    trap(b, malformed);
    let zero = c64(b, 0);
    let empty = cmp(b, IntCC::Equal, length, zero);
    let release = b.create_block();
    let descend = b.create_block();
    b.ins().brif(empty, release, &[], descend, &[]);
    b.switch_to_block(release);
    let ty = c32(b, id);
    let status = call(b, r.release, &[ty, address, s.ctx]);
    trap(b, status);
    again(b, s, s.ty, s.root, offset);
    b.switch_to_block(descend);
    let length = b.ins().ireduce(types::I32, length);
    let one = c32(b, 1);
    let index = b.ins().isub(length, one);
    frames::push(
        b,
        s.arena,
        frames::Pending { leaf: id, root: s.root, ty: s.ty, offset, index, role: 0 },
    )?;
    let data = load(b, types::I64, address, 0);
    let index = b.ins().uextend(types::I64, index);
    let stride = c64(b, meta.stride());
    let bytes = b.ins().imul(index, stride);
    let root = b.ins().iadd(data, bytes);
    let child = c32(b, meta.element().index());
    let size = c32(b, u::size(p, meta.element().index())?);
    again(b, s, child, root, size);
    Ok(())
}
fn shared(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    rt: &[FuncRef],
    s: State,
    id: u32,
    offset: Value,
    address: Value,
) -> Result<(), Diagnostic> {
    let meta = p.containers().and_then(|c| c.control(id)).ok_or_else(invariant)?;
    let control = load(b, types::I64, address, 0);
    runtime::control_release(p, rt, b, s.ctx, p.control_drop(id, false)?, &[control, s.flag])?;
    let last = load(b, types::I32, s.flag, 0);
    let descend = b.create_block();
    let done = b.create_block();
    b.ins().brif(last, descend, &[], done, &[]);
    b.switch_to_block(descend);
    let index = c32(b, 0);
    frames::push(
        b,
        s.arena,
        frames::Pending { leaf: id, root: s.root, ty: s.ty, offset, index, role: 1 },
    )?;
    let root = u::add(b, control, meta.payload_offset());
    let child = c32(b, meta.payload().index());
    let size = c32(b, u::size(p, meta.payload().index())?);
    again(b, s, child, root, size);
    b.switch_to_block(done);
    again(b, s, s.ty, s.root, offset);
    Ok(())
}
