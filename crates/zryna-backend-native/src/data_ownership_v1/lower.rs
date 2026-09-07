use std::collections::BTreeMap;

use cranelift_codegen::ir::{
    FuncRef, InstBuilder, MemFlagsData, StackSlot, StackSlotData, StackSlotKind, TrapCode,
    condcodes::IntCC, types,
};
use cranelift_codegen::{Context, isa::TargetFrontendConfig};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedCallArgument, VerifiedFunction, VerifiedImmediate, VerifiedMirModule,
    VerifiedOperation,
    raw::{Opcode, PlaceKind, TypeCategory},
};

use super::clone as clone_ops;
use super::control;
use super::drop as drop_ops;
use super::invariant_error;
use super::runtime as runtime_ops;
use super::state::{
    borrow_capacity, get_borrow, get_borrow_type, get_value, runtime_function, set_borrow,
    set_borrow_type, set_value, value_capacity,
};
use super::storage::{
    allocate_place_slots, copy_from_address, copy_place_value, index_value, indexed_address,
    indexed_borrow_address, initialize_parameter_places, move_place_value, place_storage_address,
    store_owned_typed, store_place, type_record,
};

pub(super) fn native_type(
    program: &VerifiedMirModule,
    id: u32,
) -> Result<cranelift_codegen::ir::Type, Diagnostic> {
    Ok(match type_record(program, id)?.category() {
        TypeCategory::Bool => types::I8,
        TypeCategory::I32 => types::I32,
        TypeCategory::Struct
        | TypeCategory::Enum
        | TypeCategory::FixedArray
        | TypeCategory::String
        | TypeCategory::Vec
        | TypeCategory::Shared
        | TypeCategory::Weak => types::I64,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_function(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    context: &mut Context,
    builder_context: &mut FunctionBuilderContext,
    callees: &BTreeMap<(u32, u32), FuncRef>,
    runtime: &BTreeMap<&str, FuncRef>,
    clones: &BTreeMap<u32, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    frontend: TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    let mut builder = FunctionBuilder::new(&mut context.func, builder_context);
    let blocks = function.blocks().collect::<Vec<_>>();
    let encoded = blocks.iter().map(|_| builder.create_block()).collect::<Vec<_>>();
    let entry = *encoded.first().ok_or_else(invariant_error)?;
    let mut values = vec![None; value_capacity(function)?];
    let mut borrows = vec![None; borrow_capacity(function)?];
    let mut borrow_types = vec![None; borrows.len()];
    let slots = allocate_place_slots(function, &mut builder)?;

    builder.append_block_params_for_function_params(entry);
    let parameters = builder.block_params(entry).to_vec();
    let value_parameter_count = function.parameters().len();
    for (parameter, value) in function.parameters().zip(parameters.iter().copied()) {
        set_value(&mut values, parameter.id(), value)?;
    }
    for (parameter, value) in
        function.borrow_parameters().zip(parameters.into_iter().skip(value_parameter_count))
    {
        set_borrow(&mut borrows, parameter.id(), value)?;
        set_borrow_type(&mut borrow_types, parameter.id(), parameter.referent())?;
    }
    for (block, encoded_block) in blocks.iter().zip(&encoded) {
        for parameter in block.parameters() {
            builder.append_block_param(*encoded_block, native_type(program, parameter.ty())?);
            let value = *builder.block_params(*encoded_block).last().ok_or_else(invariant_error)?;
            set_value(&mut values, parameter.id(), value)?;
        }
    }
    for (block, encoded_block) in blocks.iter().copied().zip(&encoded) {
        builder.switch_to_block(*encoded_block);
        if block.id() == 0 {
            initialize_parameter_places(function, &slots, &values, &mut builder)?;
        }
        // Block parameters can own temporaries, including an upgraded Shared value.
        for parameter in block.parameters() {
            if let Some(place) = function.places().find(
                |place| matches!(place.kind(), PlaceKind::Temporary(id) if *id == parameter.id()),
            ) {
                store_place(
                    program,
                    function,
                    place.id(),
                    get_value(&values, parameter.id())?,
                    &slots,
                    runtime,
                    &mut builder,
                )?;
            }
        }
        for operation in block.operations() {
            lower_operation(
                program,
                function,
                operation,
                &slots,
                &mut values,
                &mut borrows,
                &mut borrow_types,
                callees,
                runtime,
                clones,
                drops,
                &mut builder,
            )?;
        }
        control::lower_terminator(
            program,
            function,
            block.terminator(),
            block.cleanup(),
            &encoded,
            &slots,
            &values,
            runtime,
            drops,
            &mut builder,
        )?;
    }
    builder.seal_all_blocks();
    builder.finalize(frontend);
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_operation(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    operation: VerifiedOperation<'_>,
    slots: &[Option<StackSlot>],
    values: &mut [Option<cranelift_codegen::ir::Value>],
    borrows: &mut [Option<cranelift_codegen::ir::Value>],
    borrow_types: &mut [Option<u32>],
    callees: &BTreeMap<(u32, u32), FuncRef>,
    runtime: &BTreeMap<&str, FuncRef>,
    clones: &BTreeMap<u32, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let operand = |index: usize| {
        operation
            .values()
            .get(index)
            .copied()
            .ok_or_else(invariant_error)
            .and_then(|id| get_value(values, id))
    };
    let result = match operation.opcode() {
        Opcode::BoolLiteral => match operation.immediate() {
            VerifiedImmediate::Bool(value) => {
                Some(builder.ins().iconst(types::I8, i64::from(value)))
            }
            _ => return Err(invariant_error()),
        },
        Opcode::I32Literal => match operation.immediate() {
            VerifiedImmediate::I32(value) => {
                Some(builder.ins().iconst(types::I32, i64::from(value)))
            }
            _ => return Err(invariant_error()),
        },
        Opcode::I32Add => Some(builder.ins().iadd(operand(0)?, operand(1)?)),
        Opcode::I32Sub => Some(builder.ins().isub(operand(0)?, operand(1)?)),
        Opcode::I32Mul => Some(builder.ins().imul(operand(0)?, operand(1)?)),
        Opcode::I32Neg => Some(builder.ins().ineg(operand(0)?)),
        Opcode::Eq => Some(builder.ins().icmp(IntCC::Equal, operand(0)?, operand(1)?)),
        Opcode::Ne => Some(builder.ins().icmp(IntCC::NotEqual, operand(0)?, operand(1)?)),
        Opcode::I32LtS => Some(builder.ins().icmp(IntCC::SignedLessThan, operand(0)?, operand(1)?)),
        Opcode::I32LeS => {
            Some(builder.ins().icmp(IntCC::SignedLessThanOrEqual, operand(0)?, operand(1)?))
        }
        Opcode::I32GtS => {
            Some(builder.ins().icmp(IntCC::SignedGreaterThan, operand(0)?, operand(1)?))
        }
        Opcode::I32GeS => {
            Some(builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, operand(0)?, operand(1)?))
        }
        Opcode::Call => {
            let callee = *callees
                .get(&operation.callee().ok_or_else(invariant_error)?)
                .ok_or_else(invariant_error)?;
            let arguments = operation
                .call_arguments()
                .map(|argument| match argument {
                    VerifiedCallArgument::Value(id) => get_value(values, id),
                    VerifiedCallArgument::Borrow(id) => get_borrow(borrows, id),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let call = builder.ins().call(callee, &arguments);
            Some(*builder.inst_results(call).first().ok_or_else(invariant_error)?)
        }
        Opcode::Construct => Some(construct(program, operation, values, runtime, builder)?),
        Opcode::Copy => {
            let place = *operation.places().first().ok_or_else(invariant_error)?;
            Some(copy_place_value(program, function, place, slots, runtime, builder)?)
        }
        Opcode::Move => {
            let place = *operation.places().first().ok_or_else(invariant_error)?;
            Some(move_place_value(program, function, place, slots, runtime, builder)?)
        }
        Opcode::Initialize | Opcode::Replace => {
            let place = *operation.places().first().ok_or_else(invariant_error)?;
            if operation.opcode() == Opcode::Replace {
                drop_ops::drop_place(program, function, place, slots, runtime, drops, builder)?;
            }
            let value = operand(0)?;
            store_place(program, function, place, value, slots, runtime, builder)?;
            None
        }
        Opcode::Drop => {
            let place = *operation.places().first().ok_or_else(invariant_error)?;
            drop_ops::drop_place(program, function, place, slots, runtime, drops, builder)?;
            None
        }
        Opcode::Discriminant => {
            let place = *operation.places().first().ok_or_else(invariant_error)?;
            let address = place_storage_address(program, function, place, slots, builder)?;
            Some(builder.ins().load(types::I32, MemFlagsData::new(), address, 0))
        }
        Opcode::Index => Some(index_value(
            program,
            function,
            *operation.places().first().ok_or_else(invariant_error)?,
            operand(0)?,
            slots,
            runtime,
            builder,
        )?),
        Opcode::BeginBorrow | Opcode::BeginIndexedBorrow => {
            let id = *operation.borrows().first().ok_or_else(invariant_error)?;
            let place = *operation.places().first().ok_or_else(invariant_error)?;
            let address = if operation.opcode() == Opcode::BeginBorrow {
                place_storage_address(program, function, place, slots, builder)?
            } else {
                indexed_address(program, function, place, operand(0)?, slots, builder)?
            };
            set_borrow(borrows, id, address)?;
            set_borrow_type(
                borrow_types,
                id,
                operation.borrow_type().ok_or_else(invariant_error)?,
            )?;
            None
        }
        Opcode::BindBorrow => {
            let parent = get_borrow(borrows, operation.borrows()[0])?;
            set_borrow(borrows, operation.borrows()[1], parent)?;
            set_borrow_type(
                borrow_types,
                operation.borrows()[1],
                operation.borrow_type().ok_or_else(invariant_error)?,
            )?;
            None
        }
        Opcode::BorrowRead => {
            let address = get_borrow(borrows, operation.borrows()[0])?;
            let ty = operation.result().ok_or_else(invariant_error)?.ty();
            Some(copy_from_address(program, ty, address, runtime, builder)?)
        }
        Opcode::BorrowWrite | Opcode::BorrowReplace => {
            let address = get_borrow(borrows, operation.borrows()[0])?;
            let ty = operation.borrow_type().ok_or_else(invariant_error)?;
            if operation.opcode() == Opcode::BorrowReplace {
                drop_ops::drop_contents(ty, address, drops, builder)?;
            }
            let value = operand(0)?;
            store_owned_typed(program, ty, address, value, runtime, builder)?;
            None
        }
        Opcode::EndBorrow => None,
        Opcode::Clone | Opcode::StringClone | Opcode::VecClone => {
            let source = if let Some(place) = operation.places().first() {
                place_storage_address(program, function, *place, slots, builder)?
            } else {
                get_borrow(borrows, *operation.borrows().first().ok_or_else(invariant_error)?)?
            };
            Some(clone_ops::clone_value(program, operation, source, runtime, clones, builder)?)
        }
        Opcode::String => Some(runtime_ops::string_literal(program, operation, runtime, builder)?),
        Opcode::StringConcat => {
            Some(runtime_ops::string_concat(program, function, operation, slots, runtime, builder)?)
        }
        Opcode::VecConstruct => {
            let operands = operation
                .values()
                .iter()
                .map(|id| get_value(values, *id))
                .collect::<Result<Vec<_>, _>>()?;
            Some(runtime_ops::vec_construct(program, operation, &operands, runtime, builder)?)
        }
        Opcode::VecPush => {
            runtime_ops::vec_push(
                program,
                function,
                operation,
                operand(0)?,
                slots,
                runtime,
                builder,
            )?;
            None
        }
        Opcode::SharedConstruct => {
            Some(runtime_ops::shared_construct(program, operation, operand(0)?, runtime, builder)?)
        }
        Opcode::SharedClone | Opcode::WeakDowngrade | Opcode::WeakClone => {
            let source =
                place_storage_address(program, function, operation.places()[0], slots, builder)?;
            Some(runtime_ops::handle_transition(program, operation, source, runtime, builder)?)
        }
        Opcode::ProjectBorrow => {
            let parent_id = operation.borrows()[0];
            let child_id = operation.borrows()[1];
            let parent = get_borrow(borrows, parent_id)?;
            let parent_type = get_borrow_type(borrow_types, parent_id)?;
            let address =
                indexed_borrow_address(program, parent_type, parent, operand(0)?, builder)?;
            set_borrow(borrows, child_id, address)?;
            set_borrow_type(
                borrow_types,
                child_id,
                operation.borrow_type().ok_or_else(invariant_error)?,
            )?;
            None
        }
    };
    if let (Some(definition), Some(value)) = (operation.result(), result) {
        set_value(values, definition.id(), value)?;
        if let Some(place) = function.places().find(
            |place| matches!(place.kind(), PlaceKind::Temporary(id) if *id == definition.id()),
        ) {
            store_place(program, function, place.id(), value, slots, runtime, builder)?;
        }
    } else if operation.result().is_some() != result.is_some() {
        return Err(invariant_error());
    }
    Ok(())
}

fn construct(
    program: &VerifiedMirModule,
    operation: VerifiedOperation<'_>,
    values: &[Option<cranelift_codegen::ir::Value>],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let result = operation.result().ok_or_else(invariant_error)?;
    let layout = type_record(program, result.ty())?;
    let output =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let out_address = builder.ins().stack_addr(types::I64, output, 0);
    let size = builder
        .ins()
        .iconst(types::I64, i64::try_from(layout.size()).map_err(|_| invariant_error())?);
    let alignment = builder
        .ins()
        .iconst(types::I32, i64::try_from(layout.alignment()).map_err(|_| invariant_error())?);
    let allocate = runtime_function(runtime, "zryna_rt_o1_allocate")?;
    let call = builder.ins().call(allocate, &[size, alignment, out_address]);
    let status = *builder.inst_results(call).first().ok_or_else(invariant_error)?;
    builder.ins().trapnz(status, TrapCode::unwrap_user(2));
    let pointer = builder.ins().stack_load(types::I64, types::I64, output, 0);
    if layout.category() == TypeCategory::Enum {
        let VerifiedImmediate::Variant(variant) = operation.immediate() else {
            return Err(invariant_error());
        };
        let tag = builder.ins().iconst(types::I32, i64::from(variant));
        builder.ins().store(MemFlagsData::new(), tag, pointer, 0);
    }
    for (index, id) in operation.values().iter().copied().enumerate() {
        let value = get_value(values, id)?;
        let (ty, offset) = construct_member(program, layout, operation.immediate(), index)?;
        let address = builder
            .ins()
            .iadd_imm_u(pointer, i64::try_from(offset).map_err(|_| invariant_error())?);
        store_owned_typed(program, ty, address, value, runtime, builder)?;
    }
    Ok(pointer)
}

fn construct_member(
    program: &VerifiedMirModule,
    layout: zryna_native_mir::data_ownership_v1::VerifiedType<'_>,
    immediate: VerifiedImmediate<'_>,
    index: usize,
) -> Result<(u32, u64), Diagnostic> {
    match layout.category() {
        TypeCategory::Struct => layout
            .fields()
            .get(index)
            .map(|field| (field.ty, field.offset))
            .ok_or_else(invariant_error),
        TypeCategory::FixedArray => Ok((
            layout.referenced_type().ok_or_else(invariant_error)?,
            layout
                .array_stride()
                .ok_or_else(invariant_error)?
                .checked_mul(u64::try_from(index).map_err(|_| invariant_error())?)
                .ok_or_else(invariant_error)?,
        )),
        TypeCategory::Enum => {
            let VerifiedImmediate::Variant(ordinal) = immediate else {
                return Err(invariant_error());
            };
            let ty = layout
                .variants()
                .iter()
                .find(|variant| variant.ordinal == ordinal)
                .and_then(|variant| variant.payload)
                .ok_or_else(invariant_error)?;
            Ok((ty, layout.enum_payload().ok_or_else(invariant_error)?.0))
        }
        _ => {
            let _ = program;
            Err(invariant_error())
        }
    }
}
