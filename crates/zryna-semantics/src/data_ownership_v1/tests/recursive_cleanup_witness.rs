use crate::data_ownership_v1::tests::*;
use zryna_ir::data_ownership_v1::{VerifiedDropAction, VerifiedInstruction};
use zryna_layout::VerifiedLayouts;

#[path = "recursive_cleanup_witness_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    StringRelease(FaultValueIdentity),
    // Logical release request, including an empty-vector no-op; not observed storage freeing.
    VecStorageRelease(FaultValueIdentity),
}

#[derive(Clone, Copy)]
struct Limits {
    work: usize,
    stack: usize,
    events: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Failure {
    Site,
    Prefix,
    Provenance,
    Partial,
    DuplicateOwner,
    WorkLimit,
    StackLimit,
    EventLimit,
    Fault,
}

#[derive(Debug, Eq, PartialEq)]
struct Witness {
    events: Vec<Event>,
    work: usize,
    peak_stack: usize,
}

struct Budget {
    limits: Limits,
    work: usize,
    peak_stack: usize,
}

impl Budget {
    fn charge(&mut self, amount: usize) -> Result<(), Failure> {
        let next = self.work.checked_add(amount).ok_or(Failure::WorkLimit)?;
        if next > self.limits.work {
            return Err(Failure::WorkLimit);
        }
        self.work = next;
        Ok(())
    }
    fn push(&mut self, stack: &mut Vec<Task>, task: Task) -> Result<(), Failure> {
        self.charge(1)?;
        let next = stack.len().checked_add(1).ok_or(Failure::StackLimit)?;
        if next > self.limits.stack {
            return Err(Failure::StackLimit);
        }
        self.peak_stack = self.peak_stack.max(next);
        stack.push(task);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Task {
    Visit(FaultValueIdentity),
    ReleaseStorage(FaultValueIdentity),
}

struct Provenance<'a> {
    fault: VerifiedInstruction<'a>,
    producers: Vec<VerifiedInstruction<'a>>,
    owners: Vec<Option<FaultPlaceIdentity>>,
    roots: Vec<Option<(FaultPlaceIdentity, FaultValueIdentity)>>,
}

fn admitted(kind: VerifiedInstructionKind) -> bool {
    matches!(
        kind,
        VerifiedInstructionKind::BoolLiteral
            | VerifiedInstructionKind::I32Literal
            | VerifiedInstructionKind::StringFromUtf8
            | VerifiedInstructionKind::StructConstruct
            | VerifiedInstructionKind::FixedArrayConstruct
            | VerifiedInstructionKind::EnumConstruct
            | VerifiedInstructionKind::VecConstruct
    )
}

fn provenance<'a>(
    function: VerifiedFunction<'a>,
    instruction: VerifiedInstruction<'a>,
    budget: &mut Budget,
) -> Result<Provenance<'a>, Failure> {
    if function.parameters().len() != 0
        || function.borrow_parameters().len() != 0
        || function.blocks().len() != 1
    {
        return Err(Failure::Prefix);
    }
    let block = function.blocks().next().ok_or(Failure::Site)?;
    if block.parameters().len() != 0 {
        return Err(Failure::Prefix);
    }
    let cleanup = instruction.cleanup().ok_or(Failure::Site)?;
    budget.charge(function.cleanup_plans().len())?;
    let site = function.cleanup_plans().find(|p| p.id() == cleanup).ok_or(Failure::Site)?.site();
    if site.block() != block.id() || site.role() != VerifiedCleanupRole::PrepareFailure {
        return Err(Failure::Site);
    }
    let index = usize::try_from(site.instruction_index().ok_or(Failure::Site)?)
        .map_err(|_| Failure::Site)?;
    let count = index.checked_add(1).ok_or(Failure::WorkLimit)?;
    budget.charge(count)?;
    let mut producers = block.instructions().take(count).collect::<Vec<_>>();
    let actual = producers.pop().ok_or(Failure::Site)?;
    if producers.len() != index
        || actual.result() != instruction.result()
        || actual.cleanup() != instruction.cleanup()
        || actual.kind() != instruction.kind()
        || actual.span() != instruction.span()
        || actual.result_type() != instruction.result_type()
        || !actual.value_operands().eq(instruction.value_operands())
        || actual.string_utf8_bytes() != instruction.string_utf8_bytes()
    {
        return Err(Failure::Site);
    }
    // Scan EVERY preceding instruction, including ones not reached from a cleanup operand.
    for (id, producer) in producers.iter().enumerate() {
        if !admitted(producer.kind()) || producer.result().is_none_or(|v| v.index() as usize != id)
        {
            return Err(Failure::Prefix);
        }
    }
    budget.charge(function.places().len())?;
    let mut owners = vec![None; producers.len()];
    let mut roots = vec![None; function.places().len()];
    for place in function.places() {
        let VerifiedPlaceKind::Temporary(value) = place.kind() else {
            return Err(Failure::Prefix);
        };
        let id = usize::try_from(value.index()).map_err(|_| Failure::Provenance)?;
        if id >= producers.len() {
            continue;
        }
        if producers[id].result() != Some(value) {
            return Err(Failure::Provenance);
        }
        if !place.is_copy() {
            if owners[id].replace(place.id()).is_some() {
                return Err(Failure::Provenance);
            }
            roots[place.id().index() as usize] = Some((place.id(), value));
        }
    }
    Ok(Provenance { fault: actual, producers, owners, roots })
}

fn complete_root(
    action: &VerifiedDropAction,
    context: &Provenance<'_>,
) -> Result<FaultValueIdentity, Failure> {
    if action.kind() != VerifiedDropActionKind::Place
        || action.moved_projections().len() != 0
        || action.initialized_projections().len() != 0
    {
        return Err(Failure::Partial);
    }
    let (place, value) = context
        .roots
        .get(action.root().index() as usize)
        .copied()
        .flatten()
        .ok_or(Failure::Provenance)?;
    if place != action.root() {
        return Err(Failure::Provenance);
    }
    let producer = &context.producers[value.index() as usize];
    if action.active_variant().is_some_and(|variant| producer.variant() != Some(variant))
        || action.active_variants().any(|active| {
            active.place() != action.root() || producer.variant() != Some(active.variant())
        })
    {
        return Err(Failure::Partial);
    }
    Ok(value)
}

fn walk(
    context: &Provenance<'_>,
    layouts: &VerifiedLayouts,
    roots: &[FaultValueIdentity],
    budget: &mut Budget,
) -> Result<Vec<Event>, Failure> {
    let mut stack = Vec::new();
    for root in roots.iter().rev() {
        budget.push(&mut stack, Task::Visit(*root))?;
    }
    let mut released = vec![false; context.producers.len()];
    let mut events = Vec::new();
    while let Some(task) = stack.pop() {
        budget.charge(1)?;
        let (Task::Visit(value) | Task::ReleaseStorage(value)) = task;
        let id = usize::try_from(value.index()).map_err(|_| Failure::Provenance)?;
        let producer = context.producers.get(id).ok_or(Failure::Provenance)?;
        if producer.result() != Some(value) {
            return Err(Failure::Provenance);
        }
        let ty = layouts
            .type_by_id(producer.result_type().ok_or(Failure::Provenance)?)
            .ok_or(Failure::Provenance)?;
        let event = if matches!(task, Task::ReleaseStorage(_)) {
            if producer.kind() != VerifiedInstructionKind::VecConstruct {
                return Err(Failure::Provenance);
            }
            Some(Event::VecStorageRelease(value))
        } else {
            if ty.drop_kind() != 0 {
                if context.owners[id].is_none() {
                    return Err(Failure::Provenance);
                }
                if std::mem::replace(&mut released[id], true) {
                    return Err(Failure::DuplicateOwner);
                }
            }
            match producer.kind() {
                VerifiedInstructionKind::StringFromUtf8 => Some(Event::StringRelease(value)),
                VerifiedInstructionKind::BoolLiteral | VerifiedInstructionKind::I32Literal => None,
                VerifiedInstructionKind::VecConstruct => {
                    budget.push(&mut stack, Task::ReleaseStorage(value))?;
                    for child in producer.value_operands() {
                        budget.push(&mut stack, Task::Visit(child))?;
                    }
                    None
                }
                VerifiedInstructionKind::StructConstruct
                | VerifiedInstructionKind::FixedArrayConstruct
                | VerifiedInstructionKind::EnumConstruct => {
                    for child in producer.value_operands() {
                        budget.push(&mut stack, Task::Visit(child))?;
                    }
                    None
                }
                _ => return Err(Failure::Prefix),
            }
        };
        if let Some(event) = event {
            if events.len() >= budget.limits.events {
                return Err(Failure::EventLimit);
            }
            events.push(event);
        }
    }
    Ok(events)
}

fn witness(
    abi: &VerifiedOwnershipRuntimeAbi,
    layouts: &VerifiedLayouts,
    function: VerifiedFunction<'_>,
    instruction: VerifiedInstruction<'_>,
    status: RuntimeStatus,
    limits: Limits,
) -> Result<Witness, Failure> {
    let operation = match instruction.kind() {
        VerifiedInstructionKind::StringFromUtf8 => LogicalOperation::StringFromUtf8Copy,
        VerifiedInstructionKind::VecConstruct => LogicalOperation::VecAllocate,
        _ => return Err(Failure::Site),
    };
    let mut budget = Budget { limits, work: 0, peak_stack: 0 };
    let context = provenance(function, instruction, &mut budget)?;
    // Continue from the instruction retrieved from THIS function's exact site, not a caller view.
    let instruction = context.fault;
    let trace = owned_fault_trace(
        abi,
        function,
        instruction,
        OwnedFaultInjection::Runtime { operation, status },
        0,
        1,
    )
    .map_err(|_| Failure::Fault)?;
    if trace.result_committed
        || trace.uncommitted_result != instruction.result()
        || trace.prefix_owner.is_some()
        || !trace.reverse_prefix.is_empty()
    {
        return Err(Failure::Fault);
    }
    let action_view = instruction.derived_drop_actions();
    budget.charge(action_view.len())?;
    let actions = action_view.collect::<Vec<_>>();
    if actions.iter().map(VerifiedDropAction::root).collect::<Vec<_>>() != trace.reverse_cleanup {
        return Err(Failure::Fault);
    }
    let mut roots = Vec::new();
    for action in &actions {
        budget.charge(1)?;
        roots.push(complete_root(action, &context)?);
    }
    let events = walk(&context, layouts, &roots, &mut budget)?;
    Ok(Witness { events, work: budget.work, peak_stack: budget.peak_stack })
}
