//! Cleanup is conditional on exact unmodified status, inside its actual structured guard.
use super::{
    Diagnostic, Event, Expr, Model, O, Region, State, UpgradeStatus, call, invariant, push,
    sections,
};
pub(super) fn step(
    s: &mut State,
    op: &O<'_>,
    events: &mut Vec<Event>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    match *op {
        O::If { .. } => condition(s, events, m)?,
        O::Block { .. } | O::Loop { .. } => s.region(Region::None)?,
        O::End => {
            if s.depth == 0 && s.container() && s.len > 0 {
                let value = s.pop()?;
                push(
                    events,
                    if s.wide() { Event::WideReturn(value) } else { Event::RuntimeReturn(value) },
                    m,
                )?;
            }
            if s.depth > 0 {
                s.depth -= 1;
                let region = s.regions[s.depth];
                if region != Region::None {
                    push(events, Event::End(region), m)?;
                }
            }
        }
        O::Call { function_index } => {
            s.last_target = Some(function_index);
            let (parameters, results) = sections::signature(m, function_index as usize)?;
            let mut args = [Expr::Unknown; 6];
            if parameters > 6 {
                return Err(invariant());
            }
            for n in (0..parameters as usize).rev() {
                args[n] = s.pop()?;
            }
            call(events, function_index, &args[..parameters as usize], m)?;
            for _ in 0..results {
                let value = if (s.container() || s.source())
                    && m.container_count != 0
                    && (function_index
                        == m.container(
                            crate::generic_execution_v1::model::containers::ALLOCATE_RAW,
                        )
                        || (s.source() && function_index == m.control_birth()))
                {
                    Expr::PackedCallResult(function_index)
                } else if s.container() {
                    Expr::CallResult(function_index)
                } else {
                    Expr::Unknown
                };
                s.push(value)?;
            }
        }
        O::Return => {
            if s.wide() {
                let value = s.pop()?;
                push(events, Event::WideReturn(value), m)?;
            } else if s.container() {
                let value = s.pop()?;
                push(events, Event::RuntimeReturn(value), m)?;
            }
            if s.child() && s.len > 0 && s.values[s.len - 1] == Expr::Local(2) {
                if !s.inside(Region::Child) {
                    return Err(invariant());
                }
                push(events, Event::ChildReturn, m)?;
            }
            if s.source() {
                let result = s.pop()?;
                let event = match result {
                    Expr::Global(1) if s.inside(Region::Failure) => Event::FailureReturn,
                    Expr::Const(0) => Event::SuccessReturn,
                    Expr::Const(n) if s.status == Some(Expr::Const(n)) => Event::TrapReturn(n),
                    _ => return Err(invariant()),
                };
                push(events, event, m)?;
            }
            s.len = 0;
        }
        O::Br { relative_depth } => {
            if s.source() {
                push(events, Event::Jump(relative_depth), m)?;
            }
            s.len = 0;
        }
        O::BrIf { .. } => {
            if s.source() {
                return Err(invariant());
            }
            s.pop()?;
        }
        O::Unreachable => {
            if s.semantic() {
                push(events, Event::Trap, m)?;
            }
            s.len = 0;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn condition(
    s: &mut State,
    events: &mut Vec<Event>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let value = s.pop()?;
    let mut region = Region::None;
    if s.pending {
        region = pending_condition(s, value, events, m)?;
    } else if (s.release()
        || s.last_target
            == Some(m.container(crate::generic_execution_v1::model::containers::VEC_RELEASE)))
        && s.last_call
        && value == Expr::Unknown
    {
        push(events, Event::ReleaseFailure, m)?;
        region = Region::Release;
    } else if s.counts() && matches!(value, Expr::ControlZero(0 | 4) | Expr::NoOwners) {
        region = match value {
            Expr::ControlZero(0) => Region::StrongZero,
            Expr::ControlZero(4) => Region::WeakZero,
            Expr::NoOwners => Region::NoOwners,
            _ => return Err(invariant()),
        };
        push(events, Event::ControlGuard(region), m)?;
    } else if s.semantic() {
        region = semantic_condition(s, value, events, m)?;
    } else if let Some(window) = &s.runtime {
        push(events, Event::RuntimeGuard(window.guard(m)?), m)?;
        region = Region::RuntimeGuard;
    }
    s.region(region)
}

fn semantic_condition(
    s: &State,
    value: Expr,
    events: &mut Vec<Event>,
    m: &Model<'_, '_, '_>,
) -> Result<Region, Diagnostic> {
    let region = match value {
        Expr::LoanAppendFull(base, cell, delta) if s.source() => {
            push(events, Event::LoanAppendFull(base, cell, delta), m)?;
            Region::AppendFull
        }
        Expr::LoanVecBounds(base, cell, delta) if s.source() => {
            push(events, Event::LoanVecBounds(base, cell, delta), m)?;
            Region::Bounds
        }
        Expr::AppendFull(offset) if s.source() => {
            push(events, Event::AppendFull(offset), m)?;
            Region::AppendFull
        }
        Expr::UpgradeSuccess if s.source() => {
            push(events, Event::UpgradeSuccess, m)?;
            Region::UpgradeSuccess
        }
        Expr::UpgradeExpired if s.source() => {
            push(events, Event::UpgradeExpired, m)?;
            Region::UpgradeExpired
        }
        Expr::LoanTag(base, cell, delta, ordinal) if s.source() => {
            push(events, Event::LoanTag(base, cell, delta, ordinal), m)?;
            Region::Tag
        }
        Expr::Tag(base, offset, ordinal) => {
            push(events, Event::Tag { base, offset, ordinal }, m)?;
            Region::Tag
        }
        Expr::Element(n) if s.vector() => {
            push(events, Event::Element(n), m)?;
            Region::Element
        }
        Expr::Capacity(n) if s.vector() => {
            push(events, Event::Capacity(n), m)?;
            Region::Capacity
        }
        Expr::Empty if s.vector() => {
            push(events, Event::Empty, m)?;
            Region::Empty
        }
        Expr::Length if s.vector() => {
            push(events, Event::Length, m)?;
            Region::Length
        }
        Expr::AbsoluteLoad(address, 4)
            if s.vector() && address == i64::from(m.memory.claims().scratch_start + 4) =>
        {
            push(events, Event::AllocationStatus, m)?;
            Region::AllocationStatus
        }
        Expr::AbsoluteLoad(address, 4)
            if s.shared() && address == i64::from(m.memory.claims().scratch_start) =>
        {
            push(events, Event::LastStrong, m)?;
            Region::LastStrong
        }
        Expr::VecBounds(base, offset) => {
            push(events, Event::VecBounds(base, offset), m)?;
            Region::Bounds
        }
        Expr::Bounds(n) => {
            push(events, Event::Bounds(n), m)?;
            Region::Bounds
        }
        Expr::Pc(n) if s.source() => {
            push(events, Event::Block(n), m)?;
            Region::Block
        }
        Expr::Load(0, offset, 1) if s.source() => {
            push(events, Event::Branch(offset), m)?;
            Region::Branch
        }
        _ => return Err(invariant()),
    };
    Ok(region)
}

fn pending_condition(
    s: &mut State,
    value: Expr,
    events: &mut Vec<Event>,
    m: &Model<'_, '_, '_>,
) -> Result<Region, Diagnostic> {
    let local = if s.child() { 2 } else { 3 };
    if value != Expr::Local(local) {
        return Err(invariant());
    }
    let region = if s.child() { Region::Child } else { Region::Failure };
    push(events, if s.child() { Event::ChildFailure } else { Event::Failure }, m)?;
    s.pending = false;
    s.upgrade_status = UpgradeStatus::Absent;
    Ok(region)
}
