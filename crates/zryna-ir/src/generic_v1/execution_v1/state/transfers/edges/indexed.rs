//! Only source-authenticated borrowed arm edges can retain lexical indexed ancestry.
use super::{Context, Failure, budget, reject};
use crate::generic_v1::execution_v1::places;

pub(super) fn allowed(
    c: &Context<'_>,
    loan: u32,
    from: usize,
    target: usize,
    work: &mut usize,
) -> Result<bool, Failure> {
    let scopes = &c.checked().match_scopes;
    budget::charge(work, scopes.len())?;
    for scope in scopes {
        let entry = from == scope.entry && target == scope.blocks.start;
        let inside = scope.blocks.contains(&from)
            && (scope.blocks.contains(&target) || target == scope.join);
        if !entry && !inside {
            continue;
        }
        if ancestor(c, loan, scope.parent, work)? {
            return Ok(true);
        }
        // A selected child may cross internal arm control only. It must end before
        // the result join and may never become a sibling arm's live permission.
        if target != scope.join
            && places::within(scope.region, c.raw().loans[loan as usize].region.span)
            && ancestor(c, scope.parent, loan, work)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ancestor(
    c: &Context<'_>,
    ancestor: u32,
    mut child: u32,
    work: &mut usize,
) -> Result<bool, Failure> {
    loop {
        budget::charge(work, 1)?;
        if child == ancestor {
            return Ok(true);
        }
        let row = c
            .checked()
            .places
            .loans
            .get(child as usize)
            .ok_or_else(|| reject("match scope names a foreign loan"))?;
        let Some(parent) = row.parent else {
            return Ok(false);
        };
        child = parent;
    }
}
