use wasm_encoder::{Function, Instruction, MemArg};
use zryna_ir::data_ownership_v1::VerifiedFunction;
use zryna_layout::{TypeCategory, TypeId, VerifiedLayouts};

use super::{Context, Locals, index_error, memory, operations};

const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };

pub(super) fn indexed_address(
    function: VerifiedFunction<'_>,
    place: u32,
    index: u32,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let container = operations::place_type(function, place, layouts)?;
    operations::place_value(function, place, locals, layouts, body)?;
    body.instruction(&Instruction::LocalSet(locals.heap));
    match container.category() {
        TypeCategory::FixedArray => {
            check_bound(index, container.array_length().ok_or_else(index_error)?, body)?;
            body.instruction(&Instruction::LocalGet(locals.heap));
            scaled_index(index, container.array_stride().ok_or_else(index_error)?, body)?;
        }
        TypeCategory::Vec => {
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::LocalGet(locals.heap));
            body.instruction(&Instruction::I32Const(4));
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::I32Load(WORD));
            bounds_trap(body);
            body.instruction(&Instruction::LocalGet(locals.heap));
            body.instruction(&Instruction::I32Load(WORD));
            let element = container
                .referenced_type()
                .and_then(|ty| layouts.type_by_id(ty))
                .ok_or_else(index_error)?;
            scaled_index(index, element.size().max(1), body)?;
        }
        _ => return Err(index_error()),
    }
    Ok(())
}

pub(super) fn indexed_value(
    function: VerifiedFunction<'_>,
    place: u32,
    index: u32,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let container = operations::place_type(function, place, layouts)?;
    let element = container
        .referenced_type()
        .and_then(|ty| layouts.type_by_id(ty))
        .ok_or_else(index_error)?;
    indexed_address(function, place, index, locals, layouts, body)?;
    memory::load_value(element, body);
    Ok(())
}

pub(super) fn projected_address(
    function: VerifiedFunction<'_>,
    parent: u32,
    index: u32,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let ty = function.backend_borrow_type(parent).ok_or_else(index_error)?;
    let container = context.layouts.type_by_id(ty).ok_or_else(index_error)?;
    body.instruction(&Instruction::LocalGet(locals.borrows + parent));
    memory::load_value(container, body);
    body.instruction(&Instruction::LocalSet(locals.heap));
    match container.category() {
        TypeCategory::FixedArray => {
            check_bound(index, container.array_length().ok_or_else(index_error)?, body)?;
            body.instruction(&Instruction::LocalGet(locals.heap));
            scaled_index(index, container.array_stride().ok_or_else(index_error)?, body)?;
        }
        TypeCategory::Vec => {
            body.instruction(&Instruction::LocalGet(index));
            body.instruction(&Instruction::LocalGet(locals.heap));
            body.instruction(&Instruction::I32Const(4));
            body.instruction(&Instruction::I32Add);
            body.instruction(&Instruction::I32Load(WORD));
            bounds_trap(body);
            body.instruction(&Instruction::LocalGet(locals.heap));
            body.instruction(&Instruction::I32Load(WORD));
            let element = container
                .referenced_type()
                .and_then(|child| context.layouts.type_by_id(child))
                .ok_or_else(index_error)?;
            scaled_index(index, element.size().max(1), body)?;
        }
        _ => return Err(index_error()),
    }
    Ok(())
}

pub(super) fn string_value(
    bytes: &[u8],
    temporary: u32,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let size = i32::try_from(12_usize.checked_add(bytes.len()).ok_or_else(index_error)?)
        .map_err(|_| index_error())?;
    body.instruction(&Instruction::I32Const(size));
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::LocalSet(temporary));
    write_header(temporary, bytes.len(), bytes.len(), body)?;
    for (index, byte) in bytes.iter().enumerate() {
        body.instruction(&Instruction::LocalGet(temporary));
        body.instruction(&Instruction::I32Const(
            i32::try_from(12 + index).map_err(|_| index_error())?,
        ));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::I32Const(i32::from(*byte)));
        body.instruction(&Instruction::I32Store8(BYTE));
    }
    body.instruction(&Instruction::LocalGet(temporary));
    Ok(())
}

pub(super) fn string_concat(
    function: VerifiedFunction<'_>,
    left: u32,
    right: u32,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    operations::place_value(function, left, locals, layouts, body)?;
    body.instruction(&Instruction::LocalSet(locals.heap));
    operations::place_value(function, right, locals, layouts, body)?;
    body.instruction(&Instruction::LocalSet(locals.heap + 1));
    load_length(locals.heap, locals.heap + 2, body);
    load_length(locals.heap + 1, locals.heap + 3, body);
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalTee(locals.heap + 2));
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    body.instruction(&Instruction::I32LtU);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::LocalSet(locals.scratch));
    write_header_from_local(locals.scratch, locals.heap + 2, body);
    copy_string_bytes(locals.scratch, locals.heap, 0, body);
    copy_string_bytes_after_left(
        locals.scratch,
        locals.heap + 1,
        locals.heap + 2,
        locals.heap + 3,
        body,
    );
    body.instruction(&Instruction::LocalGet(locals.scratch));
    Ok(())
}

pub(super) fn vec_value(
    values: &[u32],
    result_type: TypeId,
    temporary: u32,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let vector = context.layouts.type_by_id(result_type).ok_or_else(index_error)?;
    let element = vector
        .referenced_type()
        .and_then(|ty| context.layouts.type_by_id(ty))
        .ok_or_else(index_error)?;
    let stride = usize::try_from(element.size().max(1)).map_err(|_| index_error())?;
    let bytes =
        values.len().checked_mul(stride).and_then(|n| n.checked_add(12)).ok_or_else(index_error)?;
    body.instruction(&Instruction::I32Const(i32::try_from(bytes).map_err(|_| index_error())?));
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::LocalSet(temporary));
    write_header(temporary, values.len(), values.len(), body)?;
    for (index, value) in values.iter().enumerate() {
        body.instruction(&Instruction::LocalGet(temporary));
        body.instruction(&Instruction::I32Const(
            i32::try_from(12 + index * stride).map_err(|_| index_error())?,
        ));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalGet(*value));
        memory::store_value(element, body);
    }
    body.instruction(&Instruction::LocalGet(temporary));
    Ok(())
}

pub(super) fn vec_push(
    function: VerifiedFunction<'_>,
    place: u32,
    value: u32,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let vector = operations::place_type(function, place, context.layouts)?;
    let element = vector
        .referenced_type()
        .and_then(|ty| context.layouts.type_by_id(ty))
        .ok_or_else(index_error)?;
    let stride = i32::try_from(element.size().max(1)).map_err(|_| index_error())?;
    operations::place_value(function, place, locals, context.layouts, body)?;
    body.instruction(&Instruction::LocalSet(locals.heap));
    load_word(locals.heap, 4, locals.heap + 1, body);
    load_word(locals.heap, 8, locals.heap + 2, body);
    body.instruction(&Instruction::LocalGet(locals.heap + 1));
    body.instruction(&Instruction::I32Const(1_048_576));
    body.instruction(&Instruction::I32GeU);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(locals.heap + 1));
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32GeU);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    grow_vec(function, place, stride, locals, context, body)?;
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::LocalGet(locals.heap));
    body.instruction(&Instruction::LocalSet(locals.heap + 3));
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(locals.heap + 1));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalGet(value));
    memory::store_value(element, body);
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    body.instruction(&Instruction::I32Const(4));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalGet(locals.heap + 1));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Store(WORD));
    Ok(())
}

fn grow_vec(
    function: VerifiedFunction<'_>,
    place: u32,
    stride: i32,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::LocalSet(locals.heap + 2));
    body.instruction(&Instruction::Else);
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Shl);
    body.instruction(&Instruction::LocalSet(locals.heap + 2));
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32Const(1_048_576));
    body.instruction(&Instruction::I32GtU);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::LocalSet(locals.heap + 3));
    write_header_from_local(locals.heap + 3, locals.heap + 1, body);
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    body.instruction(&Instruction::I32Const(8));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalGet(locals.heap + 2));
    body.instruction(&Instruction::I32Store(WORD));
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(locals.heap));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(locals.heap + 1));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::Call(1));
    let layout = operations::place_type(function, place, context.layouts)?;
    operations::place_address(function, place, locals, context.layouts, body)?;
    body.instruction(&Instruction::LocalGet(locals.heap + 3));
    operations::store_place_value(function, place, layout, body)?;
    Ok(())
}

pub(super) fn shared_value(
    value: u32,
    result_type: TypeId,
    temporary: u32,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let shared = context.layouts.type_by_id(result_type).ok_or_else(index_error)?;
    let payload = shared
        .referenced_type()
        .and_then(|ty| context.layouts.type_by_id(ty))
        .ok_or_else(index_error)?;
    let offset = memory::align_up(8, payload.alignment()).ok_or_else(index_error)?;
    let size = offset.checked_add(payload.size()).ok_or_else(index_error)?.max(1);
    body.instruction(&Instruction::I32Const(i32::try_from(size).map_err(|_| index_error())?));
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::LocalSet(temporary));
    store_const(temporary, 0, 1, body);
    store_const(temporary, 4, 1, body);
    memory::address(temporary, offset, body)?;
    body.instruction(&Instruction::LocalGet(value));
    memory::store_value(payload, body);
    body.instruction(&Instruction::LocalGet(temporary));
    Ok(())
}

fn check_bound(
    index: u32,
    length: u64,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::LocalGet(index));
    body.instruction(&Instruction::I32Const(i32::try_from(length).map_err(|_| index_error())?));
    bounds_trap(body);
    Ok(())
}

fn bounds_trap(body: &mut Function) {
    body.instruction(&Instruction::I32GeU);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
}

fn scaled_index(
    index: u32,
    stride: u64,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::LocalGet(index));
    body.instruction(&Instruction::I32Const(i32::try_from(stride).map_err(|_| index_error())?));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Add);
    Ok(())
}

fn write_header(
    base: u32,
    length: usize,
    capacity: usize,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Store(WORD));
    store_const(base, 4, i32::try_from(length).map_err(|_| index_error())?, body);
    store_const(base, 8, i32::try_from(capacity).map_err(|_| index_error())?, body);
    Ok(())
}

fn write_header_from_local(base: u32, length: u32, body: &mut Function) {
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Store(WORD));
    for offset in [4, 8] {
        body.instruction(&Instruction::LocalGet(base));
        body.instruction(&Instruction::I32Const(offset));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalGet(length));
        body.instruction(&Instruction::I32Store(WORD));
    }
}

fn copy_string_bytes(destination: u32, source: u32, destination_offset: u32, body: &mut Function) {
    body.instruction(&Instruction::LocalGet(destination));
    body.instruction(&Instruction::I32Load(WORD));
    if destination_offset != 0 {
        body.instruction(&Instruction::LocalGet(destination_offset));
        body.instruction(&Instruction::I32Add);
    }
    body.instruction(&Instruction::LocalGet(source));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(source));
    body.instruction(&Instruction::I32Const(4));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::Call(1));
}

fn copy_string_bytes_after_left(
    destination: u32,
    source: u32,
    total: u32,
    right_length: u32,
    body: &mut Function,
) {
    body.instruction(&Instruction::LocalGet(destination));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(total));
    body.instruction(&Instruction::LocalGet(right_length));
    body.instruction(&Instruction::I32Sub);
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalGet(source));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(right_length));
    body.instruction(&Instruction::Call(1));
}

fn load_length(base: u32, target: u32, body: &mut Function) {
    load_word(base, 4, target, body);
}

fn load_word(base: u32, offset: i32, target: u32, body: &mut Function) {
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::I32Const(offset));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalSet(target));
}

fn store_const(base: u32, offset: i32, value: i32, body: &mut Function) {
    body.instruction(&Instruction::LocalGet(base));
    if offset != 0 {
        body.instruction(&Instruction::I32Const(offset));
        body.instruction(&Instruction::I32Add);
    }
    body.instruction(&Instruction::I32Const(value));
    body.instruction(&Instruction::I32Store(WORD));
}
