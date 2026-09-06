use std::collections::BTreeMap;

use cranelift_codegen::{
    Context,
    ir::{AbiParam, BlockArg, FuncRef, InstBuilder, MemFlagsData, TrapCode, types},
    isa::TargetFrontendConfig,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedMirModule, VerifiedOperation, raw::TypeCategory,
};

use super::{
    invariant_error,
    runtime::{align_up, allocate_record, call_ok},
    storage::type_record,
};

pub(super) fn build_helper(
    program: &VerifiedMirModule,
    ty: u32,
    context: &mut Context,
    builder_context: &mut FunctionBuilderContext,
    runtime: &BTreeMap<&str, FuncRef>,
    clones: &BTreeMap<u32, FuncRef>,
    frontend: TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    context.func.signature.params.extend([AbiParam::new(types::I64), AbiParam::new(types::I64)]);
    context.func.signature.returns.push(AbiParam::new(types::I32));
    let mut builder = FunctionBuilder::new(&mut context.func, builder_context);
    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    let source = builder.block_params(entry)[0];
    let destination = builder.block_params(entry)[1];
    clone_into(program, ty, source, destination, runtime, clones, &mut builder)?;
    let ok = builder.ins().iconst(types::I32, 0);
    builder.ins().return_(&[ok]);
    builder.seal_all_blocks();
    builder.finalize(frontend);
    Ok(())
}

pub(super) fn clone_value(
    program: &VerifiedMirModule,
    operation: VerifiedOperation<'_>,
    source: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    clones: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let ty = operation.result().ok_or_else(invariant_error)?.ty();
    let layout = type_record(program, ty)?;
    if matches!(layout.category(), TypeCategory::Bool | TypeCategory::I32) {
        let native = if layout.category() == TypeCategory::Bool { types::I8 } else { types::I32 };
        return Ok(builder.ins().load(native, MemFlagsData::new(), source, 0));
    }
    let result = allocate_record(program, ty, runtime, builder)?;
    call_helper(clones, ty, source, result, builder)?;
    Ok(result)
}

fn clone_into(
    program: &VerifiedMirModule,
    ty: u32,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    clones: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    match layout.category() {
        TypeCategory::Bool => copy_scalar(types::I8, source, destination, builder),
        TypeCategory::I32 => copy_scalar(types::I32, source, destination, builder),
        TypeCategory::String => {
            call_ok(runtime, "zryna_rt_o1_string_clone", &[source, destination], builder)
        }
        TypeCategory::Struct => {
            for field in layout.fields() {
                call_child(
                    clones,
                    field.ty,
                    offset(source, field.offset, builder)?,
                    offset(destination, field.offset, builder)?,
                    builder,
                )?;
            }
            Ok(())
        }
        TypeCategory::Enum => clone_enum(program, ty, source, destination, clones, builder),
        TypeCategory::FixedArray => {
            let element = layout.referenced_type().ok_or_else(invariant_error)?;
            let stride = layout.array_stride().ok_or_else(invariant_error)?;
            for index in 0..layout.array_length().ok_or_else(invariant_error)? {
                let displacement = stride.checked_mul(index).ok_or_else(invariant_error)?;
                call_child(
                    clones,
                    element,
                    offset(source, displacement, builder)?,
                    offset(destination, displacement, builder)?,
                    builder,
                )?;
            }
            Ok(())
        }
        TypeCategory::Vec => clone_vec(program, ty, source, destination, runtime, clones, builder),
        TypeCategory::Shared | TypeCategory::Weak => {
            let control = builder.ins().load(types::I64, MemFlagsData::new(), source, 0);
            let symbol = if layout.category() == TypeCategory::Shared {
                "zryna_rt_o1_strong_clone"
            } else {
                "zryna_rt_o1_weak_clone"
            };
            call_ok(runtime, symbol, &[control], builder)?;
            builder.ins().store(MemFlagsData::new(), control, destination, 0);
            Ok(())
        }
    }
}

fn clone_enum(
    program: &VerifiedMirModule,
    ty: u32,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    clones: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    let (payload_offset, _) = layout.enum_payload().ok_or_else(invariant_error)?;
    let discriminant = builder.ins().load(types::I32, MemFlagsData::new(), source, 0);
    builder.ins().store(MemFlagsData::new(), discriminant, destination, 0);
    let done = builder.create_block();
    for variant in layout.variants() {
        let matched = builder.create_block();
        let next = builder.create_block();
        let condition = builder.ins().icmp_imm_u(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            discriminant,
            i64::from(variant.ordinal),
        );
        builder.ins().brif(condition, matched, &[], next, &[]);
        builder.switch_to_block(matched);
        if let Some(payload) = variant.payload {
            call_child(
                clones,
                payload,
                offset(source, payload_offset, builder)?,
                offset(destination, payload_offset, builder)?,
                builder,
            )?;
        }
        builder.ins().jump(done, &[]);
        builder.switch_to_block(next);
    }
    builder.ins().trap(TrapCode::unwrap_user(3));
    builder.switch_to_block(done);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn clone_vec(
    program: &VerifiedMirModule,
    ty: u32,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    clones: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let element = type_record(program, ty)?.referenced_type().ok_or_else(invariant_error)?;
    let element_layout = type_record(program, element)?;
    let stride = align_up(element_layout.size(), element_layout.alignment())?;
    let length = builder.ins().load(types::I64, MemFlagsData::new(), source, 8);
    let element_id = builder.ins().iconst(types::I32, i64::from(element));
    call_ok(runtime, "zryna_rt_o1_vec_allocate", &[element_id, length, destination], builder)?;
    let source_data = builder.ins().load(types::I64, MemFlagsData::new(), source, 0);
    let destination_data = builder.ins().load(types::I64, MemFlagsData::new(), destination, 0);
    let header = builder.create_block();
    let body = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(header, types::I64);
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().jump(header, &[BlockArg::Value(zero)]);
    builder.switch_to_block(header);
    let index = builder.block_params(header)[0];
    let more = builder.ins().icmp(
        cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
        index,
        length,
    );
    builder.ins().brif(more, body, &[], done, &[]);
    builder.switch_to_block(body);
    let byte_offset =
        builder.ins().imul_imm_u(index, i64::try_from(stride).map_err(|_| invariant_error())?);
    let source_item = builder.ins().iadd(source_data, byte_offset);
    let destination_item = builder.ins().iadd(destination_data, byte_offset);
    call_child(clones, element, source_item, destination_item, builder)?;
    let next = builder.ins().iadd_imm_u(index, 1);
    builder.ins().jump(header, &[BlockArg::Value(next)]);
    builder.switch_to_block(done);
    builder.ins().store(MemFlagsData::new(), length, destination, 8);
    Ok(())
}

fn call_child(
    clones: &BTreeMap<u32, FuncRef>,
    ty: u32,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    call_helper(clones, ty, source, destination, builder)
}

fn call_helper(
    clones: &BTreeMap<u32, FuncRef>,
    ty: u32,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let function = *clones.get(&ty).ok_or_else(invariant_error)?;
    let call = builder.ins().call(function, &[source, destination]);
    let status = *builder.inst_results(call).first().ok_or_else(invariant_error)?;
    builder.ins().trapnz(status, TrapCode::unwrap_user(2));
    Ok(())
}

fn copy_scalar(
    ty: cranelift_codegen::ir::Type,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let value = builder.ins().load(ty, MemFlagsData::new(), source, 0);
    builder.ins().store(MemFlagsData::new(), value, destination, 0);
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
