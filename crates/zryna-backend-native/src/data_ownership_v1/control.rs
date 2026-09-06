use std::collections::BTreeMap;

use cranelift_codegen::ir::{
    Block, BlockArg, FuncRef, InstBuilder, MemFlagsData, StackSlot, TrapCode, condcodes::IntCC,
    types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedFunction, VerifiedMirModule,
    raw::{Edge, Terminator, TypeCategory},
};

use super::{
    drop as drop_ops, invariant_error,
    runtime::{allocate_record, release_record},
    state::{edge_arguments, encoded_block, get_value, runtime_function},
    storage::{place_storage_address, place_type, type_record},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_terminator(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    terminator: &Terminator,
    cleanup: Option<u32>,
    blocks: &[Block],
    slots: &[Option<StackSlot>],
    values: &[Option<cranelift_codegen::ir::Value>],
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    match terminator {
        Terminator::Return(id) => {
            let result = get_value(values, *id)?;
            drop_ops::execute_cleanup_plan(
                program, function, cleanup, slots, runtime, drops, builder,
            )?;
            builder.ins().return_(&[result])
        }
        Terminator::Jump(edge) => {
            let arguments = edge_arguments(values, &edge.arguments)?;
            builder.ins().jump(encoded_block(blocks, edge.target)?, &arguments)
        }
        Terminator::Branch { condition, when_true, when_false } => {
            let condition =
                builder.ins().icmp_imm_u(IntCC::NotEqual, get_value(values, *condition)?, 0);
            let yes = edge_arguments(values, &when_true.arguments)?;
            let no = edge_arguments(values, &when_false.arguments)?;
            builder.ins().brif(
                condition,
                encoded_block(blocks, when_true.target)?,
                &yes,
                encoded_block(blocks, when_false.target)?,
                &no,
            )
        }
        Terminator::EnumMatch { place, arms } => {
            lower_enum_match(program, function, *place, arms, blocks, slots, values, builder)?;
            return Ok(());
        }
        Terminator::WeakUpgrade { weak, success, expired, runtime_symbol } => {
            lower_weak_upgrade(
                program,
                function,
                *weak,
                success,
                expired,
                runtime_symbol,
                cleanup,
                blocks,
                slots,
                values,
                runtime,
                drops,
                builder,
            )?;
            return Ok(());
        }
        Terminator::Trap(identity) => {
            drop_ops::execute_cleanup_plan(
                program, function, cleanup, slots, runtime, drops, builder,
            )?;
            builder.ins().trap(TrapCode::unwrap_user(identity.saturating_add(10)))
        }
    };
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_enum_match(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    place: u32,
    arms: &[(u32, Edge)],
    blocks: &[Block],
    slots: &[Option<StackSlot>],
    values: &[Option<cranelift_codegen::ir::Value>],
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let address = place_storage_address(program, function, place, slots, builder)?;
    let tag = builder.ins().load(types::I32, MemFlagsData::new(), address, 0);
    for (variant, edge) in arms {
        let next = builder.create_block();
        let condition = builder.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(*variant));
        builder.ins().brif(
            condition,
            encoded_block(blocks, edge.target)?,
            &edge_arguments(values, &edge.arguments)?,
            next,
            &[],
        );
        builder.switch_to_block(next);
    }
    builder.ins().trap(TrapCode::unwrap_user(3));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_weak_upgrade(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    weak: u32,
    success: &Edge,
    expired: &Edge,
    runtime_symbol: &str,
    cleanup: Option<u32>,
    blocks: &[Block],
    slots: &[Option<StackSlot>],
    values: &[Option<cranelift_codegen::ir::Value>],
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let weak_type = type_record(program, place_type(function, weak)?)?;
    if weak_type.category() != TypeCategory::Weak {
        return Err(invariant_error());
    }
    let target = function
        .blocks()
        .find(|block| block.id() == success.target)
        .and_then(|block| block.parameters().next())
        .ok_or_else(invariant_error)?;
    let shared_type = type_record(program, target.ty())?;
    if shared_type.category() != TypeCategory::Shared
        || shared_type.referenced_type() != weak_type.referenced_type()
    {
        return Err(invariant_error());
    }
    let weak_address = place_storage_address(program, function, weak, slots, builder)?;
    let control = builder.ins().load(types::I64, MemFlagsData::new(), weak_address, 0);
    let result = allocate_record(program, target.ty(), runtime, builder)?;
    let call = builder.ins().call(runtime_function(runtime, runtime_symbol)?, &[control]);
    let status = *builder.inst_results(call).first().ok_or_else(invariant_error)?;
    let upgraded = builder.create_block();
    let not_upgraded = builder.create_block();
    let ok = builder.ins().icmp_imm_u(IntCC::Equal, status, 0);
    builder.ins().brif(ok, upgraded, &[], not_upgraded, &[]);

    builder.switch_to_block(upgraded);
    builder.ins().store(MemFlagsData::new(), control, result, 0);
    let mut success_arguments = vec![BlockArg::Value(result)];
    success_arguments.extend(edge_arguments(values, &success.arguments)?);
    builder.ins().jump(encoded_block(blocks, success.target)?, &success_arguments);

    builder.switch_to_block(not_upgraded);
    let is_expired = builder.ins().icmp_imm_u(IntCC::Equal, status, 5);
    let expired_block = builder.create_block();
    let failed = builder.create_block();
    builder.ins().brif(is_expired, expired_block, &[], failed, &[]);

    builder.switch_to_block(expired_block);
    release_record(program, target.ty(), result, runtime, builder)?;
    builder
        .ins()
        .jump(encoded_block(blocks, expired.target)?, &edge_arguments(values, &expired.arguments)?);

    builder.switch_to_block(failed);
    release_record(program, target.ty(), result, runtime, builder)?;
    drop_ops::execute_cleanup_plan(program, function, cleanup, slots, runtime, drops, builder)?;
    builder.ins().trap(TrapCode::unwrap_user(2));
    Ok(())
}
