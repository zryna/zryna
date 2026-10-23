//! Normative runtime guard, status, return, record and lifetime obligations.
use super::super::predicates::Guard;
use super::{Diagnostic, Event, Expr, Model, Region, call, push, read, store};
use crate::generic_execution_v1::model::containers as C;
use zryna_layout::StorageTarget;
pub(super) fn events(
    role: u32,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    match role {
        C::ALLOCATE_RAW => allocation(out, m)?,
        C::RELEASE_RAW => {
            begin(out, Guard::ReleaseNull, m)?;
            failure(out, Guard::ReleaseSize, 255, 255, m)?;
            returned(out, Expr::Const(0), m)?;
            end(out, Region::RuntimeGuard, m)?;
            valid(out, m)?;
            for (offset, guard) in [(-12, Guard::HeaderSize), (-4, Guard::HeaderAlignment)] {
                read(out, Expr::address(0, offset), 4, m)?;
                failure(out, guard, 255, 255, m)?;
            }
            free(out, m)?;
            returned(out, Expr::Const(0), m)?;
        }
        C::VEC_ALLOCATE => vector(out, m, true)?,
        C::VEC_RELEASE => vector(out, m, false)?,
        C::STRONG_CLONE | C::DOWNGRADE | C::WEAK_CLONE => acquiring(role, out, m)?,
        C::STRONG_BEGIN => {
            store(out, Expr::Local(1), Expr::Const(0), 4, m)?;
            control(out, m, false)?;
            read(out, Expr::Local(0), 4, m)?;
            push(out, Event::ControlGuard(Region::StrongZero), m)?;
            status_return(out, 255, 255, m)?;
            end(out, Region::StrongZero, m)?;
            decrement(out, 0, m)?;
            read(out, Expr::Local(0), 4, m)?;
            push(out, Event::ControlGuard(Region::StrongZero), m)?;
            read(out, Expr::address(0, -4), 4, m)?;
            store(out, Expr::address(0, -4), Expr::Unknown, 4, m)?;
            store(out, Expr::Local(1), Expr::Const(1), 4, m)?;
            end(out, Region::StrongZero, m)?;
            returned(out, Expr::Const(0), m)?;
        }
        C::STRONG_FINISH => {
            control(out, m, true)?;
            read(out, Expr::Local(0), 4, m)?;
            read(out, Expr::address(0, 4), 4, m)?;
            failure(out, Guard::FinishCounts, 255, 255, m)?;
            read(out, Expr::address(0, -4), 4, m)?;
            store(out, Expr::address(0, -4), Expr::Unknown, 4, m)?;
            decrement(out, 4, m)?;
            read(out, Expr::address(0, 4), 4, m)?;
            push(out, Event::ControlGuard(Region::WeakZero), m)?;
            free(out, m)?;
            end(out, Region::WeakZero, m)?;
            returned(out, Expr::Const(0), m)?;
        }
        C::WEAK_RELEASE => {
            store(out, Expr::Local(1), Expr::Const(0), 4, m)?;
            control(out, m, false)?;
            read(out, Expr::address(0, 4), 4, m)?;
            read(out, Expr::Local(0), 4, m)?;
            failure(out, Guard::WeakReleaseCounts, 255, 255, m)?;
            decrement(out, 4, m)?;
            read(out, Expr::Local(0), 4, m)?;
            read(out, Expr::address(0, 4), 4, m)?;
            push(out, Event::ControlGuard(Region::NoOwners), m)?;
            free(out, m)?;
            store(out, Expr::Local(1), Expr::Const(1), 4, m)?;
            end(out, Region::NoOwners, m)?;
            returned(out, Expr::Const(0), m)?;
        }
        _ => return Err(super::invariant()),
    }
    Ok(())
}
fn allocation(out: &mut Vec<Event>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    for (guard, status, result) in
        [(Guard::AllocateArguments, 255, 255i64 << 32), (Guard::AllocateZero, 0, 0)]
    {
        begin(out, guard, m)?;
        if status != 0 {
            push(out, Event::Status(status), m)?;
        }
        wide(out, Expr::Const(result), m)?;
        end(out, Region::RuntimeGuard, m)?;
    }
    if m.observed() {
        begin(out, Guard::AllocationFault, m)?;
        push(out, Event::Status(2), m)?;
        wide(out, Expr::Const(1i64 << 32), m)?;
        end(out, Region::RuntimeGuard, m)?;
    }
    for (guard, status, result) in
        [(Guard::AllocationCapacity, 3, 2i64 << 32), (Guard::AllocationAddress, 2, 1i64 << 32)]
    {
        begin(out, guard, m)?;
        push(out, Event::Status(status), m)?;
        wide(out, Expr::Const(result), m)?;
        end(out, Region::RuntimeGuard, m)?;
    }
    for (offset, value) in
        [(0, Expr::Global(7)), (4, Expr::Local(0)), (8, Expr::Const(1)), (12, Expr::Local(1))]
    {
        store(out, Expr::address(2, offset), value, 4, m)?;
    }
    wide(out, Expr::UnsignedLocal(3), m)?;
    Ok(())
}
fn acquiring(role: u32, out: &mut Vec<Event>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    control(out, m, false)?;
    let live = if role == C::WEAK_CLONE { 4 } else { 0 };
    read(out, Expr::address(0, live), 4, m)?;
    failure(out, Guard::CountZero(crate::generic_execution_v1::checked_u32(live)?), 255, 255, m)?;
    let offset = if role == C::STRONG_CLONE { 0 } else { 4 };
    read(out, Expr::address(0, offset), 4, m)?;
    failure(out, Guard::CountMax(crate::generic_execution_v1::checked_u32(offset)?), 4, 3, m)?;
    read(out, Expr::address(0, offset), 4, m)?;
    store(out, Expr::address(0, offset), Expr::Deref(0, offset, 1), 4, m)?;
    returned(out, Expr::Const(0), m)?;
    Ok(())
}
fn wide(out: &mut Vec<Event>, value: Expr, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    push(out, Event::WideReturn(value), m)
}
fn returned(out: &mut Vec<Event>, value: Expr, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    push(out, Event::RuntimeReturn(value), m)
}
fn begin(out: &mut Vec<Event>, guard: Guard, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    push(out, Event::RuntimeGuard(guard), m)
}
fn end(out: &mut Vec<Event>, region: Region, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    push(out, Event::End(region), m)
}
fn status_return(
    out: &mut Vec<Event>,
    status: i64,
    result: i64,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    push(out, Event::Status(status), m)?;
    returned(out, Expr::Const(result), m)
}
fn failure(
    out: &mut Vec<Event>,
    guard: Guard,
    status: i64,
    result: i64,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    begin(out, guard, m)?;
    status_return(out, status, result, m)?;
    end(out, Region::RuntimeGuard, m)
}
fn valid(out: &mut Vec<Event>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    failure(out, Guard::PointerValidity, 255, 255, m)?;
    read(out, Expr::address(0, -8), 4, m)?;
    failure(out, Guard::HeaderLive, 255, 255, m)
}
fn control(out: &mut Vec<Event>, m: &Model<'_, '_, '_>, pending: bool) -> Result<(), Diagnostic> {
    valid(out, m)?;
    read(out, Expr::address(0, -12), 4, m)?;
    read(out, Expr::address(0, -4), 4, m)?;
    failure(out, Guard::ControlFlags(pending), 255, 255, m)
}
fn free(out: &mut Vec<Event>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    store(out, Expr::address(0, -8), Expr::Const(0), 4, m)?;
    push(out, Event::LiveOwners(Expr::GlobalDelta(8, -1)), m)
}
fn decrement(out: &mut Vec<Event>, offset: i64, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    read(out, Expr::address(0, offset), 4, m)?;
    store(out, Expr::address(0, offset), Expr::Deref(0, offset, -1), 4, m)
}
fn vector(out: &mut Vec<Event>, m: &Model<'_, '_, '_>, allocate: bool) -> Result<(), Diagnostic> {
    let scratch = i64::from(m.memory.claims().scratch_start);
    if allocate {
        for offset in [0, 4, 8] {
            store(out, Expr::address(2, offset), Expr::Const(0), 4, m)?;
        }
        push(out, Event::Empty, m)?;
        returned(out, Expr::Const(0), m)?;
        end(out, Region::Empty, m)?;
    } else {
        push(out, Event::Length, m)?;
        status_return(out, 255, 255, m)?;
        end(out, Region::Length, m)?;
    }
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|v| v.target() == StorageTarget::Linear32V1) {
        push(out, Event::Element(i64::from(row.element().index())), m)?;
        let capacity =
            (zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES - 16) / row.stride();
        push(out, Event::Capacity(crate::generic_execution_v1::checked_i64(capacity)?), m)?;
        status_return(out, 3, 2, m)?;
        end(out, Region::Capacity, m)?;
        let stride = crate::generic_execution_v1::checked_i64(row.stride())?;
        let align = Expr::Const(crate::generic_execution_v1::checked_i64(row.alignment())?);
        if allocate {
            call(out, m.container(C::ALLOCATE_RAW), &[Expr::Scaled(1, stride), align], m)?;
            store(
                out,
                Expr::Const(scratch),
                Expr::PackedCallResult(m.container(C::ALLOCATE_RAW)),
                8,
                m,
            )?;
            read(out, Expr::Const(scratch + 4), 4, m)?;
            push(out, Event::AllocationStatus, m)?;
            read(out, Expr::Const(scratch + 4), 4, m)?;
            returned(out, Expr::AbsoluteLoad(scratch + 4, 4), m)?;
            end(out, Region::AllocationStatus, m)?;
            read(out, Expr::Const(scratch), 4, m)?;
            store(out, Expr::Local(2), Expr::AbsoluteLoad(scratch, 4), 4, m)?;
            store(out, Expr::address(2, 8), Expr::Local(1), 4, m)?;
            returned(out, Expr::Const(0), m)?;
        } else {
            call(
                out,
                m.container(C::RELEASE_RAW),
                &[Expr::Local(1), Expr::Scaled(3, stride), align],
                m,
            )?;
            returned(out, Expr::CallResult(m.container(C::RELEASE_RAW)), m)?;
        }
        end(out, Region::Element, m)?;
    }
    status_return(out, 255, 255, m)
}
