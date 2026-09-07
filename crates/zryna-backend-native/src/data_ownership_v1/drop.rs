use std::collections::BTreeMap;

use cranelift_codegen::{
    Context,
    ir::{
        AbiParam, BlockArg, FuncRef, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind,
        TrapCode, condcodes::IntCC, types,
    },
    isa::TargetFrontendConfig,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedFunction, VerifiedMirModule,
    raw::{DropKind, PlaceKind, TypeCategory},
};

use super::{
    invariant_error,
    runtime::{align_up, call_ok, release_record},
    storage::{place_storage_address, place_type, type_record},
};

pub(super) fn build_helper(
    program: &VerifiedMirModule,
    ty: u32,
    context: &mut Context,
    builder_context: &mut FunctionBuilderContext,
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    frontend: TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    context.func.signature.params.push(AbiParam::new(types::I64));
    context.func.signature.returns.push(AbiParam::new(types::I32));
    let mut builder = FunctionBuilder::new(&mut context.func, builder_context);
    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    let address = builder.block_params(entry)[0];
    drop_contents_impl(program, ty, address, runtime, drops, &mut builder)?;
    let ok = builder.ins().iconst(types::I32, 0);
    builder.ins().return_(&[ok]);
    builder.seal_all_blocks();
    builder.finalize(frontend);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn drop_place(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    place: u32,
    slots: &[Option<cranelift_codegen::ir::StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let ty = place_type(function, place)?;
    let address = place_storage_address(program, function, place, slots, builder)?;
    call_helper(drops, ty, address, builder)?;
    let root =
        function.places().find(|candidate| candidate.id() == place).ok_or_else(invariant_error)?;
    if matches!(
        root.kind(),
        PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_)
    ) && !matches!(type_record(program, ty)?.category(), TypeCategory::Bool | TypeCategory::I32)
    {
        release_record(program, ty, address, runtime, builder)?;
    }
    Ok(())
}

pub(super) fn drop_contents(
    ty: u32,
    address: cranelift_codegen::ir::Value,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    call_helper(drops, ty, address, builder)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_cleanup_plan(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    cleanup: Option<u32>,
    slots: &[Option<cranelift_codegen::ir::StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let Some(cleanup) = cleanup else { return Ok(()) };
    let plan = function.cleanup_plan(cleanup).ok_or_else(invariant_error)?;
    for action in plan.actions() {
        if action.kind() != DropKind::Place {
            return Err(invariant_error());
        }
        drop_place(program, function, action.place(), slots, runtime, drops, builder)?;
    }
    Ok(())
}

fn drop_contents_impl(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    match layout.category() {
        TypeCategory::Bool | TypeCategory::I32 => Ok(()),
        TypeCategory::String => call_ok(runtime, "zryna_rt_o1_string_release", &[address], builder),
        TypeCategory::Struct => {
            for field in layout.fields().iter().rev() {
                call_child(drops, field.ty, offset(address, field.offset, builder)?, builder)?;
            }
            Ok(())
        }
        TypeCategory::Enum => drop_enum(program, ty, address, drops, builder),
        TypeCategory::FixedArray => {
            let element = layout.referenced_type().ok_or_else(invariant_error)?;
            let stride = layout.array_stride().ok_or_else(invariant_error)?;
            for index in (0..layout.array_length().ok_or_else(invariant_error)?).rev() {
                let displacement = stride.checked_mul(index).ok_or_else(invariant_error)?;
                call_child(drops, element, offset(address, displacement, builder)?, builder)?;
            }
            Ok(())
        }
        TypeCategory::Vec => drop_vec(program, ty, address, runtime, drops, builder),
        TypeCategory::Shared => drop_shared(program, ty, address, runtime, drops, builder),
        TypeCategory::Weak => {
            let control = builder.ins().load(types::I64, MemFlagsData::new(), address, 0);
            let output = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                2,
            ));
            let output = builder.ins().stack_addr(types::I64, output, 0);
            call_ok(runtime, "zryna_rt_o1_weak_release", &[control, output], builder)
        }
    }
}

fn drop_enum(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    let (payload_offset, _) = layout.enum_payload().ok_or_else(invariant_error)?;
    let discriminant = builder.ins().load(types::I32, MemFlagsData::new(), address, 0);
    let done = builder.create_block();
    for variant in layout.variants() {
        let matched = builder.create_block();
        let next = builder.create_block();
        let condition =
            builder.ins().icmp_imm_u(IntCC::Equal, discriminant, i64::from(variant.ordinal));
        builder.ins().brif(condition, matched, &[], next, &[]);
        builder.switch_to_block(matched);
        if let Some(payload) = variant.payload {
            call_child(drops, payload, offset(address, payload_offset, builder)?, builder)?;
        }
        builder.ins().jump(done, &[]);
        builder.switch_to_block(next);
    }
    builder.ins().trap(TrapCode::unwrap_user(3));
    builder.switch_to_block(done);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn drop_vec(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let element = type_record(program, ty)?.referenced_type().ok_or_else(invariant_error)?;
    let element_layout = type_record(program, element)?;
    let stride = align_up(element_layout.size(), element_layout.alignment())?;
    let data = builder.ins().load(types::I64, MemFlagsData::new(), address, 0);
    let length = builder.ins().load(types::I64, MemFlagsData::new(), address, 8);
    let header = builder.create_block();
    let body = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(header, types::I64);
    builder.ins().jump(header, &[BlockArg::Value(length)]);
    builder.switch_to_block(header);
    let remaining = builder.block_params(header)[0];
    let has_item = builder.ins().icmp_imm_u(IntCC::NotEqual, remaining, 0);
    builder.ins().brif(has_item, body, &[], done, &[]);
    builder.switch_to_block(body);
    let index = builder.ins().iadd_imm_s(remaining, -1);
    let byte_offset =
        builder.ins().imul_imm_u(index, i64::try_from(stride).map_err(|_| invariant_error())?);
    let item = builder.ins().iadd(data, byte_offset);
    call_child(drops, element, item, builder)?;
    builder.ins().jump(header, &[BlockArg::Value(index)]);
    builder.switch_to_block(done);
    let element_id = builder.ins().iconst(types::I32, i64::from(element));
    call_ok(runtime, "zryna_rt_o1_vec_release_storage", &[element_id, address], builder)
}

#[allow(clippy::too_many_arguments)]
fn drop_shared(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let control = builder.ins().load(types::I64, MemFlagsData::new(), address, 0);
    let output =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4, 2));
    let output_address = builder.ins().stack_addr(types::I64, output, 0);
    call_ok(runtime, "zryna_rt_o1_strong_release_begin", &[control, output_address], builder)?;
    let is_last = builder.ins().stack_load(types::I64, types::I32, output, 0);
    let last = builder.create_block();
    let done = builder.create_block();
    let condition = builder.ins().icmp_imm_u(IntCC::NotEqual, is_last, 0);
    builder.ins().brif(condition, last, &[], done, &[]);
    builder.switch_to_block(last);
    let payload = type_record(program, ty)?.referenced_type().ok_or_else(invariant_error)?;
    let payload_layout = type_record(program, payload)?;
    let payload_offset = align_up(8, payload_layout.alignment())?;
    call_child(drops, payload, offset(control, payload_offset, builder)?, builder)?;
    call_ok(runtime, "zryna_rt_o1_strong_release_finish", &[control], builder)?;
    builder.ins().jump(done, &[]);
    builder.switch_to_block(done);
    Ok(())
}

fn call_child(
    drops: &BTreeMap<u32, FuncRef>,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    call_helper(drops, ty, address, builder)
}

fn call_helper(
    drops: &BTreeMap<u32, FuncRef>,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let function = *drops.get(&ty).ok_or_else(invariant_error)?;
    let call = builder.ins().call(function, &[address]);
    let status = *builder.inst_results(call).first().ok_or_else(invariant_error)?;
    builder.ins().trapnz(status, TrapCode::unwrap_user(2));
    Ok(())
}

fn offset(
    address: cranelift_codegen::ir::Value,
    displacement: u64,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    Ok(builder
        .ins()
        .iadd_imm_u(address, i64::try_from(displacement).map_err(|_| invariant_error())?))
}
