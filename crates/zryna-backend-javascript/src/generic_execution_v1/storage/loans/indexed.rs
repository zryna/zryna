//! Late address activation from exact opaque indexed views; no ownership inference.
use super::{
    Context, Diagnostic, VerifiedLoan, VerifiedLoanOrigin, VerifiedOperation, Writer, charge,
    emission_budget, internal, raw,
};
use crate::generic_execution_v1::context::Function;
use zryna_layout::TypeCategory;
pub(super) fn deferred(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    mut loan: VerifiedLoan<'_, '_>,
) -> Result<bool, Diagnostic> {
    let mut visited = 0usize;
    loop {
        charge(&c.work, 1)?;
        visited = visited.checked_add(1).ok_or_else(emission_budget)?;
        if visited > f.loans.len() || loan.function() != f.view.index() {
            return Err(internal());
        }
        if loan.indexed().is_some() {
            return Ok(true);
        }
        match loan.origin().ok_or_else(internal)? {
            VerifiedLoanOrigin::Parent(parent)
            | VerifiedLoanOrigin::StaticProjection { parent, .. } => loan = parent,
            VerifiedLoanOrigin::Formal(_) | VerifiedLoanOrigin::Place(_) => return Ok(false),
        }
    }
}
pub(in crate::generic_execution_v1) fn begin(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    id: u32,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    charge(&c.work, 4)?;
    if op.site().function != f.view.index()
        || !matches!(op.kind(), raw::Operation::BeginLoan {loan,..} if *loan==id)
    {
        return Err(internal());
    }
    let Some(selection) = op.indexed_loan() else {
        return activate(c, f, f.loan(id)?, w);
    };
    if selection.loan().id() != id
        || selection.loan().function() != f.view.index()
        || selection.index().ty() != raw::ValueType::Stored(1)
        || op.failure().is_none()
    {
        return Err(internal());
    }
    let (check, items) = match c.ty(selection.container_type())?.category() {
        TypeCategory::Vec => ("$vecIndex", ".$items"),
        TypeCategory::FixedArray => ("$index", ""),
        _ => return Err(internal()),
    };
    w.format(format_args!("$l[{id}]=(() => {{const $container="))?;
    super::super::either(c, f, selection.base_place(), selection.base_loan(), w)?;
    w.format(format_args!(",$position=$v[{}];{check}($container,$position);return $address($container{items},[$position]);}})()",selection.index().id()))
}
pub(in crate::generic_execution_v1) fn activate(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    loan: VerifiedLoan<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    charge(&c.work, 2)?;
    if loan.function() != f.view.index() || loan.indexed().is_some() {
        return Err(internal());
    }
    let id = loan.id();
    if !deferred(c, f, loan)? {
        return w.format(format_args!("$l[{id}]=$d[{id}]"));
    }
    match loan.origin().ok_or_else(internal)? {
        VerifiedLoanOrigin::Parent(parent) => {
            w.format(format_args!("$l[{id}]=$l[{}]", parent.id()))
        }
        VerifiedLoanOrigin::StaticProjection { parent, projection } => {
            w.format(format_args!("$l[{id}]=$project($l[{}],[", parent.id()))?;
            match projection {
                raw::StaticProjection::StructField(ordinal)
                | raw::StaticProjection::FixedArrayConstant(ordinal) => {
                    w.format(format_args!("{ordinal}"))?;
                }
                raw::StaticProjection::EnumPayload(_) => w.text("'payload'")?,
            }
            w.text("])")
        }
        VerifiedLoanOrigin::Formal(_) | VerifiedLoanOrigin::Place(_) => Err(internal()),
    }
}
