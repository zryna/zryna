//! Pure certified inline initialization; heap/count leaves remain unpublished and zero.
use super::super::calls::Refs;
use super::util::{self as u, c32, c64, call, cmp, trap};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod bytes;
mod members;
pub(super) struct Input {
    pub source: Value,
    pub destination: Value,
    pub context: Value,
}
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    args: &[Value],
) -> Result<(), Diagnostic> {
    let input = Input { source: args[1], destination: args[2], context: args[3] };
    let plan = program.containers().ok_or_else(invariant)?;
    for index in 0..program.program().type_count() {
        let ty = u32::try_from(index).map_err(codegen)?;
        let Some(node) = plan.clone_node(ty) else {
            continue;
        };
        let active = builder.create_block();
        let next = builder.create_block();
        let key = c32(builder, ty);
        let matched = cmp(builder, IntCC::Equal, args[0], key);
        builder.ins().brif(matched, active, &[], next, &[]);
        builder.switch_to_block(active);
        bytes::emit(builder, None, input.destination, u::size(program, ty)?);
        match node.kind() {
            CloneNodeKind::Copy => {
                bytes::emit(builder, Some(input.source), input.destination, u::size(program, ty)?);
            }
            CloneNodeKind::Struct | CloneNodeKind::Enum => {
                members::emit(program, builder, refs, ty, &input)?;
            }
            CloneNodeKind::FixedArray { length } => {
                array(program, builder, refs, ty, &input, length)?;
            }
            CloneNodeKind::String
            | CloneNodeKind::VecEach
            | CloneNodeKind::SharedCount
            | CloneNodeKind::WeakCount => {}
        }
        u::done(builder);
        builder.switch_to_block(next);
    }
    builder.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
pub(super) fn child(
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    ty: u32,
    source: Value,
    destination: Value,
    context: Value,
) {
    let key = c32(builder, ty);
    let status = call(builder, refs.init, &[key, source, destination, context]);
    trap(builder, status);
}
fn array(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    ty: u32,
    input: &Input,
    length: u64,
) -> Result<(), Diagnostic> {
    if length == 0 {
        return Ok(());
    }
    let view = program.stored_type(ty).ok_or_else(invariant)?;
    let node = program.containers().and_then(|p| p.clone_node(ty)).ok_or_else(invariant)?;
    let child_ty = node.element().ok_or_else(invariant)?.ty();
    if view.array_length() != Some(length)
        || view.referenced_type().map(zryna_layout::generic_v1::TypeId::index) != Some(child_ty)
    {
        return Err(invariant());
    }
    let repeat = u::block(builder, &[types::I32]);
    let body = builder.create_block();
    let done = builder.create_block();
    let zero = c32(builder, 0);
    builder.ins().jump(repeat, &[zero.into()]);
    builder.switch_to_block(repeat);
    let index = builder.block_params(repeat)[0];
    let count = c32(builder, u32::try_from(length).map_err(codegen)?);
    let more = cmp(builder, IntCC::UnsignedLessThan, index, count);
    builder.ins().brif(more, body, &[], done, &[]);
    builder.switch_to_block(body);
    let wide = builder.ins().uextend(types::I64, index);
    let stride = c64(builder, view.array_stride().ok_or_else(invariant)?);
    let offset = builder.ins().imul(wide, stride);
    let source = builder.ins().iadd(input.source, offset);
    let destination = builder.ins().iadd(input.destination, offset);
    child(builder, refs, child_ty, source, destination, input.context);
    let one = c32(builder, 1);
    let next = builder.ins().iadd(index, one);
    builder.ins().jump(repeat, &[next.into()]);
    builder.switch_to_block(done);
    Ok(())
}
