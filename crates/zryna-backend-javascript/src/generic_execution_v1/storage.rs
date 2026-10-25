//! Exact checked storage expressions and retained cleanup, never a new ownership analysis.
use super::{
    Context, Writer,
    context::{Function, charge},
    internal,
};
use zryna_diagnostics::Diagnostic;
pub(super) mod initialize;
pub(super) mod loans;
use zryna_ir::generic_v1::execution_v1::{
    OperationSite, VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedClone, VerifiedCloneSource,
    VerifiedDropTarget, VerifiedOperation, VerifiedPlace, raw,
};

pub(super) fn place(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    p: VerifiedPlace<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    place_at(c, f, p, w, 0)
}
fn place_at(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    p: VerifiedPlace<'_, '_>,
    w: &mut Writer,
    depth: usize,
) -> Result<(), Diagnostic> {
    charge(&c.work, 1)?;
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(super::emission_budget());
    }
    match p.kind() {
        raw::PlaceKind::Parameter(ordinal) => {
            let Some(zryna_ir::generic_v1::execution_v1::VerifiedParameter::Value(value)) =
                f.parameters.get(ordinal as usize)
            else {
                return Err(internal());
            };
            w.format(format_args!("$v[{}]", value.id()))
        }
        raw::PlaceKind::Local(_) => w.format(format_args!("$p[{}]", p.id())),
        raw::PlaceKind::Temporary(value) => w.format(format_args!("$v[{value}]")),
        raw::PlaceKind::StructField { base, ordinal }
        | raw::PlaceKind::FixedArrayConstant { base, index: ordinal } => {
            place_at(c, f, f.place(base)?, w, depth + 1)?;
            w.format(format_args!("[{ordinal}]"))
        }
        raw::PlaceKind::EnumPayload { base, .. } => {
            place_at(c, f, f.place(base)?, w, depth + 1)?;
            w.text(".payload")
        }
    }
}
pub(super) fn source(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    source: raw::PlaceOrLoan,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    match source {
        raw::PlaceOrLoan::Place(id) => place(c, f, f.place(id)?, w),
        raw::PlaceOrLoan::Loan(id) => loans::read(id, w),
    }
}
pub(super) fn clone_source(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    clone: VerifiedClone<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    match clone.source() {
        VerifiedCloneSource::Place(p) => place(c, f, p, w),
        VerifiedCloneSource::Loan(loan) => loans::read(loan.id(), w),
    }
}
pub(super) fn drop_place(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    p: VerifiedPlace<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    if c.ty(p.ty())?.drop_kind() != 0 {
        w.format(format_args!("$drop{}(", p.ty()))?;
        place(c, f, p, w)?;
        super::runtime::accounting::argument(c, w, true)?;
        w.text(");\n")?;
    }
    place(c, f, p, w)?;
    w.text("=undefined;\n")
}
pub(super) fn cleanup(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    plan: VerifiedCleanupPlan<'_, '_>,
    rolled_back: Option<OperationSite>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    for action in plan.actions() {
        charge(&c.work, 1)?;
        match action {
            VerifiedCleanupAction::DropPlace(p) => drop_place(c, f, p, w)?,
            VerifiedCleanupAction::EndLoan(loan) => {
                w.format(format_args!("$l[{}]=undefined;\n", loan.id()))?;
            }
            VerifiedCleanupAction::DropInitializedPrefix { preparation, .. } => {
                // The checked Clone helper already unwound its actual successful children.
                // The operation-bound token authorizes no caller-supplied progress count.
                if rolled_back != Some(preparation.site()) {
                    return Err(internal());
                }
            }
        }
    }
    Ok(())
}
pub(super) fn check_prefix(
    clone: VerifiedClone<'_, '_>,
    site: OperationSite,
) -> Result<(), Diagnostic> {
    if !clone.frontier().may_fail() {
        return if clone.prefix_failure().is_none() && clone.outer_failure().is_none() {
            Ok(())
        } else {
            Err(internal())
        };
    }
    let destination = clone.destination_place().ok_or_else(internal)?;
    let plan = clone.prefix_failure().ok_or_else(internal)?;
    if clone.outer_failure().is_none() {
        return Err(internal());
    }
    match plan.actions().next() {
        Some(VerifiedCleanupAction::DropInitializedPrefix { destination: actual, preparation })
            if actual.id() == destination.id()
                && actual.function() == destination.function()
                && preparation.site() == site =>
        {
            Ok(())
        }
        _ => Err(internal()),
    }
}

pub(super) fn drop_old(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    original: VerifiedPlace<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    let proof = op.drop_targets().ok_or_else(internal)?;
    if proof.is_whole() {
        return drop_place(c, f, original, w);
    }
    for target in proof.targets() {
        match target {
            VerifiedDropTarget::Place(p) => drop_place(c, f, p, w)?,
            VerifiedDropTarget::Loan(_) => return Err(internal()),
        }
    }
    place(c, f, original, w)?;
    w.text("=undefined;\n")
}

pub(super) fn either(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    place_view: Option<VerifiedPlace<'_, '_>>,
    loan_view: Option<zryna_ir::generic_v1::execution_v1::VerifiedLoan<'_, '_>>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    match (place_view, loan_view) {
        (Some(place_view), None) => place(c, f, place_view, w),
        (None, Some(loan_view)) => loans::read(loan_view.id(), w),
        _ => Err(internal()),
    }
}
