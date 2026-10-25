//! Exact dynamic record data, child frontier, rollback and count-terminal obligations.
use super::{Diagnostic, Event, Expr, Model, Region, call, invariant, push, read, store};
use crate::generic_execution_v1::model::containers as C;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
pub(super) fn cloned(
    ty: usize,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let node = m.clones[ty].ok_or_else(invariant)?;
    if node.kind() == CloneNodeKind::VecEach {
        let meta = m.element(crate::generic_execution_v1::checked_u32(ty)?)?;
        let child = meta.element().index();
        let stride = crate::generic_execution_v1::checked_i64(meta.stride())?;
        read(out, Expr::address(0, 4), 4, m)?;
        call(
            out,
            m.container(C::VEC_ALLOCATE),
            &[Expr::Const(i64::from(child)), Expr::Load(0, 4, 4), Expr::Local(1)],
            m,
        )?;
        child_return(out, m)?;
        read(out, Expr::address(0, 4), 4, m)?;
        read(out, Expr::Local(0), 4, m)?;
        read(out, Expr::Local(1), 4, m)?;
        call(
            out,
            m.clone_index(child),
            &[Expr::IndirectIndexed(0, 0, 3, stride), Expr::IndirectIndexed(1, 0, 3, stride)],
            m,
        )?;
        push(out, Event::ChildFailure, m)?;
        read(out, Expr::Local(1), 4, m)?;
        call(out, m.drop_index(child), &[Expr::IndirectIndexed(1, 0, 3, stride)], m)?;
        release_vec(ty, 1, m, out)?;
        push(out, Event::ChildReturn, m)?;
        push(out, Event::End(Region::Child), m)?;
        store(out, Expr::address(1, 4), Expr::Local(3), 4, m)?;
    } else {
        let operation = match node.kind() {
            CloneNodeKind::SharedCount => C::STRONG_CLONE,
            CloneNodeKind::WeakCount => C::WEAK_CLONE,
            _ => return Err(invariant()),
        };
        read(out, Expr::Local(0), 4, m)?;
        call(out, m.container(operation), &[Expr::Load(0, 0, 4)], m)?;
        child_return(out, m)?;
        read(out, Expr::Local(0), 4, m)?;
        store(out, Expr::Local(1), Expr::Load(0, 0, 4), 4, m)?;
    }
    Ok(())
}
fn child_return(out: &mut Vec<Event>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    push(out, Event::ChildFailure, m)?;
    push(out, Event::ChildReturn, m)?;
    push(out, Event::End(Region::Child), m)
}
fn checked(out: &mut Vec<Event>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    push(out, Event::ReleaseFailure, m)?;
    push(out, Event::Trap, m)?;
    push(out, Event::End(Region::Release), m)
}
fn release_vec(
    ty: usize,
    local: u32,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    for offset in [0, 4, 8] {
        read(out, Expr::address(local, offset), 4, m)?;
    }
    call(
        out,
        m.container(C::VEC_RELEASE),
        &[
            Expr::Const(i64::from(
                m.element(crate::generic_execution_v1::checked_u32(ty)?)?.element().index(),
            )),
            Expr::Load(local, 0, 4),
            Expr::Load(local, 4, 4),
            Expr::Load(local, 8, 4),
        ],
        m,
    )?;
    checked(out, m)?;
    for offset in [0, 4, 8] {
        store(out, Expr::address(local, offset), Expr::Const(0), 4, m)?;
    }
    Ok(())
}
pub(super) fn dropped(
    ty: usize,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let scratch = i64::from(m.memory.claims().scratch_start);
    match m.types[ty].category() {
        TypeCategory::Vec => {
            let meta = m.element(crate::generic_execution_v1::checked_u32(ty)?)?;
            read(out, Expr::address(0, 4), 4, m)?;
            read(out, Expr::Local(0), 4, m)?;
            call(
                out,
                m.drop_index(meta.element().index()),
                &[Expr::IndirectIndexed(
                    0,
                    0,
                    1,
                    crate::generic_execution_v1::checked_i64(meta.stride())?,
                )],
                m,
            )?;
            release_vec(ty, 0, m, out)?;
        }
        TypeCategory::Shared => {
            read(out, Expr::Local(0), 4, m)?;
            call(
                out,
                m.container(C::STRONG_BEGIN),
                &[Expr::Load(0, 0, 4), Expr::Const(scratch)],
                m,
            )?;
            checked(out, m)?;
            read(out, Expr::Const(scratch), 4, m)?;
            push(out, Event::LastStrong, m)?;
            read(out, Expr::Local(0), 4, m)?;
            let meta = m.control(crate::generic_execution_v1::checked_u32(ty)?)?;
            call(
                out,
                m.drop_index(meta.payload().index()),
                &[Expr::Deref(
                    0,
                    0,
                    crate::generic_execution_v1::checked_i64(meta.payload_offset())?,
                )],
                m,
            )?;
            read(out, Expr::Local(0), 4, m)?;
            call(out, m.container(C::STRONG_FINISH), &[Expr::Load(0, 0, 4)], m)?;
            checked(out, m)?;
            push(out, Event::End(Region::LastStrong), m)?;
            store(out, Expr::Local(0), Expr::Const(0), 4, m)?;
        }
        TypeCategory::Weak => {
            read(out, Expr::Local(0), 4, m)?;
            call(
                out,
                m.container(C::WEAK_RELEASE),
                &[Expr::Load(0, 0, 4), Expr::Const(scratch)],
                m,
            )?;
            checked(out, m)?;
            store(out, Expr::Local(0), Expr::Const(0), 4, m)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
