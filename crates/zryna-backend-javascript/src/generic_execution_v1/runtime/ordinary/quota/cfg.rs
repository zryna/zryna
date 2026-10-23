use super::{Context, Diagnostic, Function, charge, emission_budget, filled, raw};
pub(super) fn edges(
    t: &raw::Terminator,
) -> impl Iterator<Item = (&raw::Edge, Option<&raw::ValueDefinition>)> {
    let (a, b, arms) = match t {
        raw::Terminator::Jump(e) => (Some((e, None)), None, &[][..]),
        raw::Terminator::Branch { yes, no, .. } => (Some((yes, None)), Some((no, None)), &[][..]),
        raw::Terminator::ClosedEnumMatch { arms, .. } => (None, None, arms.as_slice()),
        raw::Terminator::WeakUpgradeBranch { success, expired, success_binding, .. } => {
            (Some((success, Some(success_binding))), Some((expired, None)), &[][..])
        }
        _ => (None, None, &[][..]),
    };
    a.into_iter().chain(b).chain(arms.iter().map(|arm| {
        (
            &arm.edge,
            match &arm.binding {
                Some(raw::MatchBinding::Value(v)) => Some(v),
                _ => None,
            },
        )
    }))
}
pub(super) fn order(c: &Context<'_, '_>, f: &Function<'_, '_>) -> Result<Vec<usize>, Diagnostic> {
    let n = f.blocks.len();
    let mut degree = filled(c, n, 0usize)?;
    charge(&c.work, n)?;
    let mut visits = 0usize;
    for b in &f.blocks {
        let edges = match b.terminator() {
            raw::Terminator::ClosedEnumMatch { arms, .. } => arms.len(),
            raw::Terminator::Jump(_) => 1,
            raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
            _ => 0,
        };
        visits = visits.checked_add(edges).ok_or_else(emission_budget)?;
    }
    charge(
        &c.work,
        visits
            .checked_mul(2)
            .and_then(|e| e.checked_add(n.checked_mul(2)?))
            .ok_or_else(emission_budget)?,
    )?;
    for b in &f.blocks {
        for (e, _) in edges(b.terminator()) {
            degree[e.target as usize] =
                degree[e.target as usize].checked_add(1).ok_or_else(emission_budget)?;
        }
    }
    let mut queue = filled(c, n, 0usize)?;
    queue.clear();
    let mut result = filled(c, n, 0usize)?;
    result.clear();
    for (i, d) in degree.iter().enumerate() {
        if *d == 0 {
            queue.push(i);
        }
    }
    while let Some(i) = queue.pop() {
        result.push(i);
        for (e, _) in edges(f.blocks[i].terminator()) {
            let d = &mut degree[e.target as usize];
            *d -= 1;
            if *d == 0 {
                queue.push(e.target as usize);
            }
        }
    }
    if result.len() != n {
        return Err(emission_budget());
    }
    Ok(result)
}
