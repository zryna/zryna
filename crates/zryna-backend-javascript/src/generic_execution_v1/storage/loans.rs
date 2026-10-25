//! Checked static addresses preserve caller storage without callbacks or fabricated local roots.
use super::super::{
    Context, Writer,
    context::{Function, charge},
    emission_budget, internal,
};
use zryna_diagnostics::Diagnostic;
mod indexed;
pub(in crate::generic_execution_v1) use indexed::{activate, begin};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedDropTarget, VerifiedLoan, VerifiedLoanOrigin, VerifiedOperation, raw,
};

#[derive(Clone, Copy)]
pub(in crate::generic_execution_v1) enum Base {
    Place(u32),
    Formal(u32),
}
pub(in crate::generic_execution_v1) struct Address {
    pub(in crate::generic_execution_v1) base: Base,
    pub(in crate::generic_execution_v1) path: Vec<raw::StaticProjection>,
    pub(in crate::generic_execution_v1) indexed: bool,
}
fn append(
    context: &Context<'_, '_>,
    path: &mut Vec<raw::StaticProjection>,
    key: raw::StaticProjection,
) -> Result<(), Diagnostic> {
    charge(
        &context.work,
        // Exact growth may copy every retained prefix slot before appending one.
        path.len()
            .checked_add(1)
            .and_then(|n| n.checked_mul(size_of::<raw::StaticProjection>()))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(emission_budget)?,
    )?;
    path.try_reserve_exact(1).map_err(|_| emission_budget())?;
    path.push(key);
    Ok(())
}
pub(in crate::generic_execution_v1) fn resolve(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    mut loan: VerifiedLoan<'_, '_>,
) -> Result<Address, Diagnostic> {
    let mut path = Vec::new();
    let mut visited = 0usize;
    let mut indexed = false;
    let base = loop {
        charge(&context.work, 1)?;
        visited = visited.checked_add(1).ok_or_else(emission_budget)?;
        if visited > function.loans.len() {
            return Err(internal());
        }
        if let Some(selection) = loan.indexed() {
            indexed = true;
            if let Some(parent) = selection.base_loan() {
                loan = parent;
                continue;
            }
            let place = selection.base_place().ok_or_else(internal)?;
            return finish_place(context, function, place.id(), path, indexed);
        }
        match loan.origin().ok_or_else(internal)? {
            VerifiedLoanOrigin::Formal(ordinal) => break Base::Formal(ordinal),
            VerifiedLoanOrigin::Parent(parent) => loan = parent,
            VerifiedLoanOrigin::StaticProjection { parent, projection } => {
                append(context, &mut path, projection)?;
                loan = parent;
            }
            VerifiedLoanOrigin::Place(place) => {
                return finish_place(context, function, place.id(), path, indexed);
            }
        }
    };
    charge(&context.work, path.len())?;
    path.reverse();
    Ok(Address { base, path, indexed })
}
fn finish_place(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    mut id: u32,
    mut path: Vec<raw::StaticProjection>,
    indexed: bool,
) -> Result<Address, Diagnostic> {
    let mut count = 0usize;
    loop {
        charge(&context.work, 1)?;
        count = count.checked_add(1).ok_or_else(emission_budget)?;
        if count > function.places.len() {
            return Err(internal());
        }
        let (parent, key) = match function.place(id)?.kind() {
            raw::PlaceKind::StructField { base, ordinal } => {
                (base, raw::StaticProjection::StructField(ordinal))
            }
            raw::PlaceKind::FixedArrayConstant { base, index } => {
                (base, raw::StaticProjection::FixedArrayConstant(index))
            }
            raw::PlaceKind::EnumPayload { base, variant } => {
                (base, raw::StaticProjection::EnumPayload(variant))
            }
            _ => break,
        };
        append(context, &mut path, key)?;
        id = parent;
    }
    charge(&context.work, path.len())?;
    path.reverse();
    Ok(Address { base: Base::Place(id), path, indexed })
}
pub(in crate::generic_execution_v1) fn prologue(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    charge(&context.work, function.loans.len())?;
    for loan in &function.loans {
        if indexed::deferred(context, function, *loan)? {
            continue;
        }
        let address = resolve(context, function, *loan)?;
        writer.format(format_args!("$d[{}]=", loan.id()))?;
        match address.base {
            Base::Formal(ordinal) if address.path.is_empty() => {
                writer.format(format_args!("$a{ordinal};\n"))?;
                continue;
            }
            Base::Formal(ordinal) => writer.format(format_args!("$project($a{ordinal},["))?,
            Base::Place(id) => match function.place(id)?.kind() {
                raw::PlaceKind::Temporary(value) => {
                    writer.format(format_args!("$address($v,[{value}"))?;
                }
                raw::PlaceKind::Local(_) => writer.format(format_args!("$address($p,[{id}"))?,
                raw::PlaceKind::Parameter(ordinal) => {
                    let Some(zryna_ir::generic_v1::execution_v1::VerifiedParameter::Value(value)) =
                        function.parameters.get(ordinal as usize)
                    else {
                        return Err(internal());
                    };
                    writer.format(format_args!("$address($v,[{}", value.id()))?;
                }
                _ => return Err(internal()),
            },
        }
        charge(&context.work, address.path.len())?;
        for (index, key) in address.path.iter().enumerate() {
            if index != 0 || matches!(address.base, Base::Place(_)) {
                writer.text(",")?;
            }
            match key {
                raw::StaticProjection::StructField(ordinal)
                | raw::StaticProjection::FixedArrayConstant(ordinal) => {
                    writer.format(format_args!("{ordinal}"))?;
                }
                raw::StaticProjection::EnumPayload(_) => writer.text("'payload'")?,
            }
        }
        writer.text("]);\n")?;
    }
    Ok(())
}
pub(in crate::generic_execution_v1) fn read(
    id: u32,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    writer.format(format_args!("$loanGet($l[{id}])"))
}
pub(in crate::generic_execution_v1) fn replace(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    operation: VerifiedOperation<'_, '_>,
    id: u32,
    value: u32,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    let loan = function.loan(id)?;
    let proof = operation.drop_targets().ok_or_else(internal)?;
    let mut targets = proof.targets();
    charge(&context.work, targets.len())?;
    if !proof.is_whole()
        || targets.len() != 1
        || !matches!(targets.next(), Some(VerifiedDropTarget::Loan(target)) if target.id() == loan.id() && target.referent() == loan.referent())
    {
        return Err(internal());
    }
    writer.text("(() => {const $replacement=")?;
    super::super::shapes::records::value(context, function, value, writer)?;
    writer.text(";\n")?;
    if context.ty(loan.referent())?.drop_kind() != 0 {
        writer.format(format_args!("$drop{}(", loan.referent()))?;
        read(id, writer)?;
        super::super::runtime::accounting::argument(context, writer, true)?;
        writer.text(");\n")?;
    }
    writer.format(format_args!("$loanSet($l[{id}],$replacement); }})()"))
}

#[cfg(test)]
mod prepayment_tests {
    use super::*;
    use crate::generic_execution_v1::tests::with_program;
    #[test]
    fn projection_growth_pays_retained_prefix_before_reserve() {
        with_program("scalar-i32-clone", "native", |program| {
            let context = Context::new(program).expect("genuine new execution context");
            let make = || vec![raw::StaticProjection::StructField(0); 32];
            let mut measured = make();
            let cost = (measured.len() + 1) * size_of::<raw::StaticProjection>() + 1;
            let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
            context.work.set(limit - cost);
            append(&context, &mut measured, raw::StaticProjection::StructField(1))
                .expect("exact full-prefix growth credit");
            assert_eq!(context.work.get(), limit);
            assert_eq!(measured.len(), 33);
            for start in [limit - cost + 1, usize::MAX] {
                let mut path = make();
                let capacity = path.capacity();
                context.work.set(start);
                let error = append(&context, &mut path, raw::StaticProjection::StructField(1))
                    .expect_err("insufficient prefix-copy credit rejects before reserve");
                assert_eq!(error.code(), "ZRYNA-J2003");
                assert_eq!(context.work.get(), start);
                assert_eq!(path.len(), 32);
                assert_eq!(path.capacity(), capacity);
                assert!(path.iter().all(|key| *key == raw::StaticProjection::StructField(0)));
            }
            context.work.set(0);
            let mut retry = make();
            append(&context, &mut retry, raw::StaticProjection::StructField(1))
                .expect("pristine retry");
            assert_eq!(context.work.get(), cost);
        });
    }
}
