use wasm_encoder::{Function, Instruction};
use zryna_ir::data_ownership_v1::{
    VerifiedBackendTerminator as T, VerifiedEdge, VerifiedFunction, VerifiedPlaceKind,
    VerifiedValueDefinition,
};

use super::{Context, Locals, index_error, operations};

#[allow(clippy::too_many_lines)]
pub(super) fn terminator(
    function: VerifiedFunction<'_>,
    terminator: zryna_ir::data_ownership_v1::VerifiedTerminator<'_>,
    parameters: &[Vec<VerifiedValueDefinition>],
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    match terminator.backend_terminator() {
        T::Return(value) => {
            operations::emit_terminator_drops(function, terminator, locals, context, body)?;
            body.instruction(&Instruction::LocalGet(value.index()));
            body.instruction(&Instruction::Return);
        }
        T::Jump(edge) => edge_jump(
            function,
            &edge,
            &parameters[edge.target().index() as usize],
            0,
            locals,
            context,
            body,
        )?,
        T::Branch { condition, when_true, when_false } => {
            body.instruction(&Instruction::LocalGet(condition.index()));
            body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            edge_jump(
                function,
                &when_true,
                &parameters[when_true.target().index() as usize],
                0,
                locals,
                context,
                body,
            )?;
            body.instruction(&Instruction::Else);
            edge_jump(
                function,
                &when_false,
                &parameters[when_false.target().index() as usize],
                0,
                locals,
                context,
                body,
            )?;
            body.instruction(&Instruction::End);
        }
        T::EnumMatch { place, arms } => {
            for arm in arms {
                operations::place_value(function, place.index(), locals, context.layouts, body)?;
                operations::load(body);
                body.instruction(&Instruction::I32Const(
                    i32::try_from(arm.variant()).map_err(|_| index_error())?,
                ));
                body.instruction(&Instruction::I32Eq);
                body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                let edge = arm.edge();
                edge_jump(
                    function,
                    edge,
                    &parameters[edge.target().index() as usize],
                    0,
                    locals,
                    context,
                    body,
                )?;
                body.instruction(&Instruction::End);
            }
            body.instruction(&Instruction::Unreachable);
        }
        T::WeakUpgrade { weak, success, expired } => {
            operations::place_value(function, weak.index(), locals, context.layouts, body)?;
            body.instruction(&Instruction::LocalTee(locals.scratch));
            operations::load(body);
            body.instruction(&Instruction::LocalTee(locals.heap));
            body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            super::observation::probe(4, context, body);
            body.instruction(&Instruction::GlobalGet(1));
            body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            operations::emit_terminator_drops(function, terminator, locals, context, body)?;
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::Return);
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::LocalGet(locals.heap));
            body.instruction(&Instruction::I32Const(-1));
            body.instruction(&Instruction::I32Eq);
            body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            operations::emit_terminator_drops(function, terminator, locals, context, body)?;
            super::failure::helper_trap(4, body);
            body.instruction(&Instruction::End);
            body.instruction(&Instruction::LocalGet(locals.scratch));
            body.instruction(&Instruction::LocalGet(locals.heap));
            body.instruction(&Instruction::I32Const(1));
            body.instruction(&Instruction::I32Add);
            operations::store(body);
            edge_jump(
                function,
                &success,
                &parameters[success.target().index() as usize],
                1,
                locals,
                context,
                body,
            )?;
            body.instruction(&Instruction::Else);
            edge_jump(
                function,
                &expired,
                &parameters[expired.target().index() as usize],
                0,
                locals,
                context,
                body,
            )?;
            body.instruction(&Instruction::End);
        }
        T::Trap(identity) => {
            operations::emit_terminator_drops(function, terminator, locals, context, body)?;
            let code = match identity {
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::BoundsV1 => 1,
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::AllocationV1 => 2,
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::CapacityV1 => 3,
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::RefcountV1 => 4,
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::Utf8V1 => 5,
            };
            body.instruction(&Instruction::I32Const(code));
            body.instruction(&Instruction::GlobalSet(1));
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::Return);
        }
    }
    Ok(())
}

fn edge_jump(
    function: VerifiedFunction<'_>,
    edge: &VerifiedEdge,
    parameters: &[VerifiedValueDefinition],
    synthesized: usize,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    for (index, argument) in edge.arguments().enumerate() {
        body.instruction(&Instruction::LocalGet(argument.index()));
        body.instruction(&Instruction::LocalSet(
            locals.scratch + u32::try_from(index + synthesized).map_err(|_| index_error())?,
        ));
    }
    for (index, parameter) in parameters.iter().enumerate() {
        body.instruction(&Instruction::LocalGet(
            locals.scratch + u32::try_from(index).map_err(|_| index_error())?,
        ));
        body.instruction(&Instruction::LocalSet(parameter.id().index()));
        if let Some(place) = function
            .places()
            .find(|place| place.kind() == VerifiedPlaceKind::Temporary(parameter.id()))
        {
            operations::place_address(function, place.id().index(), locals, context.layouts, body)?;
            body.instruction(&Instruction::LocalGet(parameter.id().index()));
            operations::store(body);
        }
    }
    body.instruction(&Instruction::I32Const(
        i32::try_from(edge.target().index()).map_err(|_| index_error())?,
    ));
    body.instruction(&Instruction::LocalSet(locals.state));
    body.instruction(&Instruction::Br(1));
    Ok(())
}
