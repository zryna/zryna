//! Cleanup loan uses are authorized by the exact instruction, never its plan's claimed span.
use super::{Failure, at, budget, raw, reject};
pub(super) fn check(
    f: &raw::Function,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<(), Failure> {
    let (first, second) = match &i.operation {
        raw::Operation::Clone { outer_failure, prefix_failure, .. } => {
            (*outer_failure, *prefix_failure)
        }
        raw::Operation::BeginLoan { failure, .. }
        | raw::Operation::ClosedGenericCall { failure, .. }
        | raw::Operation::SourceCall { failure, .. } => (*failure, None),
        raw::Operation::StringFromUtf8 { failure, .. }
        | raw::Operation::IndexCopy { failure, .. }
        | raw::Operation::VecConstruct { failure, .. }
        | raw::Operation::SharedConstruct { failure, .. }
        | raw::Operation::WeakDowngrade { failure, .. }
        | raw::Operation::VecPush { failure, .. }
        | raw::Operation::StringConcat { failure, .. } => (Some(*failure), None),
        _ => (None, None),
    };
    for id in [first, second].into_iter().flatten() {
        budget::charge(work, 1)?;
        let plan = f
            .cleanup_plans
            .get(id as usize)
            .ok_or_else(|| reject("instruction failure cleanup is foreign"))?;
        budget::charge(work, plan.actions.len())?;
        for action in &plan.actions {
            if let raw::CleanupAction::EndLoan(id) = action {
                at(f, *id, i.site_span, work)?;
            }
        }
    }
    Ok(())
}
