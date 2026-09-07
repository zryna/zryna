use std::collections::BTreeMap;

use cranelift_codegen::ir::{
    FuncRef, InstBuilder, MemFlagsData, StackSlot, StackSlotData, StackSlotKind, condcodes::IntCC,
    types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedFunction, VerifiedMirModule, VerifiedType,
    raw::{PlaceKind, TypeCategory},
};

use super::{
    invariant_error,
    lower::native_type,
    runtime::{allocate_record, release_record},
};

pub(super) fn allocate_place_slots(
    function: VerifiedFunction<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Vec<Option<StackSlot>>, Diagnostic> {
    function
        .places()
        .map(|place| match place.kind() {
            PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_) => {
                Ok(Some(builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8,
                    3,
                ))))
            }
            PlaceKind::Field { .. }
            | PlaceKind::EnumPayload { .. }
            | PlaceKind::ArrayElement { .. } => Ok(None),
        })
        .collect()
}

pub(super) fn initialize_parameter_places(
    function: VerifiedFunction<'_>,
    slots: &[Option<StackSlot>],
    values: &[Option<cranelift_codegen::ir::Value>],
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    for place in function.places() {
        if let PlaceKind::Parameter(ordinal) = place.kind() {
            let parameter = function
                .parameters()
                .nth(usize::try_from(*ordinal).map_err(|_| invariant_error())?)
                .ok_or_else(invariant_error)?;
            let value = values
                .get(usize::try_from(parameter.id()).map_err(|_| invariant_error())?)
                .and_then(|value| *value)
                .ok_or_else(invariant_error)?;
            let address = root_address(slots, place.id(), builder)?;
            builder.ins().store(MemFlagsData::new(), value, address, 0);
        }
    }
    Ok(())
}

pub(super) fn place_value(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    id: u32,
    slots: &[Option<StackSlot>],
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let place = function.places().find(|place| place.id() == id).ok_or_else(invariant_error)?;
    let ty = place.ty();
    let address = place_address(program, function, id, slots, builder)?;
    if matches!(
        place.kind(),
        PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_)
    ) && !matches!(type_record(program, ty)?.category(), TypeCategory::Bool | TypeCategory::I32)
    {
        Ok(builder.ins().load(types::I64, MemFlagsData::new(), address, 0))
    } else {
        load_typed(program, ty, address, builder)
    }
}

pub(super) fn place_storage_address(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    id: u32,
    slots: &[Option<StackSlot>],
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let place = function.places().find(|place| place.id() == id).ok_or_else(invariant_error)?;
    if matches!(
        place.kind(),
        PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_)
    ) && !matches!(
        type_record(program, place.ty())?.category(),
        TypeCategory::Bool | TypeCategory::I32
    ) {
        place_value(program, function, id, slots, builder)
    } else {
        place_address(program, function, id, slots, builder)
    }
}

fn place_address(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    id: u32,
    slots: &[Option<StackSlot>],
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let place = function.places().find(|place| place.id() == id).ok_or_else(invariant_error)?;
    match place.kind() {
        PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_) => {
            root_address(slots, id, builder)
        }
        PlaceKind::Field { base, offset }
        | PlaceKind::ArrayElement { base, offset }
        | PlaceKind::EnumPayload { base, offset, .. } => {
            offset_address(program, function, *base, *offset, slots, builder)
        }
    }
}

fn offset_address(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    base: u32,
    offset: u64,
    slots: &[Option<StackSlot>],
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let base = place_storage_address(program, function, base, slots, builder)?;
    Ok(builder.ins().iadd_imm_u(base, i64::try_from(offset).map_err(|_| invariant_error())?))
}

pub(super) fn store_place(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    id: u32,
    value: cranelift_codegen::ir::Value,
    slots: &[Option<StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let ty = place_type(function, id)?;
    let address = place_address(program, function, id, slots, builder)?;
    let place = function.places().find(|place| place.id() == id).ok_or_else(invariant_error)?;
    if matches!(
        place.kind(),
        PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_)
    ) && !matches!(type_record(program, ty)?.category(), TypeCategory::Bool | TypeCategory::I32)
    {
        builder.ins().store(MemFlagsData::new(), value, address, 0);
        Ok(())
    } else {
        store_owned_typed(program, ty, address, value, runtime, builder)
    }
}

pub(super) fn copy_place_value(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    id: u32,
    slots: &[Option<StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let ty = place_type(function, id)?;
    let address = place_storage_address(program, function, id, slots, builder)?;
    copy_from_address(program, ty, address, runtime, builder)
}

pub(super) fn move_place_value(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    id: u32,
    slots: &[Option<StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let place = function.places().find(|place| place.id() == id).ok_or_else(invariant_error)?;
    let ty = place.ty();
    let layout = type_record(program, ty)?;
    if matches!(layout.category(), TypeCategory::Bool | TypeCategory::I32)
        || matches!(
            place.kind(),
            PlaceKind::Parameter(_) | PlaceKind::Local(_) | PlaceKind::Temporary(_)
        )
    {
        return place_value(program, function, id, slots, builder);
    }
    let source = place_storage_address(program, function, id, slots, builder)?;
    let result = allocate_record(program, ty, runtime, builder)?;
    copy_memory(source, result, layout.size(), builder)?;
    Ok(result)
}

pub(super) fn copy_from_address(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let layout = type_record(program, ty)?;
    if matches!(layout.category(), TypeCategory::Bool | TypeCategory::I32) {
        return load_typed(program, ty, address, builder);
    }
    let result = allocate_record(program, ty, runtime, builder)?;
    copy_memory(address, result, layout.size(), builder)?;
    Ok(result)
}

pub(super) fn load_typed(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    Ok(if matches!(type_record(program, ty)?.category(), TypeCategory::Bool | TypeCategory::I32) {
        builder.ins().load(native_type(program, ty)?, MemFlagsData::new(), address, 0)
    } else {
        address
    })
}

pub(super) fn store_typed(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let layout = type_record(program, ty)?;
    if matches!(layout.category(), TypeCategory::Bool | TypeCategory::I32) {
        builder.ins().store(MemFlagsData::new(), value, address, 0);
        return Ok(());
    }
    copy_memory(value, address, layout.size(), builder)?;
    Ok(())
}

pub(super) fn store_owned_typed(
    program: &VerifiedMirModule,
    ty: u32,
    address: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    runtime: &BTreeMap<&str, FuncRef>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    store_typed(program, ty, address, value, builder)?;
    if !matches!(type_record(program, ty)?.category(), TypeCategory::Bool | TypeCategory::I32) {
        release_record(program, ty, value, runtime, builder)?;
    }
    Ok(())
}

pub(super) fn copy_memory(
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    size: u64,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let mut offset = 0_u64;
    for (width, ty) in [(8_u64, types::I64), (4, types::I32), (2, types::I16), (1, types::I8)] {
        while size.saturating_sub(offset) >= width {
            let displacement = i32::try_from(offset).map_err(|_| invariant_error())?;
            let value = builder.ins().load(ty, MemFlagsData::new(), source, displacement);
            builder.ins().store(MemFlagsData::new(), value, destination, displacement);
            offset = offset.checked_add(width).ok_or_else(invariant_error)?;
        }
    }
    if offset != size {
        return Err(invariant_error());
    }
    Ok(())
}

pub(super) fn indexed_address(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    place: u32,
    index: cranelift_codegen::ir::Value,
    slots: &[Option<StackSlot>],
    failed: cranelift_codegen::ir::Block,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let layout = type_record(program, place_type(function, place)?)?;
    let base = place_storage_address(program, function, place, slots, builder)?;
    let (data, length, stride) = match layout.category() {
        TypeCategory::FixedArray => (
            base,
            layout.array_length().ok_or_else(invariant_error)?,
            layout.array_stride().ok_or_else(invariant_error)?,
        ),
        TypeCategory::Vec => {
            let data = builder.ins().load(types::I64, MemFlagsData::new(), base, 0);
            let length = builder.ins().load(types::I64, MemFlagsData::new(), base, 8);
            let extended = builder.ins().uextend(types::I64, index);
            let ok = builder.ins().icmp(IntCC::UnsignedLessThan, extended, length);
            super::failure::bounds(ok, failed, builder);
            let stride =
                type_record(program, layout.referenced_type().ok_or_else(invariant_error)?)?.size();
            return scaled_address(data, index, stride, builder);
        }
        _ => return Err(invariant_error()),
    };
    let limit =
        builder.ins().iconst(types::I32, i64::try_from(length).map_err(|_| invariant_error())?);
    let ok = builder.ins().icmp(IntCC::UnsignedLessThan, index, limit);
    super::failure::bounds(ok, failed, builder);
    scaled_address(data, index, stride, builder)
}

pub(super) fn indexed_borrow_address(
    program: &VerifiedMirModule,
    container_type: u32,
    container: cranelift_codegen::ir::Value,
    index: cranelift_codegen::ir::Value,
    failed: cranelift_codegen::ir::Block,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let layout = type_record(program, container_type)?;
    match layout.category() {
        TypeCategory::FixedArray => {
            let length = builder.ins().iconst(
                types::I32,
                i64::try_from(layout.array_length().ok_or_else(invariant_error)?)
                    .map_err(|_| invariant_error())?,
            );
            let ok = builder.ins().icmp(IntCC::UnsignedLessThan, index, length);
            super::failure::bounds(ok, failed, builder);
            scaled_address(
                container,
                index,
                layout.array_stride().ok_or_else(invariant_error)?,
                builder,
            )
        }
        TypeCategory::Vec => {
            let data = builder.ins().load(types::I64, MemFlagsData::new(), container, 0);
            let length = builder.ins().load(types::I64, MemFlagsData::new(), container, 8);
            let extended = builder.ins().uextend(types::I64, index);
            let ok = builder.ins().icmp(IntCC::UnsignedLessThan, extended, length);
            super::failure::bounds(ok, failed, builder);
            let element = layout.referenced_type().ok_or_else(invariant_error)?;
            let element = type_record(program, element)?;
            scaled_address(data, index, align_up(element.size(), element.alignment())?, builder)
        }
        _ => Err(invariant_error()),
    }
}

fn align_up(value: u64, alignment: u64) -> Result<u64, Diagnostic> {
    value
        .checked_add(alignment.checked_sub(1).ok_or_else(invariant_error)?)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(invariant_error)
}

fn scaled_address(
    data: cranelift_codegen::ir::Value,
    index: cranelift_codegen::ir::Value,
    stride: u64,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let extended = builder.ins().uextend(types::I64, index);
    let offset =
        builder.ins().imul_imm_u(extended, i64::try_from(stride).map_err(|_| invariant_error())?);
    Ok(builder.ins().iadd(data, offset))
}

pub(super) fn index_value(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    place: u32,
    index: cranelift_codegen::ir::Value,
    slots: &[Option<StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    failed: cranelift_codegen::ir::Block,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let container = type_record(program, place_type(function, place)?)?;
    let element = container.referenced_type().ok_or_else(invariant_error)?;
    let address = indexed_address(program, function, place, index, slots, failed, builder)?;
    copy_from_address(program, element, address, runtime, builder)
}

pub(super) fn type_record(
    program: &VerifiedMirModule,
    id: u32,
) -> Result<VerifiedType<'_>, Diagnostic> {
    program.types().find(|ty| ty.id() == id).ok_or_else(invariant_error)
}

pub(super) fn place_type(function: VerifiedFunction<'_>, id: u32) -> Result<u32, Diagnostic> {
    function
        .places()
        .find(|place| place.id() == id)
        .map(zryna_native_mir::data_ownership_v1::VerifiedPlace::ty)
        .ok_or_else(invariant_error)
}

fn root_address(
    slots: &[Option<StackSlot>],
    id: u32,
    builder: &mut FunctionBuilder<'_>,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    let slot = slots
        .get(usize::try_from(id).map_err(|_| invariant_error())?)
        .and_then(|slot| *slot)
        .ok_or_else(invariant_error)?;
    Ok(builder.ins().stack_addr(types::I64, slot, 0))
}
