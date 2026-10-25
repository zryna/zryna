use super::{DeferredCall, Failure, TypeWorld, charge, exhausted, raw, reject, reserve};
pub(super) fn calls(
    program: &mut raw::Program,
    all: &[Vec<DeferredCall>],
    spent: &mut usize,
) -> Result<(), Failure> {
    let n = program.functions.len();
    charge(spent, n * 6)?;
    let mut reverse = reserve(n)?;
    reverse.resize_with(n, Vec::new);
    let mut pending = reserve(n)?;
    pending.resize(n, 0usize);
    let mut failure = reserve(n)?;
    failure.resize(n, false);
    let mut depth = reserve(n)?;
    depth.resize(n, 1usize);
    let mut queue = reserve(n)?;
    let mut reachable = reserve(n)?;
    reachable.resize(n, false);
    for (fi, list) in all.iter().enumerate() {
        charge(spent, list.len())?;
        pending[fi] = list.len();
        for call in list {
            charge(spent, 1)?;
            reverse[call.target].try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
            reverse[call.target].push(fi);
        }
    }
    charge(spent, n)?;
    for (fi, f) in program.functions.iter().enumerate() {
        charge(spent, f.blocks.len())?;
        for b in &f.blocks {
            charge(spent, b.instructions.len())?;
            failure[fi] |= matches!(b.terminator, raw::Terminator::WeakUpgradeBranch { .. });
            failure[fi] |= b.instructions.iter().any(|i| {
                matches!(
                    i.operation,
                    raw::Operation::StringFromUtf8 { .. }
                        | raw::Operation::StringConcat { .. }
                        | raw::Operation::IndexCopy { .. }
                        | raw::Operation::BeginLoan { failure: Some(_), .. }
                        | raw::Operation::VecConstruct { .. }
                        | raw::Operation::SharedConstruct { .. }
                        | raw::Operation::WeakDowngrade { .. }
                        | raw::Operation::VecPush { .. }
                        | raw::Operation::Clone { outer_failure: Some(_), .. }
                )
            });
        }
        if pending[fi] == 0 {
            queue.push(fi);
        }
    }
    let mut at = 0;
    while at < queue.len() {
        charge(spent, 1)?;
        let child = queue[at];
        at += 1;
        charge(spent, reverse[child].len())?;
        for parent in &reverse[child] {
            failure[*parent] |= failure[child];
            depth[*parent] = depth[*parent].max(depth[child] + 1);
            if depth[*parent] > 128 {
                return Err(exhausted("source call depth exceeds inherited limit"));
            }
            pending[*parent] -= 1;
            if pending[*parent] == 0 {
                queue.push(*parent);
            }
        }
    }
    if queue.len() != n {
        return Err(reject(
            "recursive original source calls are outside the accepted bounded call graph",
        ));
    }
    queue.clear();
    charge(spent, n)?;
    for (fi, f) in program.functions.iter().enumerate() {
        if f.key.first() == Some(&0x41) {
            reachable[fi] = true;
            queue.push(fi);
        }
    }
    at = 0;
    while at < queue.len() {
        charge(spent, 1)?;
        let fi = queue[at];
        at += 1;
        charge(spent, all[fi].len())?;
        for c in &all[fi] {
            if !reachable[c.target] {
                reachable[c.target] = true;
                queue.push(c.target);
            }
        }
    }
    charge(spent, n)?;
    if reachable.iter().any(|b| !*b) {
        return Err(reject("supplied closed generic instance is not demanded by original roots"));
    }
    apply_failures(program, all, &failure, spent)
}
fn apply_failures(
    program: &mut raw::Program,
    all: &[Vec<DeferredCall>],
    failure: &[bool],
    spent: &mut usize,
) -> Result<(), Failure> {
    for (fi, list) in all.iter().enumerate() {
        charge(spent, list.len())?;
        for c in list {
            if failure[c.target] {
                charge(spent, c.actions.len() + 1)?;
                let mut actions = reserve(c.actions.len())?;
                actions.extend(c.actions.iter().cloned());
                let f = &mut program.functions[fi];
                let id =
                    u32::try_from(f.cleanup_plans.len()).map_err(|_| Failure::InternalFailure)?;
                f.cleanup_plans.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                f.cleanup_plans.push(raw::CleanupPlan { id, span: c.span, actions });
                let op = &mut f.blocks[c.block].instructions[c.position].operation;
                match op {
                    raw::Operation::SourceCall { failure, .. }
                    | raw::Operation::ClosedGenericCall { failure, .. } => *failure = Some(id),
                    _ => return Err(Failure::InternalFailure),
                }
            }
        }
    }
    Ok(())
}
pub(super) fn demand(
    program: &raw::Program,
    world: &TypeWorld,
    used: &mut [bool],
    spent: &mut usize,
) -> Result<(), Failure> {
    charge(spent, used.len())?;
    let mut queue = reserve(used.len())?;
    for (i, b) in used.iter().enumerate() {
        if *b {
            queue.push(i);
        }
    }
    let mut at = 0;
    while at < queue.len() {
        charge(spent, 1)?;
        let id = queue[at];
        at += 1;
        let args = world
            .arguments(u32::try_from(id).map_err(|_| Failure::InternalFailure)?)
            .ok_or(Failure::InternalFailure)?;
        let members = world
            .members(u32::try_from(id).map_err(|_| Failure::InternalFailure)?)
            .ok_or(Failure::InternalFailure)?;
        charge(spent, args.len() + members.len())?;
        for child in args.iter().copied().chain(members.iter().flatten().copied()) {
            let child = child as usize;
            if !used[child] {
                used[child] = true;
                queue.push(child);
            }
        }
    }
    charge(spent, program.type_keys.len())?;
    if used.iter().any(|b| !*b) {
        return Err(reject("closed type inventory differs from complete source demand"));
    }
    Ok(())
}
