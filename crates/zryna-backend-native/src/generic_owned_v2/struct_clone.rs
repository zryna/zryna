//! Private flattened clone destination and progress; no carrier allocation or memory-record ABI.
use super::{body::get, codegen, effects::Context, invariant};
use cranelift_codegen::ir::{InstBuilder, StackSlot, StackSlotData, StackSlotKind, Value, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_owned_v2::{CloneNode, VerifiedStructClone};

mod cleanup;
mod emit;

struct Kernel<'c, 'p, 'a, 'v> {
    context: &'c Context<'c, 'p, 'a>,
    source: &'v [Value],
    destination: StackSlot,
    cursor: StackSlot,
    failed: cranelift_codegen::ir::Block,
}

pub(super) fn operation(
    context: &Context<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    values: &[Option<Vec<Value>>],
    clone: VerifiedStructClone<'_>,
) -> Result<Vec<Value>, Diagnostic> {
    let source = get(values, clone.source())?;
    let root = clone.root();
    if source.len() != root.width() as usize {
        return Err(invariant());
    }
    // The retained IR proof independently establishes this complete Copy operation.
    let Some(step) = clone.failure_step() else {
        return Ok(source.to_vec());
    };
    let destination = b.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        root.width().max(1).checked_mul(8).ok_or_else(invariant)?,
        3,
    ));
    let cursor = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().stack_store(types::I64, zero, cursor, 0);
    // Enum padding has a defined non-owning lane value on every successful path.
    for lane in 0..root.width() {
        b.ins().stack_store(types::I64, zero, destination, offset(lane)?);
    }
    let failed = b.create_block();
    b.append_block_param(failed, types::I32);
    let done = b.create_block();
    let kernel = Kernel { context, source, destination, cursor, failed };
    emit::node(&kernel, b, root)?;
    b.ins().jump(done, &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    let progress = b.ins().stack_load(types::I64, types::I64, cursor, 0);
    cleanup::node(&kernel, b, root, progress)?;
    super::effects::cleanup(context, b, values, step)?;
    b.ins().return_(&[status]);
    b.switch_to_block(done);
    let mut lanes = Vec::new();
    lanes.try_reserve(source.len()).map_err(|_| invariant())?;
    for lane in 0..root.width() {
        lanes.push(b.ins().stack_load(types::I64, types::I64, destination, offset(lane)?));
    }
    Ok(lanes)
}

fn offset(lane: u32) -> Result<i32, Diagnostic> {
    i32::try_from(lane.checked_mul(8).ok_or_else(invariant)?).map_err(codegen)
}

fn source<'v>(
    kernel: &Kernel<'_, '_, '_, 'v>,
    node: &CloneNode,
) -> Result<&'v [Value], Diagnostic> {
    let begin = node.begin() as usize;
    let end = begin.checked_add(node.width() as usize).ok_or_else(invariant)?;
    kernel.source.get(begin..end).ok_or_else(invariant)
}
