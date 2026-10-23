//! Reverse actual Vec elements, and last-strong payload before implicit-weak release.
use super::{Helper, invariant};
use crate::generic_execution_v1::records;
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::TypeCategory;
use zryna_ownership_runtime_abi::LogicalOperation as O;
pub(super) fn emit(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    address: Value,
) -> Result<(), Diagnostic> {
    let view = h.p.stored_type(h.ty).ok_or_else(invariant)?;
    match view.category() {
        TypeCategory::String => h.checked(b, O::StringRelease, &[address])?,
        TypeCategory::Struct => {
            let mut fields = super::reserve(view.fields().len())?;
            fields.extend(view.fields());
            for (_, child, offset) in fields.into_iter().rev() {
                let child_address = records::add(b, address, offset)?;
                h.refs.drop_value(h.p, b, child.index(), child_address)?;
            }
        }
        TypeCategory::Enum => {
            let tag = b.ins().load(types::I32, MemFlagsData::new(), address, 0);
            let done = b.create_block();
            for (ordinal, child) in view.variants() {
                let active = b.create_block();
                let next = b.create_block();
                let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
                b.ins().brif(test, active, &[], next, &[]);
                b.switch_to_block(active);
                if let Some(child) = child {
                    let child_address =
                        records::add(b, address, view.payload().ok_or_else(invariant)?.0)?;
                    h.refs.drop_value(h.p, b, child.index(), child_address)?;
                }
                b.ins().jump(done, &[]);
                b.switch_to_block(next);
            }
            b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
            b.switch_to_block(done);
        }
        TypeCategory::FixedArray => {
            let count = view.array_length().ok_or_else(invariant)?;
            if count != 0 {
                let length =
                    b.ins().iconst(types::I64, i64::try_from(count).map_err(super::codegen)?);
                super::sequence::drop(
                    h,
                    b,
                    view.referenced_type().ok_or_else(invariant)?.index(),
                    address,
                    length,
                    view.array_stride().ok_or_else(invariant)?,
                )?;
            }
        }
        TypeCategory::Vec => {
            let data = b.ins().load(types::I64, MemFlagsData::new(), address, 0);
            let length = b.ins().load(types::I64, MemFlagsData::new(), address, 8);
            let metadata = h.p.containers().and_then(|p| p.element(h.ty)).ok_or_else(invariant)?;
            super::sequence::drop(
                h,
                b,
                metadata.element().index(),
                data,
                length,
                metadata.stride(),
            )?;
            let id = b.ins().iconst(types::I32, i64::from(metadata.element().index()));
            h.checked(b, O::VecReleaseStorage, &[id, address])?;
        }
        TypeCategory::Shared => shared(h, b, address)?,
        TypeCategory::Weak => {
            let control = b.ins().load(types::I64, MemFlagsData::new(), address, 0);
            let slot =
                b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4, 2));
            let out = b.ins().stack_addr(types::I64, slot, 0);
            records::zero(b, out, 4)?;
            h.checked(b, O::WeakRelease, &[control, out])?;
        }
        TypeCategory::Bool | TypeCategory::I32 => {}
    }
    Ok(())
}
fn shared(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    address: Value,
) -> Result<(), Diagnostic> {
    let metadata = h.p.containers().and_then(|p| p.control(h.ty)).ok_or_else(invariant)?;
    let control = b.ins().load(types::I64, MemFlagsData::new(), address, 0);
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4, 2));
    let out = b.ins().stack_addr(types::I64, slot, 0);
    records::zero(b, out, 4)?;
    h.checked(b, O::StrongReleaseBegin, &[control, out])?;
    let last = b.ins().load(types::I32, MemFlagsData::new(), out, 0);
    let active = b.create_block();
    let done = b.create_block();
    b.ins().brif(last, active, &[], done, &[]);
    b.switch_to_block(active);
    let payload = records::add(b, control, metadata.payload_offset())?;
    h.refs.drop_value(h.p, b, metadata.payload().index(), payload)?;
    h.checked(b, O::StrongReleaseFinish, &[control])?;
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    Ok(())
}
