use super::super::super::{clone_frontier, failure};
use super::{
    Context, Function, Instruction, VerifiedType, WORD, address, cleanup_sequence, index_error,
    indexed, load_value, store_value,
};

pub(super) fn clone_vec(
    ty: VerifiedType<'_>,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let child_id = ty.referenced_type().ok_or_else(index_error)?;
    let child = context.layouts.type_by_id(child_id).ok_or_else(index_error)?;
    let stride = i32::try_from(child.size().max(1)).map_err(|_| index_error())?;
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::I32Const(4));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Load(WORD));
    body.instruction(&Instruction::LocalSet(1));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::Call(0));
    failure::helper_check(context, body);
    clone_frontier::acquired(context, body);
    body.instruction(&Instruction::LocalTee(2));
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Const(stride));
    body.instruction(&Instruction::I32Mul);
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::Call(1));
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::I32Const(12));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Store(WORD));
    // Only initialized elements were allocated and copied; spare source capacity is not owned.
    address(2, 8, body)?;
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::I32Store(WORD));
    if child.drop_kind() != 0 {
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::LocalSet(3));
        body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::LocalGet(3));
        body.instruction(&Instruction::LocalGet(1));
        body.instruction(&Instruction::I32GeU);
        body.instruction(&Instruction::BrIf(1));
        indexed(2, 3, stride, body);
        indexed(0, 3, stride, body);
        load_value(child, body);
        body.instruction(&Instruction::Call(context.clone_index(child_id)));
        cleanup_sequence(child_id, child, stride, true, context, body);
        store_value(child, body);
        body.instruction(&Instruction::LocalGet(3));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(3));
        body.instruction(&Instruction::Br(0));
        body.instruction(&Instruction::End);
        body.instruction(&Instruction::End);
    }
    body.instruction(&Instruction::LocalGet(2));
    Ok(())
}
