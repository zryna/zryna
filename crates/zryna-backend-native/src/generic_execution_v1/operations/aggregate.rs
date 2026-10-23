//! Inline aggregate construction in pre-zeroed physical destination storage.
use super::super::{body::Frame, invariant, records};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, raw};
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    result: Option<Value>,
) -> Result<(), Diagnostic> {
    match op.kind() {
        raw::Operation::StructConstruct { ty, fields } => {
            let view = c.p.stored_type(*ty).ok_or_else(invariant)?;
            let destination = result.ok_or_else(invariant)?;
            for ((_, child, offset), id) in view.fields().zip(fields) {
                let target = records::add(b, destination, offset)?;
                let source = c.value(b, *id)?;
                records::copy(
                    b,
                    source,
                    target,
                    c.p.stored_type(child.index()).ok_or_else(invariant)?.size(),
                )?;
            }
        }
        raw::Operation::ClosedEnumConstruct { ty, ordinal, payload } => {
            let view = c.p.stored_type(*ty).ok_or_else(invariant)?;
            let destination = result.ok_or_else(invariant)?;
            let tag = b.ins().iconst(types::I32, i64::from(*ordinal));
            b.ins().store(MemFlagsData::new(), tag, destination, 0);
            if let Some(id) = payload {
                let source = c.value(b, *id)?;
                let target = records::add(b, destination, view.payload().ok_or_else(invariant)?.0)?;
                records::copy(
                    b,
                    source,
                    target,
                    c.size(c.f.value(*id).ok_or_else(invariant)?.ty())?,
                )?;
            }
        }
        raw::Operation::FixedArrayConstruct { ty, elements } => {
            let view = c.p.stored_type(*ty).ok_or_else(invariant)?;
            let destination = result.ok_or_else(invariant)?;
            for (index, id) in elements.iter().enumerate() {
                let offset = (index as u64)
                    .checked_mul(view.array_stride().ok_or_else(invariant)?)
                    .ok_or_else(invariant)?;
                let target = records::add(b, destination, offset)?;
                let source = c.value(b, *id)?;
                records::copy(
                    b,
                    source,
                    target,
                    c.size(c.f.value(*id).ok_or_else(invariant)?.ty())?,
                )?;
            }
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
