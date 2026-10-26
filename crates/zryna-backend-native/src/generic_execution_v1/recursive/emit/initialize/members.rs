//! Exact certified ordinal/physical-child pairing and actual enum tag selection.
use super::super::util::{self as u, c32, cmp, load, store};
use super::{Input, Refs, child};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    ty: u32,
    input: &Input,
) -> Result<(), Diagnostic> {
    let node = program.containers().and_then(|p| p.clone_node(ty)).ok_or_else(invariant)?;
    let view = program.stored_type(ty).ok_or_else(invariant)?;
    if node.kind() == CloneNodeKind::Struct {
        if node.fields().len() != view.fields().len() {
            return Err(invariant());
        }
        for ((ordinal, entry), (physical, field, offset)) in node.fields().zip(view.fields()) {
            if ordinal != physical || entry.ty() != field.index() {
                return Err(invariant());
            }
            let source = u::add(builder, input.source, offset);
            let destination = u::add(builder, input.destination, offset);
            child(builder, refs, entry.ty(), source, destination, input.context);
        }
        return Ok(());
    }
    if node.variants().len() != view.variants().len() {
        return Err(invariant());
    }
    let tag = load(builder, types::I32, input.source, 0);
    store(builder, input.destination, 0, tag);
    let done = builder.create_block();
    for ((ordinal, entry), (physical, payload)) in node.variants().zip(view.variants()) {
        if ordinal != physical
            || entry.map(zryna_ir::generic_v1::execution_v1::VerifiedCloneNode::ty)
                != payload.map(zryna_layout::generic_v1::TypeId::index)
        {
            return Err(invariant());
        }
        let active = builder.create_block();
        let next = builder.create_block();
        let key = c32(builder, ordinal);
        let selected = cmp(builder, IntCC::Equal, tag, key);
        builder.ins().brif(selected, active, &[], next, &[]);
        builder.switch_to_block(active);
        if let Some(entry) = entry {
            let offset = u32::try_from(view.payload().ok_or_else(invariant)?.0).map_err(codegen)?;
            let source = u::add(builder, input.source, u64::from(offset));
            let destination = u::add(builder, input.destination, u64::from(offset));
            child(builder, refs, entry.ty(), source, destination, input.context);
        }
        builder.ins().jump(done, &[]);
        builder.switch_to_block(next);
    }
    builder.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    builder.switch_to_block(done);
    Ok(())
}
