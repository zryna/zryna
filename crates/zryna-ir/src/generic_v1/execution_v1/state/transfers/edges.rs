//! Ordinary CFG arguments rename pending slots simultaneously before joining state.
use super::{Context, Failure, State, raw, reject};
use crate::generic_v1::execution_v1::{budget, reserve, resource_failure};
#[derive(Clone, Copy)]
struct Mode<'a> {
    binding: Option<&'a raw::ValueDefinition>,
    selected: bool,
}
pub(super) fn transfer(
    c: &Context<'_>,
    state: State,
    edge: (usize, &raw::Edge),
    binding: Option<&raw::ValueDefinition>,
    incoming: &mut [Option<State>],
    queue: &mut Vec<usize>,
    work: &mut usize,
) -> Result<(), Failure> {
    transfer_kind(c, state, edge, Mode { binding, selected: false }, incoming, queue, work)
}
pub(super) fn borrowed_transfer(
    c: &Context<'_>,
    state: State,
    edge: (usize, &raw::Edge),
    incoming: &mut [Option<State>],
    queue: &mut Vec<usize>,
    work: &mut usize,
) -> Result<(), Failure> {
    transfer_kind(c, state, edge, Mode { binding: None, selected: true }, incoming, queue, work)
}
fn transfer_kind(
    c: &Context<'_>,
    mut state: State,
    edge: (usize, &raw::Edge),
    mode: Mode<'_>,
    incoming: &mut [Option<State>],
    queue: &mut Vec<usize>,
    work: &mut usize,
) -> Result<(), Failure> {
    let Mode { binding, selected } = mode;
    let (from, edge) = edge;
    let target = edge.target as usize;
    admission(c, &state, from, target, selected, work)?;
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
    // A repeated header cannot inherit a complete owner's first-entry discriminant.
    // Partial enum storage still requires its exact active-variant authority.
    if c.checked().graph.loop_header(target) {
        budget::charge(work, state.variants.len())?;
        for (place, variant) in state.variants.iter_mut().enumerate() {
            if state.initialized[place] {
                *variant = None;
            }
        }
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
mod indexed;
fn admission(
    c: &Context<'_>,
    state: &State,
    from: usize,
    target: usize,
    selected: bool,
    work: &mut usize,
) -> Result<(), Failure> {
    budget::charge(work, state.loan_order.len())?;
    for &id in &state.loan_order {
        let loan = &c.raw().loans[id as usize];
        if c.checked().places.loans[id as usize].dynamic.is_some()
            && loan.region.role != raw::RegionRole::TransientIndexedAccess
            && !selected
            && !indexed::allowed(c, id, from, target, work)?
        {
            return Err(reject("lexical indexed loan crosses an ordinary execution edge"));
        }
    }
    if c.checked().graph.loop_header(target) {
        // Formal call permissions are established by the signature. A lexical body
        // loan must end before either first header entry or a repeated latch.
        budget::charge(work, state.loan_order.len())?;
        if state.loan_order.iter().any(|id| !c.checked().places.loans[*id as usize].formal) {
            return Err(reject("lexical loan crosses an execution loop header"));
        }
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
