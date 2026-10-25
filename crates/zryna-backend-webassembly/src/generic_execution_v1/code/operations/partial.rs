//! Same-operation reverse maximal initialized Place frontier, including an empty administrative drop.
use super::{Body, Diagnostic, Model, VerifiedOperation, invariant, raw, s};
use zryna_ir::generic_v1::execution_v1::VerifiedDropTarget;
pub(super) fn old_drop(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let (raw::Operation::ReplacePlace { place: selected, .. }
    | raw::Operation::DropPlace { place: selected }) = *op.kind()
    else {
        return Err(invariant());
    };
    let receipt = op.drop_targets().ok_or_else(invariant)?;
    if receipt.is_whole() {
        return s::drop(m, b, s::place(m, f, selected)?);
    }
    m.take(
        receipt.targets().len().checked_mul(4).ok_or_else(crate::generic_execution_v1::budget)?,
    )?;
    for target in receipt.targets() {
        let VerifiedDropTarget::Place(target) = target else {
            return Err(invariant());
        };
        if target.function() != f {
            return Err(invariant());
        }
        let slot = s::place(m, f, target.id())?;
        if slot.ty != raw::ValueType::Stored(target.ty()) {
            return Err(invariant());
        }
        s::drop(m, b, slot)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn genuine_partial_frontier_prepays_before_any_old_drop_instruction_and_retry_is_pristine() {
        crate::generic_execution_v1::tests::partial_program(
            "partial-root-replacement",
            "native",
            |program, _| {
                let plan = crate::generic_execution_v1::memory::verify(
                    crate::generic_execution_v1::memory::derive(program, false)
                        .expect("actual memory"),
                    program,
                )
                .expect("same-program plan");
                let m = Model::new(program, &plan).expect("actual retained view");
                let op = program
                    .operations()
                    .find(|o| o.drop_targets().is_some_and(|r| !r.is_whole()))
                    .expect("actual partial old-drop receipt");
                let count = op.drop_targets().expect("receipt").targets().len();
                assert_eq!(count, 1);
                let mut body = Body::new(&m, 4).expect("unpublished source body");
                let bytes = body.bytes.bytes.clone();
                let writer = m.writer.get();
                let work = m.work.get();
                m.work.set(crate::generic_execution_v1::MAX_BYTES - 4 * count + 1);
                assert!(old_drop(&m, &mut body, op).is_err());
                assert_eq!(body.bytes.bytes, bytes);
                assert_eq!(m.writer.get(), writer);
                assert_eq!(m.work.get(), crate::generic_execution_v1::MAX_BYTES - 4 * count + 1);
                m.work.set(work);
                old_drop(&m, &mut body, op).expect("pristine retry exact old-drop frontier");
                assert!(body.bytes.bytes.len() > bytes.len());
            },
        );
    }
    #[test]
    fn genuine_zero_frontier_administrative_drop_emits_no_old_handle_read_or_call() {
        crate::generic_execution_v1::tests::partial_program(
            "whole-vs-fields",
            "native",
            |program, _| {
                let plan = crate::generic_execution_v1::memory::verify(
                    crate::generic_execution_v1::memory::derive(program, false)
                        .expect("actual memory"),
                    program,
                )
                .expect("same-program plan");
                let m = Model::new(program, &plan).expect("actual retained view");
                let op = program
                    .operations()
                    .find(|o| {
                        o.drop_targets().is_some_and(|r| !r.is_whole() && r.targets().len() == 0)
                    })
                    .expect("actual empty administrative receipt");
                let mut body = Body::new(&m, 4).expect("unpublished source body");
                let bytes = body.bytes.bytes.clone();
                let writer = m.writer.get();
                m.work.set(crate::generic_execution_v1::MAX_BYTES);
                old_drop(&m, &mut body, op).expect("empty frontier requires no dynamic drop work");
                assert_eq!(body.bytes.bytes, bytes);
                assert_eq!(m.writer.get(), writer);
            },
        );
    }
}
