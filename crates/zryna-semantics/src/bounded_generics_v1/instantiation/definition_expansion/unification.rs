//! Complete occurs proof for the restricted equations `Xi = original argument_i`.
use super::{InstantiationFailure, Term, Work};

pub(super) fn expands(
    arguments: [Option<Term>; 2],
    arity: usize,
    work: &mut Work,
) -> Result<bool, InstantiationFailure> {
    if !(1..=2).contains(&arity)
        || arguments[..arity].iter().any(Option::is_none)
        || arguments[arity..].iter().any(Option::is_some)
    {
        return Err(InstantiationFailure::InternalFailure);
    }
    let mut bindings = [None; 2];
    // Each distinct original variable occurs once on the left. Earlier equations bind
    // only their own left variable, so a later left variable is still free. There is no
    // constructor/constructor pair product, substituted tree, or recursive expansion.
    for (variable, argument) in arguments[..arity].iter().enumerate() {
        work.take(1)?;
        let term = resolve(
            argument.ok_or(InstantiationFailure::InternalFailure)?,
            &bindings,
            arity,
            work,
        )?;
        let variable_id =
            u8::try_from(variable).map_err(|_| InstantiationFailure::InternalFailure)?;
        if term.variable == Some(variable_id) {
            continue;
        }
        if occurs(variable, term.parameters, &bindings, arity, work)? {
            return Ok(true);
        }
        bindings[variable] = Some(term);
    }
    Ok(false)
}

fn resolve(
    mut term: Term,
    bindings: &[Option<Term>; 2],
    arity: usize,
    work: &mut Work,
) -> Result<Term, InstantiationFailure> {
    for _ in 0..=arity {
        work.take(1)?;
        if term.parameters & !((1u8 << arity) - 1) != 0 {
            return Err(InstantiationFailure::InternalFailure);
        }
        let Some(variable) = term.variable else { return Ok(term) };
        if variable as usize >= arity || term.parameters != 1u8 << variable {
            return Err(InstantiationFailure::InternalFailure);
        }
        let Some(next) = bindings[variable as usize] else { return Ok(term) };
        if next.owner != term.owner {
            return Err(InstantiationFailure::InternalFailure);
        }
        term = next;
    }
    Err(InstantiationFailure::InternalFailure)
}

fn occurs(
    variable: usize,
    mut pending: u8,
    bindings: &[Option<Term>; 2],
    arity: usize,
    work: &mut Work,
) -> Result<bool, InstantiationFailure> {
    let mut seen = 0;
    while pending != 0 {
        work.take(1)?;
        let index = pending.trailing_zeros() as usize;
        if index >= arity {
            return Err(InstantiationFailure::InternalFailure);
        }
        let bit = 1u8 << index;
        pending &= !bit;
        seen |= bit;
        if index == variable {
            return Ok(true);
        }
        if let Some(term) = bindings[index] {
            pending |= term.parameters & !seen;
        }
    }
    Ok(false)
}
