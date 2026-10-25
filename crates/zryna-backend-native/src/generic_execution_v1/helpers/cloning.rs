//! Complete child preparation or child-local rollback before a parent sees failure.
use super::{Helper, invariant, propagate};
use crate::generic_execution_v1::records;
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_ownership_runtime_abi::LogicalOperation as O;
pub(super) fn emit(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
) -> Result<(), Diagnostic> {
    let plan = h.p.containers().ok_or_else(invariant)?;
    let node = plan.clone_node(h.ty).ok_or_else(invariant)?;
    let view = h.p.stored_type(h.ty).ok_or_else(invariant)?;
    records::zero(b, destination, view.size())?;
    match node.kind() {
        CloneNodeKind::Copy => records::copy(b, source, destination, view.size())?,
        CloneNodeKind::String => {
            let status = h.runtime(b, O::StringClone, &[source, destination])?;
            propagate(b, status);
        }
        CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => {
            let control = b.ins().load(types::I64, MemFlagsData::new(), source, 0);
            let operation = if node.kind() == CloneNodeKind::SharedCount {
                O::StrongClone
            } else {
                O::WeakClone
            };
            let status = h.runtime(b, operation, &[control])?;
            propagate(b, status);
            b.ins().store(MemFlagsData::new(), control, destination, 0);
        }
        CloneNodeKind::Struct => structure(h, b, source, destination)?,
        CloneNodeKind::Enum => enumeration(h, b, source, destination)?,
        CloneNodeKind::FixedArray { length } => {
            if length != 0 {
                let length =
                    b.ins().iconst(types::I64, i64::try_from(length).map_err(super::codegen)?);
                super::sequence::clone(
                    h,
                    b,
                    super::sequence::Pair {
                        source,
                        destination,
                        length,
                        stride: view.array_stride().ok_or_else(invariant)?,
                        child: node.element().ok_or_else(invariant)?.ty(),
                        buffer: None,
                    },
                )?;
            }
        }
        CloneNodeKind::VecEach => vector(h, b, source, destination)?,
    }
    Ok(())
}
fn vector(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
) -> Result<(), Diagnostic> {
    let metadata = h.p.containers().and_then(|p| p.element(h.ty)).ok_or_else(invariant)?;
    let length = b.ins().load(types::I64, MemFlagsData::new(), source, 8);
    let element = b.ins().iconst(types::I32, i64::from(metadata.element().index()));
    let status = h.runtime(b, O::VecAllocate, &[element, length, destination])?;
    propagate(b, status);
    let source_data = b.ins().load(types::I64, MemFlagsData::new(), source, 0);
    let destination_data = b.ins().load(types::I64, MemFlagsData::new(), destination, 0);
    super::sequence::clone(
        h,
        b,
        super::sequence::Pair {
            source: source_data,
            destination: destination_data,
            length,
            stride: metadata.stride(),
            child: metadata.element().index(),
            buffer: Some(destination),
        },
    )?;
    b.ins().store(MemFlagsData::new(), length, destination, 8);
    Ok(())
}
pub(super) fn release_vec(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    record: Value,
) -> Result<(), Diagnostic> {
    let metadata = h.p.containers().and_then(|p| p.element(h.ty)).ok_or_else(invariant)?;
    let element = b.ins().iconst(types::I32, i64::from(metadata.element().index()));
    h.checked(b, O::VecReleaseStorage, &[element, record])
}
fn enumeration(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
) -> Result<(), Diagnostic> {
    let node = h.p.containers().and_then(|p| p.clone_node(h.ty)).ok_or_else(invariant)?;
    let view = h.p.stored_type(h.ty).ok_or_else(invariant)?;
    let tag = b.ins().load(types::I32, MemFlagsData::new(), source, 0);
    b.ins().store(MemFlagsData::new(), tag, destination, 0);
    let done = b.create_block();
    for (ordinal, child) in node.variants() {
        let active = b.create_block();
        let next = b.create_block();
        let selected = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
        b.ins().brif(selected, active, &[], next, &[]);
        b.switch_to_block(active);
        if let Some(child) = child {
            let offset = view.payload().ok_or_else(invariant)?.0;
            let s = records::add(b, source, offset)?;
            let d = records::add(b, destination, offset)?;
            let status = h.refs.clone_value(h.p, b, child.ty(), s, d, h.invocation)?;
            propagate(b, status);
        }
        b.ins().jump(done, &[]);
        b.switch_to_block(next);
    }
    b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    b.switch_to_block(done);
    Ok(())
}
fn structure(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
) -> Result<(), Diagnostic> {
    let node = h.p.containers().and_then(|p| p.clone_node(h.ty)).ok_or_else(invariant)?;
    let view = h.p.stored_type(h.ty).ok_or_else(invariant)?;
    let mut fields = super::reserve(node.fields().len())?;
    fields.extend(node.fields().zip(view.fields()));
    let progress = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().stack_store(types::I64, zero, progress, 0);
    let failed = b.create_block();
    b.append_block_param(failed, types::I32);
    for ((ordinal, child), (physical, ty, offset)) in &fields {
        if ordinal != physical || child.ty() != ty.index() {
            return Err(invariant());
        }
        let s = records::add(b, source, *offset)?;
        let d = records::add(b, destination, *offset)?;
        let status = h.refs.clone_value(h.p, b, child.ty(), s, d, h.invocation)?;
        let next = b.create_block();
        b.ins().brif(status, failed, &[status.into()], next, &[]);
        b.switch_to_block(next);
        let initialized = b.ins().iconst(types::I64, i64::from(*ordinal) + 1);
        b.ins().stack_store(types::I64, initialized, progress, 0);
    }
    let done = b.create_block();
    b.ins().jump(done, &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    let initialized = b.ins().stack_load(types::I64, types::I64, progress, 0);
    for ((ordinal, child), (_, _, offset)) in fields.into_iter().rev() {
        let active = b.create_block();
        let next = b.create_block();
        let ready = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, initialized, i64::from(ordinal));
        b.ins().brif(ready, active, &[], next, &[]);
        b.switch_to_block(active);
        let d = records::add(b, destination, offset)?;
        h.refs.drop_value(h.p, b, child.ty(), d, h.invocation)?;
        b.ins().jump(next, &[]);
        b.switch_to_block(next);
    }
    b.ins().return_(&[status]);
    b.switch_to_block(done);
    Ok(())
}
