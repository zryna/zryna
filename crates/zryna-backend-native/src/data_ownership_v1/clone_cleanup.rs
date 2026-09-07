use super::{failure::Runtime, invariant_error, storage::type_record};
use cranelift_codegen::ir::{FuncRef, InstBuilder, StackSlot, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{VerifiedMirModule, raw::TypeCategory};

pub(super) fn mark(slot: StackSlot, count: u64, builder: &mut FunctionBuilder<'_>) {
    let value = builder.ins().iconst(types::I64, i64::try_from(count).unwrap_or(i64::MAX));
    builder.ins().stack_store(types::I64, value, slot, 0);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prefix(
    program: &VerifiedMirModule,
    ty: u32,
    destination: Value,
    initialized: StackSlot,
    runtime: &Runtime<'_>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    let count = builder.ins().stack_load(types::I64, types::I64, initialized, 0);
    let fields = match layout.category() {
        TypeCategory::Struct => {
            layout.fields().iter().map(|field| (field.ty, field.offset)).collect::<Vec<_>>()
        }
        TypeCategory::FixedArray => {
            let element = layout.referenced_type().ok_or_else(invariant_error)?;
            let stride = layout.array_stride().ok_or_else(invariant_error)?;
            (0..layout.array_length().ok_or_else(invariant_error)?)
                .map(|i| (element, i * stride))
                .collect()
        }
        TypeCategory::Vec => {
            // The runtime zeroes a failed allocation result; length advances only after each clone.
            super::drop::drop_contents(ty, destination, drops, builder)?;
            return Ok(());
        }
        _ => return Ok(()),
    };
    super::failure::record(0x10000002, runtime, builder)?;
    for (index, (child, offset)) in fields.iter().enumerate().rev() {
        let live = builder.ins().icmp_imm_u(
            IntCC::UnsignedGreaterThan,
            count,
            i64::try_from(index).map_err(|_| invariant_error())?,
        );
        let release = builder.create_block();
        let next = builder.create_block();
        builder.ins().brif(live, release, &[], next, &[]);
        builder.switch_to_block(release);
        let address = builder
            .ins()
            .iadd_imm_u(destination, i64::try_from(*offset).map_err(|_| invariant_error())?);
        super::drop::drop_contents(*child, address, drops, builder)?;
        builder.ins().jump(next, &[]);
        builder.switch_to_block(next);
    }
    Ok(())
}
