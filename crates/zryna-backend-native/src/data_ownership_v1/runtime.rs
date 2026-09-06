use std::collections::BTreeMap;

use cranelift_codegen::ir::{
    FuncRef, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, TrapCode, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedFunction, VerifiedImmediate, VerifiedMirModule, VerifiedOperation, raw::Opcode,
};

use super::{
    invariant_error,
    storage::{place_storage_address, place_type, store_owned_typed, type_record},
};

pub(super) fn string_literal(
    program: &VerifiedMirModule,
    operation: VerifiedOperation<'_>,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let VerifiedImmediate::Utf8(bytes) = operation.immediate() else {
        return Err(invariant_error());
    };
    let result_ty = operation.result().ok_or_else(invariant_error)?.ty();
    let result = allocate_record(program, result_ty, runtime, builder)?;
    let byte_slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        u32::try_from(bytes.len().max(1)).map_err(|_| invariant_error())?,
        0,
    ));
    let bytes_address = builder.ins().stack_addr(types::I64, byte_slot, 0);
    for (index, byte) in bytes.iter().copied().enumerate() {
        let value = builder.ins().iconst(types::I8, i64::from(byte));
        builder.ins().stack_store(
            types::I64,
            value,
            byte_slot,
            i32::try_from(index).map_err(|_| invariant_error())?,
        );
    }
    call_ok(
        runtime,
        "zryna_rt_o1_string_from_utf8_copy",
        &[
            bytes_address,
            builder
                .ins()
                .iconst(types::I64, i64::try_from(bytes.len()).map_err(|_| invariant_error())?),
            result,
        ],
        builder,
    )?;
    Ok(result)
}

pub(super) fn string_concat(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    operation: VerifiedOperation<'_>,
    slots: &[Option<cranelift_codegen::ir::StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let result = allocate_record(
        program,
        operation.result().ok_or_else(invariant_error)?.ty(),
        runtime,
        builder,
    )?;
    let left = place_storage_address(program, function, operation.places()[0], slots, builder)?;
    let right = place_storage_address(program, function, operation.places()[1], slots, builder)?;
    call_ok(runtime, "zryna_rt_o1_string_concat", &[left, right, result], builder)?;
    Ok(result)
}

pub(super) fn vec_construct(
    program: &VerifiedMirModule,
    operation: VerifiedOperation<'_>,
    values: &[cranelift_codegen::ir::Value],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let ty = operation.result().ok_or_else(invariant_error)?.ty();
    let layout = type_record(program, ty)?;
    let element = layout.referenced_type().ok_or_else(invariant_error)?;
    let element_layout = type_record(program, element)?;
    let result = allocate_record(program, ty, runtime, builder)?;
    let capacity = builder
        .ins()
        .iconst(types::I64, i64::try_from(values.len()).map_err(|_| invariant_error())?);
    let element_id = builder.ins().iconst(types::I32, i64::from(element));
    call_ok(runtime, "zryna_rt_o1_vec_allocate", &[element_id, capacity, result], builder)?;
    let data = builder.ins().load(types::I64, MemFlagsData::new(), result, 0);
    let stride = align_up(element_layout.size(), element_layout.alignment())?;
    for (index, value) in values.iter().copied().enumerate() {
        let offset = stride
            .checked_mul(u64::try_from(index).map_err(|_| invariant_error())?)
            .ok_or_else(invariant_error)?;
        let address =
            builder.ins().iadd_imm_u(data, i64::try_from(offset).map_err(|_| invariant_error())?);
        store_owned_typed(program, element, address, value, runtime, builder)?;
    }
    builder.ins().store(MemFlagsData::new(), capacity, result, 8);
    Ok(result)
}

pub(super) fn vec_push(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    operation: VerifiedOperation<'_>,
    value: cranelift_codegen::ir::Value,
    slots: &[Option<cranelift_codegen::ir::StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let place = operation.places()[0];
    let vector = place_storage_address(program, function, place, slots, builder)?;
    let layout = type_record(program, place_type(function, place)?)?;
    let element = layout.referenced_type().ok_or_else(invariant_error)?;
    let element_layout = type_record(program, element)?;
    let old_length = builder.ins().load(types::I64, MemFlagsData::new(), vector, 8);
    let required = builder.ins().iadd_imm_u(old_length, 1);
    let output =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 24, 3));
    let output_address = builder.ins().stack_addr(types::I64, output, 0);
    let element_id = builder.ins().iconst(types::I32, i64::from(element));
    call_ok(
        runtime,
        "zryna_rt_o1_vec_reserve",
        &[element_id, vector, required, output_address],
        builder,
    )?;
    let data = builder.ins().stack_load(types::I64, types::I64, output, 0);
    let stride = align_up(element_layout.size(), element_layout.alignment())?;
    let offset =
        builder.ins().imul_imm_u(old_length, i64::try_from(stride).map_err(|_| invariant_error())?);
    let destination = builder.ins().iadd(data, offset);
    store_owned_typed(program, element, destination, value, runtime, builder)?;
    for offset in [0_i32, 8, 16] {
        let field = builder.ins().stack_load(types::I64, types::I64, output, offset);
        builder.ins().store(MemFlagsData::new(), field, vector, offset);
    }
    builder.ins().store(MemFlagsData::new(), required, vector, 8);
    Ok(())
}

pub(super) fn shared_construct(
    program: &VerifiedMirModule,
    operation: VerifiedOperation<'_>,
    value: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let result_ty = operation.result().ok_or_else(invariant_error)?.ty();
    let handle = type_record(program, result_ty)?;
    let payload_ty = handle.referenced_type().ok_or_else(invariant_error)?;
    let payload = type_record(program, payload_ty)?;
    let payload_offset = align_up(8, payload.alignment())?;
    let control_size = align_up(
        payload_offset.checked_add(payload.size()).ok_or_else(invariant_error)?,
        payload.alignment().max(4),
    )?;
    let control = allocate_bytes(control_size, payload.alignment().max(4), runtime, builder)?;
    let one = builder.ins().iconst(types::I32, 1);
    builder.ins().store(MemFlagsData::new(), one, control, 0);
    builder.ins().store(MemFlagsData::new(), one, control, 4);
    let destination = builder
        .ins()
        .iadd_imm_u(control, i64::try_from(payload_offset).map_err(|_| invariant_error())?);
    store_owned_typed(program, payload_ty, destination, value, runtime, builder)?;
    let result = allocate_record(program, result_ty, runtime, builder)?;
    builder.ins().store(MemFlagsData::new(), control, result, 0);
    Ok(result)
}

pub(super) fn handle_transition(
    program: &VerifiedMirModule,
    operation: VerifiedOperation<'_>,
    source: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let symbol = match operation.opcode() {
        Opcode::SharedClone => "zryna_rt_o1_strong_clone",
        Opcode::WeakDowngrade => "zryna_rt_o1_weak_downgrade",
        Opcode::WeakClone => "zryna_rt_o1_weak_clone",
        _ => return Err(invariant_error()),
    };
    let control = builder.ins().load(types::I64, MemFlagsData::new(), source, 0);
    call_ok(runtime, symbol, &[control], builder)?;
    let ty = operation.result().ok_or_else(invariant_error)?.ty();
    let result = allocate_record(program, ty, runtime, builder)?;
    builder.ins().store(MemFlagsData::new(), control, result, 0);
    Ok(result)
}

pub(super) fn allocate_record(
    program: &VerifiedMirModule,
    ty: u32,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let layout = type_record(program, ty)?;
    allocate_bytes(layout.size(), layout.alignment(), runtime, builder)
}

pub(super) fn release_record(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    let size = builder
        .ins()
        .iconst(types::I64, i64::try_from(layout.size()).map_err(|_| invariant_error())?);
    let alignment = builder
        .ins()
        .iconst(types::I32, i64::try_from(layout.alignment()).map_err(|_| invariant_error())?);
    call_ok(runtime, "zryna_rt_o1_release", &[address, size, alignment], builder)
}

pub(super) fn allocate_bytes(
    size: u64,
    alignment: u64,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let output =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let output_address = builder.ins().stack_addr(types::I64, output, 0);
    let size =
        builder.ins().iconst(types::I64, i64::try_from(size).map_err(|_| invariant_error())?);
    let alignment =
        builder.ins().iconst(types::I32, i64::try_from(alignment).map_err(|_| invariant_error())?);
    call_ok(runtime, "zryna_rt_o1_allocate", &[size, alignment, output_address], builder)?;
    Ok(builder.ins().stack_load(types::I64, types::I64, output, 0))
}

pub(super) fn call_ok(
    runtime: &BTreeMap<&str, FuncRef>,
    symbol: &str,
    arguments: &[cranelift_codegen::ir::Value],
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let function = *runtime.get(symbol).ok_or_else(invariant_error)?;
    let call = builder.ins().call(function, arguments);
    let status = *builder.inst_results(call).first().ok_or_else(invariant_error)?;
    builder.ins().trapnz(status, TrapCode::unwrap_user(2));
    Ok(())
}

pub(super) fn align_up(value: u64, alignment: u64) -> Result<u64, Diagnostic> {
    value
        .checked_add(alignment.checked_sub(1).ok_or_else(invariant_error)?)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(invariant_error)
}
