//! Exact physical record copies and reverse active-payload release; no carrier allocator.
use super::{
    body::{Frame, checked_offset},
    invariant,
};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, TrapCode, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::TypeCategory;
pub(super) fn chunks(size: u64) -> impl Iterator<Item = (u64, cranelift_codegen::ir::Type)> {
    let mut offset = 0;
    std::iter::from_fn(move || {
        if offset == size {
            return None;
        }
        let remaining = size - offset;
        let (width, ty) = if remaining >= 8 {
            (8, types::I64)
        } else if remaining >= 4 {
            (4, types::I32)
        } else if remaining >= 2 {
            (2, types::I16)
        } else {
            (1, types::I8)
        };
        let result = (offset, ty);
        offset += width;
        Some(result)
    })
}
pub(super) fn copy(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
    size: u64,
) -> Result<(), Diagnostic> {
    for (offset, ty) in chunks(size) {
        let offset = checked_offset(offset)?;
        let value = b.ins().load(ty, MemFlagsData::new(), source, offset);
        b.ins().store(MemFlagsData::new(), value, destination, offset);
    }
    Ok(())
}
pub(super) fn zero(
    b: &mut FunctionBuilder<'_>,
    destination: Value,
    size: u64,
) -> Result<(), Diagnostic> {
    for (offset, ty) in chunks(size) {
        let value = b.ins().iconst(ty, 0);
        b.ins().store(MemFlagsData::new(), value, destination, checked_offset(offset)?);
    }
    Ok(())
}
pub(super) fn add(
    b: &mut FunctionBuilder<'_>,
    address: Value,
    offset: u64,
) -> Result<Value, Diagnostic> {
    Ok(b.ins().iadd_imm_u(address, i64::from(checked_offset(offset)?)))
}
pub(super) fn drop_value(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    ty: u32,
    address: Value,
) -> Result<(), Diagnostic> {
    if c.p.containers().is_some() {
        return c.helpers.drop_value(c.p, b, ty, address);
    }
    let view = c.p.stored_type(ty).ok_or_else(invariant)?;
    if view.drop_kind() == 0 {
        return Ok(());
    }
    match view.category() {
        TypeCategory::String => release(c, b, address),
        TypeCategory::Struct => {
            let fields = view.fields().collect::<Vec<_>>();
            for (_, child, offset) in fields.into_iter().rev() {
                let child_address = add(b, address, offset)?;
                drop_value(c, b, child.index(), child_address)?;
            }
            Ok(())
        }
        TypeCategory::Enum => {
            let done = b.create_block();
            let tag = b.ins().load(types::I32, MemFlagsData::new(), address, 0);
            for (ordinal, payload) in view.variants() {
                let active = b.create_block();
                let next = b.create_block();
                let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
                b.ins().brif(test, active, &[], next, &[]);
                b.switch_to_block(active);
                if let Some(child) = payload {
                    let child_address = add(b, address, view.payload().ok_or_else(invariant)?.0)?;
                    drop_value(c, b, child.index(), child_address)?;
                }
                b.ins().jump(done, &[]);
                b.switch_to_block(next);
            }
            b.ins().trap(TrapCode::unwrap_user(7));
            b.switch_to_block(done);
            Ok(())
        }
        TypeCategory::FixedArray => {
            let child = view.referenced_type().ok_or_else(invariant)?;
            for index in (0..view.array_length().ok_or_else(invariant)?).rev() {
                let offset = index
                    .checked_mul(view.array_stride().ok_or_else(invariant)?)
                    .ok_or_else(invariant)?;
                let child_address = add(b, address, offset)?;
                drop_value(c, b, child.index(), child_address)?;
            }
            Ok(())
        }
        _ => Err(invariant()),
    }
}
pub(super) fn release(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    address: Value,
) -> Result<(), Diagnostic> {
    let call = b.ins().call(*c.runtime.get(2).ok_or_else(invariant)?, &[address]);
    let status = b.inst_results(call)[0];
    b.ins().trapnz(status, TrapCode::unwrap_user(7));
    Ok(())
}
