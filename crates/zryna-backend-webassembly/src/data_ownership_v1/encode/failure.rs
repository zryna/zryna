use super::{Context, Locals, operations};
use wasm_encoder::{BlockType, Function, Instruction};
use zryna_ir::data_ownership_v1::{VerifiedFunction, VerifiedInstruction};

pub(super) fn propagate(
    function: VerifiedFunction<'_>,
    instruction: VerifiedInstruction<'_>,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    body.instruction(&Instruction::GlobalGet(1));
    body.instruction(&Instruction::If(BlockType::Empty));
    for action in instruction.derived_drop_actions() {
        let root = action.root().index();
        let ty = operations::place_type(function, root, context.layouts)?;
        operations::place_value(function, root, locals, context.layouts, body)?;
        body.instruction(&Instruction::Call(context.drop_index(ty.id())));
    }
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::Return);
    body.instruction(&Instruction::End);
    Ok(())
}
