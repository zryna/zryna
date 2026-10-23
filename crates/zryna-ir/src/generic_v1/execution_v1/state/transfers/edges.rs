//! Ordinary CFG arguments rename pending slots simultaneously before joining state.
use super::{Context, Failure, State, raw, reject};
use crate::generic_v1::execution_v1::{budget, reserve, resource_failure};
pub(super) fn transfer(
    c: &Context<'_>,
    mut state: State,
    edge: &raw::Edge,
    binding: Option<&raw::ValueDefinition>,
    incoming: &mut [Option<State>],
    queue: &mut Vec<usize>,
    work: &mut usize,
) -> Result<(), Failure> {
    let target = edge.target as usize;
    let parameters = &c.raw().blocks[target].value_parameters;
    let skip = usize::from(binding.is_some());
    for id in &edge.arguments {
        state.value(c, *id)?;
    }
    let order = renamed_order(c, &state, edge, &parameters[skip..], work)?;
    for id in &edge.arguments {
        state.consume(c, *id)?;
    }
    if let Some(d) = binding {
        state.publish(c, d, None)?;
    }
    for d in &parameters[skip..] {
        state.publish(c, d, None)?;
    }
    if let Some(mut order) = order {
        // A successful upgrade acquires a genuinely new obligation after existing owners.
        // Selected match bindings retain their existing transition semantics separately.
        if let Some(d) = binding
            && c.affine(d.ty)?
        {
            let owner = c.checked().places.owners[d.id as usize]
                .ok_or_else(|| reject("generated affine edge binding lacks exact owner"))?;
            if order.len() >= order.capacity() {
                return Err(reject("edge order exceeds prepaid place capacity"));
            }
            order.push(owner);
        }
        if order.len() != state.owners.len() {
            return Err(reject("edge owner renaming lost a pending obligation"));
        }
        state.owners = order;
    }
    // Tags on incoming values are unknown; no predecessor constant authorizes another.
    if let Some(previous) = &incoming[target] {
        if !state.same(previous, c)? {
            return Err(reject("execution predecessor/backedge ownership states differ"));
        }
    } else {
        incoming[target] = Some(state);
        queue.push(target);
    }
    Ok(())
}
fn renamed_order(
    c: &Context<'_>,
    state: &State,
    edge: &raw::Edge,
    parameters: &[raw::ValueDefinition],
    work: &mut usize,
) -> Result<Option<Vec<u32>>, Failure> {
    if edge.arguments.is_empty() {
        return Ok(None);
    }
    let places = c.raw().places.len();
    // 4P scratch bytes, P copy, P final replacement, P guards, and P*A comparisons.
    // Charge the full inventory before reserve or inspecting the pending owner order.
    let cost = edge
        .arguments
        .len()
        .checked_add(8)
        .and_then(|n| n.checked_mul(places))
        .and_then(|n| n.checked_add(edge.arguments.len()))
        .and_then(|n| n.checked_add(8))
        .ok_or_else(|| resource_failure("CFG pending-anchor scratch work overflow"))?;
    budget::charge(work, cost)?;
    let mut order = reserve(places)?;
    order.extend_from_slice(&state.owners);
    for owner in &mut order {
        for (argument, parameter) in edge.arguments.iter().zip(parameters) {
            if c.checked().places.owners[*argument as usize] == Some(*owner) {
                *owner = c.checked().places.owners[parameter.id as usize]
                    .ok_or_else(|| reject("affine edge target lacks exact owning place"))?;
                break;
            }
        }
    }
    Ok(Some(order))
}
