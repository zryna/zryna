//! Pay bounded static path walks before instruction state can change.
use super::{Context, Failure, State, prepay, raw, resource_failure};
pub(in crate::generic_v1::execution_v1::state) fn instruction_credit(
    c: &Context<'_>,
    state: &State,
    operation: &raw::Operation,
    work: &mut usize,
) -> Result<(), Failure> {
    let scans = match operation {
        raw::Operation::ReadPlace { .. }
        | raw::Operation::DropPlace { .. }
        | raw::Operation::Clone { .. }
        | raw::Operation::IndexCopy { .. }
        | raw::Operation::EndLoan { .. }
        | raw::Operation::ReadLoan { .. }
        | raw::Operation::WeakDowngrade { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::ClosedEnumConstruct { payload: Some(_), .. }
        // Append also prepays its two target rechecks on its own ledger.
        | raw::Operation::VecPush { .. } => 1,
        raw::Operation::BeginLoan { .. } | raw::Operation::StringConcat { .. } => 2,
        raw::Operation::InitializePlace { .. }
        | raw::Operation::ReplacePlace { .. }
        | raw::Operation::ReplaceLoan { .. } => 3,
        raw::Operation::StructConstruct { fields, .. } => fields.len(),
        raw::Operation::FixedArrayConstruct { elements, .. }
        | raw::Operation::VecConstruct { elements, .. } => elements.len(),
        raw::Operation::ClosedGenericCall { arguments, .. }
        | raw::Operation::SourceCall { arguments, .. } => arguments
            .len()
            .checked_mul(arguments.len())
            .and_then(|n| n.checked_add(arguments.len()))
            .ok_or_else(|| resource_failure("loan call path comparison overflow"))?,
        _ => 0,
    };
    if scans != 0 {
        prepay(c, state, scans, work)?;
    }
    Ok(())
}

pub(in crate::generic_v1::execution_v1::state) fn terminator_credit(
    c: &Context<'_>,
    state: &State,
    t: &raw::Terminator,
    work: &mut usize,
) -> Result<(), Failure> {
    let add = |a: usize, b: usize| {
        a.checked_add(b).ok_or_else(|| resource_failure("terminator loan path debit overflow"))
    };
    let scans = match t {
        raw::Terminator::Return { value, .. } => usize::from(value.is_some()),
        raw::Terminator::Trap { .. } => 0,
        raw::Terminator::Jump(edge) => edge.arguments.len(),
        raw::Terminator::Branch { yes, no, .. } => add(yes.arguments.len(), no.arguments.len())?,
        raw::Terminator::ClosedEnumMatch {
            mode: raw::MatchMode::SharedLoan | raw::MatchMode::ExclusiveLoan,
            ..
        } => 1,
        raw::Terminator::ClosedEnumMatch { arms, .. } => {
            let mut scans = add(1, arms.len())?;
            for arm in arms {
                scans = add(scans, arm.edge.arguments.len())?;
            }
            scans
        }
        // The observation itself retains its existing separate one-scan debit.
        raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
            add(success.arguments.len(), expired.arguments.len())?
        }
    };
    if scans != 0 {
        prepay(c, state, scans, work)?;
    }
    Ok(())
}
