use wasm_encoder::{Function, Instruction, MemArg, ValType};
use zryna_layout::{TypeCategory, VerifiedType};

use super::{Context, index_error};

mod clone;
pub(super) use clone::clone_helper;

const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };

pub(super) fn load_value(ty: VerifiedType<'_>, body: &mut Function) {
    match ty.category() {
        TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray => {}
        TypeCategory::Bool => {
            body.instruction(&Instruction::I32Load8U(BYTE));
        }
        _ => {
            body.instruction(&Instruction::I32Load(WORD));
        }
    }
}

pub(super) fn store_value(ty: VerifiedType<'_>, body: &mut Function) {
    match ty.category() {
        TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray => {
            body.instruction(&Instruction::I32Const(i32::try_from(ty.size()).unwrap_or(i32::MAX)));
            body.instruction(&Instruction::Call(1));
        }
        TypeCategory::Bool => {
            body.instruction(&Instruction::I32Store8(BYTE));
        }
        _ => {
            body.instruction(&Instruction::I32Store(WORD));
        }
    }
}

pub(super) fn drop_helper(
    ty: VerifiedType<'_>,
    context: &Context<'_>,
) -> Result<Function, zryna_diagnostics::Diagnostic> {
    let mut body = Function::new([(3, ValType::I32)]);
    if let Some(kind) = super::observation::value_kind(ty.category()) {
        super::observation::record(0x10000000 + kind, context, &mut body);
    }
    match ty.category() {
        TypeCategory::Struct => {
            for field in ty.fields().iter().rev() {
                drop_child(field.ty(), field.offset(), context, &mut body)?;
            }
        }
        TypeCategory::FixedArray => {
            let child_id = ty.referenced_type().ok_or_else(index_error)?;
            let child = context.layouts.type_by_id(child_id).ok_or_else(index_error)?;
            if child.drop_kind() != 0 {
                let stride = i32::try_from(ty.array_stride().ok_or_else(index_error)?)
                    .map_err(|_| index_error())?;
                let length = i32::try_from(ty.array_length().ok_or_else(index_error)?)
                    .map_err(|_| index_error())?;
                drop_elements(child_id, child, stride, length, context, &mut body);
            }
        }
        TypeCategory::Enum => {
            let offset = ty.enum_payload_layout().ok_or_else(index_error)?.0;
            for variant in ty.variants() {
                let Some(payload) = variant.payload() else { continue };
                body.instruction(&Instruction::LocalGet(0));
                body.instruction(&Instruction::I32Load(WORD));
                body.instruction(&Instruction::I32Const(
                    i32::try_from(variant.ordinal()).map_err(|_| index_error())?,
                ));
                body.instruction(&Instruction::I32Eq);
                body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                drop_child(payload, offset, context, &mut body)?;
                body.instruction(&Instruction::End);
            }
        }
        TypeCategory::Vec => drop_vec(ty, context, &mut body)?,
        TypeCategory::Shared => drop_shared(ty, context, &mut body)?,
        TypeCategory::Weak => decrement(4, context, &mut body),
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
    }
    body.instruction(&Instruction::End);
    Ok(body)
}

fn drop_child(
    child: zryna_layout::TypeId,
    offset: u64,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let ty = context.layouts.type_by_id(child).ok_or_else(index_error)?;
    if ty.drop_kind() == 0 {
        return Ok(());
    }
    address(0, offset, body)?;
    load_value(ty, body);
    body.instruction(&Instruction::Call(context.drop_index(child)));
    Ok(())
}

fn drop_vec(
    ty: VerifiedType<'_>,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let child_id = ty.referenced_type().ok_or_else(index_error)?;
    let child = context.layouts.type_by_id(child_id).ok_or_else(index_error)?;
    if child.drop_kind() == 0 {
        return Ok(());
    }
    let stride = i32::try_from(child.size().max(1)).map_err(|_| index_error())?;
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::I32Const(4));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalSet(1));
    body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::BrIf(1));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Sub);
    body.instruction(&Instruction::LocalTee(1));
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Add);
    load_value(child, body);
    body.instruction(&Instruction::Call(context.drop_index(child_id)));
    body.instruction(&Instruction::Br(0));
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::End);
    Ok(())
}

fn drop_elements(
    child_id: zryna_layout::TypeId,
    child: VerifiedType<'_>,
    stride: i32,
    length: i32,
    context: &Context<'_>,
    body: &mut Function,
) {
    body.instruction(&Instruction::I32Const(length));
    body.instruction(&Instruction::LocalSet(3));
    body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::BrIf(1));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Sub);
    body.instruction(&Instruction::LocalTee(3));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::I32Add);
    load_value(child, body);
    body.instruction(&Instruction::Call(context.drop_index(child_id)));
    body.instruction(&Instruction::Br(0));
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::End);
}

fn drop_shared(
    ty: VerifiedType<'_>,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalTee(1));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Sub);
    body.instruction(&Instruction::LocalTee(1));
    body.instruction(&Instruction::I32Store(WORD));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    let payload = ty.referenced_type().ok_or_else(index_error)?;
    let payload_ty = context.layouts.type_by_id(payload).ok_or_else(index_error)?;
    let offset = align_up(8, payload_ty.alignment()).ok_or_else(index_error)?;
    drop_child(payload, offset, context, body)?;
    super::observation::record(0x10000006, context, body);
    decrement(4, context, body);
    body.instruction(&Instruction::End);
    Ok(())
}

fn decrement(offset: u64, context: &Context<'_>, body: &mut Function) {
    address(0, offset, body).expect("constant offset");
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalTee(1));
    body.instruction(&Instruction::I32Eqz);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    address(0, offset, body).expect("constant offset");
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Sub);
    body.instruction(&Instruction::I32Store(WORD));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Eq);
    body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    super::observation::record(0x10000007, context, body);
    body.instruction(&Instruction::End);
}

pub(super) fn address(
    local: u32,
    offset: u64,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::LocalGet(local));
    if offset != 0 {
        body.instruction(&Instruction::I32Const(i32::try_from(offset).map_err(|_| index_error())?));
        body.instruction(&Instruction::I32Add);
    }
    Ok(())
}

fn indexed(base: u32, index: u32, stride: i32, body: &mut Function) {
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalGet(index));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Add);
}

fn raw_indexed(base: u32, index: u32, stride: i32, body: &mut Function) {
    body.instruction(&Instruction::LocalGet(base));
    body.instruction(&Instruction::LocalGet(index));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Add);
}

pub(super) fn align_up(value: u64, alignment: u64) -> Option<u64> {
    let padding = alignment.checked_sub(1)?;
    value.checked_add(padding).map(|sum| sum / alignment * alignment)
}
