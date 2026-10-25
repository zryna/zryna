//! Derive and compare every cleanup claim before publishing retained operation indices.
use super::{Failure, budget, places, raw, reject, reserve, resource_failure, state, typed};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct OperationPlan {
    pub(super) destination: Option<u32>,
    pub(super) failure: Option<u32>,
    pub(super) prefix: Option<u32>,
    pub(super) drops: Option<DropRange>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct DropRange {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) whole: bool,
}
#[derive(Debug)]
pub(super) struct Plan {
    pub(super) drop_targets: Vec<raw::PlaceOrLoan>,
    pub(super) operations: Vec<OperationPlan>,
    used: Vec<Vec<bool>>,
}

pub(super) fn derive(
    program: &raw::Program,
    typed: &typed::Typed,
    spent: &mut usize,
) -> Result<Plan, Failure> {
    let mut work = *spent;
    budget::charge(
        &mut work,
        typed
            .sites
            .len()
            .checked_mul(4)
            .and_then(|n| n.checked_add(program.functions.len()))
            .ok_or_else(|| resource_failure("execution plan index overflow"))?,
    )?;
    let mut operations = reserve(typed.sites.len())?;
    operations.resize(typed.sites.len(), OperationPlan::default());
    let mut used = reserve(program.functions.len())?;
    for f in &program.functions {
        budget::charge(&mut work, f.cleanup_plans.len())?;
        let mut actions = 0usize;
        for (id, plan) in f.cleanup_plans.iter().enumerate() {
            if plan.id as usize != id || !places::within(f.original_span, plan.span) {
                return Err(reject("execution cleanup plan identity/span is foreign"));
            }
            actions = actions
                .checked_add(plan.actions.len())
                .ok_or_else(|| resource_failure("execution cleanup action census overflow"))?;
        }
        if actions > crate::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION {
            return Err(resource_failure("execution cleanup actions exceed inherited limit"));
        }
        budget::charge(&mut work, actions)?;
        let mut row = reserve(f.cleanup_plans.len())?;
        row.resize(f.cleanup_plans.len(), false);
        used.push(row);
    }
    let mut plan = Plan { operations, used, drop_targets: Vec::new() };
    for fi in 0..program.functions.len() {
        state::replay(program, typed, fi, &mut plan, &mut work)?;
    }
    if plan.used.iter().flatten().any(|used| !used) {
        return Err(reject("execution contains an unused unverified cleanup claim"));
    }
    *spent = work;
    Ok(plan)
}

impl Plan {
    pub(super) fn retain_drop(
        &mut self,
        index: usize,
        targets: &[raw::PlaceOrLoan],
        whole: bool,
        work: &mut usize,
    ) -> Result<(), Failure> {
        budget::charge(
            work,
            targets
                .len()
                .checked_mul(4)
                .and_then(|n| n.checked_add(8))
                .ok_or_else(|| resource_failure("retained drop target overflow"))?,
        )?;
        let start = self.drop_targets.len();
        let end = start
            .checked_add(targets.len())
            .ok_or_else(|| resource_failure("drop range overflow"))?;
        self.drop_targets.try_reserve(targets.len()).map_err(|_| Failure::AllocationFailure)?;
        self.drop_targets.extend_from_slice(targets);
        self.operations[index].drops = Some(DropRange { start, end, whole });
        Ok(())
    }

    pub(super) fn claim(
        &mut self,
        p: &raw::Program,
        fi: usize,
        id: u32,
        expected: &[raw::CleanupAction],
        span: Option<zryna_source::UntrustedSpan>,
        work: &mut usize,
    ) -> Result<(), Failure> {
        let claim = p.functions[fi]
            .cleanup_plans
            .get(id as usize)
            .ok_or_else(|| reject("execution uses a foreign cleanup plan"))?;
        if span.is_some_and(|span| claim.span != span) {
            return Err(reject("execution failure cleanup is bound to another source site"));
        }
        compare(claim, expected, work)?;
        self.used[fi][id as usize] = true;
        Ok(())
    }
}
fn compare(
    claim: &raw::CleanupPlan,
    expected: &[raw::CleanupAction],
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    budget::charge(
        &mut work,
        expected
            .len()
            .checked_add(1)
            .ok_or_else(|| resource_failure("execution cleanup comparison overflow"))?,
    )?;
    if claim.actions != expected {
        return Err(reject("execution cleanup differs from independent state replay"));
    }
    *spent = work;
    Ok(())
}
