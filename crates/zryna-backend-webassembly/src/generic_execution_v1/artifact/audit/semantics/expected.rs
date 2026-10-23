//! Derives call/address/control obligations solely from seal, memory plan and runtime roles.
use super::{Diagnostic, Event, Expr, Model, Region, call, invariant, push};
use zryna_ir::generic_v1::execution_v1::{VerifiedCleanupAction, VerifiedCleanupPlan, raw};
mod containers;
mod helpers;
mod helpers_containers;
mod operations;
mod source;
pub(super) fn events(index: usize, m: &Model<'_, '_, '_>) -> Result<Vec<Event>, Diagnostic> {
    let mut events = Vec::new();
    if index >= m.container_base as usize {
        containers::events(
            crate::generic_execution_v1::checked_u32(index)? - m.container_base,
            m,
            &mut events,
        )?;
        return Ok(events);
    }
    if index < 5 {
        runtime(index, m, &mut events)?;
    } else if index < m.source_base as usize {
        helpers::events(index, m, &mut events)?;
    } else if index < m.wrapper_base as usize {
        source::events(index - m.source_base as usize, m, &mut events)?;
    } else if index < m.observer as usize {
        let selected = m.program.export_functions()[index - m.wrapper_base as usize];
        push(&mut events, Event::Status(0), m)?;
        for (ordinal, param) in m.functions[selected].parameters().enumerate() {
            let zryna_ir::generic_v1::execution_v1::VerifiedParameter::Value(v) = param else {
                return Err(invariant());
            };
            let slot = value(m, selected, v.id())?;
            store(
                &mut events,
                Expr::Const(i64::from(m.memory.claims().frames_start + slot.offset)),
                Expr::Local(crate::generic_execution_v1::checked_u32(ordinal)?),
                crate::generic_execution_v1::checked_u8(slot.bytes)?,
                m,
            )?;
        }
        call(
            &mut events,
            m.source_base + crate::generic_execution_v1::checked_u32(selected)?,
            &[
                Expr::Const(i64::from(m.memory.claims().frames_start)),
                Expr::Const(i64::from(m.memory.claims().scratch_start) + 16),
            ],
            m,
        )?;
    }
    if index >= m.wrapper_base as usize && index < m.observer as usize {
        let selected = m.program.export_functions()[index - m.wrapper_base as usize];
        let raw::ValueType::Stored(ty) = m.functions[selected].result_type() else {
            return Err(invariant());
        };
        read(
            &mut events,
            Expr::Const(i64::from(m.memory.claims().scratch_start) + 16),
            crate::generic_execution_v1::checked_u8(m.types[ty as usize].size())?,
            m,
        )?;
    }
    if m.observed() && index == m.recorder as usize {
        for local in 0..5 {
            store(&mut events, Expr::address(6, i64::from(local * 4)), Expr::Local(local), 4, m)?;
        }
        read(&mut events, Expr::Indexed(5, 0, 7, 1), 1, m)?;
        store(&mut events, Expr::Indexed(6, 20, 7, 4), Expr::Unknown, 4, m)?;
    }
    Ok(events)
}
fn runtime(index: usize, m: &Model<'_, '_, '_>, out: &mut Vec<Event>) -> Result<(), Diagnostic> {
    use Expr::{Const as C, Global as G, Local as P};
    match index {
        0 => {
            read(out, Expr::Indexed(0, 0, 3, 1), 1, m)?;
            store(out, Expr::Indexed(1, 0, 3, 1), Expr::Unknown, 1, m)?;
        }
        1 => call(out, 4, &[P(0), P(1), C(0), C(1), P(2)], m)?,
        2 => {
            push(out, Event::Status(255), m)?;
            for offset in [0, 4, 8] {
                store(out, Expr::address(3, offset), C(0), 4, m)?;
            }
            for _ in 0..3 {
                push(out, Event::Status(255), m)?;
            }
            if m.observed() {
                call(out, m.recorder, &[C(5), G(6), C(0), P(0), P(1), P(0)], m)?;
                push(out, Event::Status(2), m)?;
            }
            call(out, 4, &[P(0), P(1), P(0), C(2), P(3)], m)?;
        }
        3 => {
            for _ in 0..4 {
                push(out, Event::Status(255), m)?;
            }
            if m.observed() {
                call(out, m.recorder, &[C(3), Expr::Load(0, -16, 4), P(0), C(0), P(1), P(0)], m)?;
            }
            store(out, Expr::address(0, -8), C(0), 4, m)?;
        }
        4 => {
            for offset in [0, 4, 8] {
                store(out, Expr::address(4, offset), C(0), 4, m)?;
            }
            if m.observed() {
                call(out, m.recorder, &[C(4), G(5), C(0), P(2), P(1), P(0)], m)?;
                push(out, Event::Status(2), m)?;
            }
            push(out, Event::Status(3), m)?;
            if m.observed() {
                call(out, m.recorder, &[C(4), G(5), C(0), P(2), P(1), P(0)], m)?;
            }
            push(out, Event::Status(2), m)?;
            for (offset, value) in [(0, P(8)), (4, P(1)), (8, C(1)), (12, C(0))] {
                store(out, Expr::address(5, offset), value, 4, m)?;
            }
            call(out, 0, &[P(0), P(6), P(1)], m)?;
            for (offset, value) in [(0, P(6)), (4, P(1)), (8, P(1))] {
                store(out, Expr::address(4, offset), value, 4, m)?;
            }
            if m.observed() {
                call(out, m.recorder, &[P(3), P(8), P(6), P(2), P(1), P(6)], m)?;
            }
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn frame(offset: u32) -> Expr {
    Expr::address(0, i64::from(offset))
}
fn value<'m>(
    m: &'m Model<'_, '_, '_>,
    f: usize,
    id: u32,
) -> Result<&'m crate::generic_execution_v1::memory::raw::Slot, Diagnostic> {
    m.memory.claims().functions[f].values.get(id as usize).ok_or_else(invariant)
}
fn place<'m>(
    m: &'m Model<'_, '_, '_>,
    f: usize,
    id: u32,
) -> Result<&'m crate::generic_execution_v1::memory::raw::Slot, Diagnostic> {
    m.memory.claims().functions[f].places.get(id as usize).ok_or_else(invariant)
}
fn referent<'m>(
    m: &'m Model<'_, '_, '_>,
    f: usize,
    p: raw::PlaceOrLoan,
) -> Result<&'m crate::generic_execution_v1::memory::raw::Slot, Diagnostic> {
    let raw::PlaceOrLoan::Place(id) = p else { return Err(invariant()) };
    place(m, f, id)
}
fn copy(
    out: &mut Vec<Event>,
    src: Expr,
    dst: Expr,
    bytes: u32,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    call(out, 0, &[src, dst, Expr::Const(i64::from(bytes))], m)
}
fn drop(
    out: &mut Vec<Event>,
    offset: u32,
    ty: raw::ValueType,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if let raw::ValueType::Stored(ty) = ty {
        call(
            out,
            5 + crate::generic_execution_v1::checked_u32(m.types.len())? + ty,
            &[frame(offset)],
            m,
        )?;
    }
    Ok(())
}
fn cleanup(
    out: &mut Vec<Event>,
    plan: VerifiedCleanupPlan<'_, '_>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    m.take(plan.actions().len())?;
    for action in plan.actions() {
        let VerifiedCleanupAction::DropPlace(p) = action else { return Err(invariant()) };
        let slot = place(m, p.function(), p.id())?;
        drop(out, slot.offset, slot.ty, m)?;
    }
    Ok(())
}
fn failure(
    out: &mut Vec<Event>,
    plan: Option<VerifiedCleanupPlan<'_, '_>>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    push(out, Event::Failure, m)?;
    if let Some(plan) = plan {
        cleanup(out, plan, m)?;
    } else {
        push(out, Event::Trap, m)?;
    }
    push(out, Event::FailureReturn, m)?;
    push(out, Event::End(Region::Failure), m)
}

fn store(
    out: &mut Vec<Event>,
    address: Expr,
    value: Expr,
    width: u8,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    push(out, Event::Store { address, value, width }, m)
}

fn read(
    out: &mut Vec<Event>,
    address: Expr,
    width: u8,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    push(out, Event::Read { address, width }, m)
}
