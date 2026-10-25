//! Type helper obligations retain actual source Clone capability and exact physical child slots.
use super::{Diagnostic, Event, Expr, Model, Region, call, copy, invariant, push, read, store};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
pub(super) fn events(
    index: usize,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    if index < 5 + m.types.len() {
        cloned(index - 5, m, out)
    } else {
        dropped(index - 5 - m.types.len(), m, out)
    }
}
fn cloned(ty: usize, m: &Model<'_, '_, '_>, out: &mut Vec<Event>) -> Result<(), Diagnostic> {
    let n = m.types.len();

    let Some(node) = m.clones[ty] else {
        push(out, Event::Trap, m)?;
        return Ok(());
    };
    let t = m.types[ty];
    match node.kind() {
        CloneNodeKind::Copy => copy(
            out,
            Expr::Local(0),
            Expr::Local(1),
            crate::generic_execution_v1::checked_u32(t.size())?,
            m,
        )?,
        CloneNodeKind::String => {
            for offset in [0, 4, 8] {
                read(out, Expr::address(0, offset), 4, m)?;
            }
            call(
                out,
                m.string_call(2),
                &[Expr::Load(0, 0, 4), Expr::Load(0, 4, 4), Expr::Load(0, 8, 4), Expr::Local(1)],
                m,
            )?;
        }
        CloneNodeKind::Enum => {
            m.take(node.variants().len())?;
            for (ordinal, child) in node.variants() {
                read(out, Expr::Local(0), 4, m)?;
                push(out, Event::Tag { base: 0, offset: 0, ordinal: i64::from(ordinal) }, m)?;
                if let Some(child) = child {
                    let offset = crate::generic_execution_v1::checked_i64(
                        t.payload().ok_or_else(invariant)?.0,
                    )?;
                    call(
                        out,
                        5 + child.ty(),
                        &[Expr::address(0, offset), Expr::address(1, offset)],
                        m,
                    )?;
                    push(out, Event::ChildFailure, m)?;
                    push(out, Event::ChildReturn, m)?;
                    push(out, Event::End(Region::Child), m)?;
                }
                store(out, Expr::Local(1), Expr::Const(i64::from(ordinal)), 4, m)?;
                push(out, Event::End(Region::Tag), m)?;
            }
            push(out, Event::Trap, m)?;
        }
        CloneNodeKind::Struct => structure(node, t, m, out)?,
        CloneNodeKind::FixedArray { .. } => {
            let child = node.element().ok_or_else(invariant)?.ty();
            let stride =
                crate::generic_execution_v1::checked_i64(t.array_stride().ok_or_else(invariant)?)?;
            call(
                out,
                5 + child,
                &[Expr::Indexed(0, 0, 3, stride), Expr::Indexed(1, 0, 3, stride)],
                m,
            )?;
            push(out, Event::ChildFailure, m)?;
            call(
                out,
                5 + crate::generic_execution_v1::checked_u32(n)? + child,
                &[Expr::Indexed(1, 0, 3, stride)],
                m,
            )?;
            push(out, Event::ChildReturn, m)?;
            push(out, Event::End(Region::Child), m)?;
        }
        CloneNodeKind::VecEach | CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => {
            super::helpers_containers::cloned(ty, m, out)?;
        }
    }
    Ok(())
}
fn dropped(ty: usize, m: &Model<'_, '_, '_>, out: &mut Vec<Event>) -> Result<(), Diagnostic> {
    let n = m.types.len();

    let t = *m.types.get(ty).ok_or_else(invariant)?;
    match t.category() {
        TypeCategory::Bool | TypeCategory::I32 => {}
        TypeCategory::String => {
            for offset in [0, 4, 8] {
                read(out, Expr::address(0, offset), 4, m)?;
            }
            call(
                out,
                m.string_call(3),
                &[Expr::Load(0, 0, 4), Expr::Load(0, 4, 4), Expr::Load(0, 8, 4)],
                m,
            )?;
            push(out, Event::ReleaseFailure, m)?;
            push(out, Event::Trap, m)?;
            push(out, Event::End(Region::Release), m)?;
            for offset in [0, 4, 8] {
                store(out, Expr::address(0, offset), Expr::Const(0), 4, m)?;
            }
        }
        TypeCategory::Struct => {
            for ordinal in (0..t.fields().len()).rev() {
                m.take(ordinal + 1)?;
                let (_, child, offset) = t.fields().nth(ordinal).ok_or_else(invariant)?;
                call(
                    out,
                    5 + crate::generic_execution_v1::checked_u32(n)? + child.index(),
                    &[Expr::address(0, crate::generic_execution_v1::checked_i64(offset)?)],
                    m,
                )?;
            }
        }
        TypeCategory::Enum => {
            m.take(t.variants().len())?;
            for (ordinal, child) in t.variants() {
                read(out, Expr::Local(0), 4, m)?;
                push(out, Event::Tag { base: 0, offset: 0, ordinal: i64::from(ordinal) }, m)?;
                if let Some(child) = child {
                    call(
                        out,
                        5 + crate::generic_execution_v1::checked_u32(n)? + child.index(),
                        &[Expr::address(
                            0,
                            crate::generic_execution_v1::checked_i64(
                                t.payload().ok_or_else(invariant)?.0,
                            )?,
                        )],
                        m,
                    )?;
                }
                push(out, Event::End(Region::Tag), m)?;
            }
            push(out, Event::Trap, m)?;
        }
        TypeCategory::FixedArray => call(
            out,
            5 + crate::generic_execution_v1::checked_u32(n)?
                + t.referenced_type().ok_or_else(invariant)?.index(),
            &[Expr::Indexed(
                0,
                0,
                1,
                crate::generic_execution_v1::checked_i64(t.array_stride().ok_or_else(invariant)?)?,
            )],
            m,
        )?,
        TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => {
            super::helpers_containers::dropped(ty, m, out)?;
        }
    }
    Ok(())
}

fn structure(
    node: zryna_ir::generic_v1::execution_v1::VerifiedCloneNode<'_, '_>,
    t: zryna_layout::generic_v1::TypeView<'_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let n = m.types.len();

    m.take(node.fields().len())?;
    for (ordinal, child) in node.fields() {
        m.take(ordinal as usize + 1)?;
        let offset = crate::generic_execution_v1::checked_i64(
            t.fields().nth(ordinal as usize).ok_or_else(invariant)?.2,
        )?;
        call(out, 5 + child.ty(), &[Expr::address(0, offset), Expr::address(1, offset)], m)?;
        push(out, Event::ChildFailure, m)?;
        for prev in (0..ordinal).rev() {
            m.take(prev as usize + 1)?;
            let (_, ty, offset) = t.fields().nth(prev as usize).ok_or_else(invariant)?;
            call(
                out,
                5 + crate::generic_execution_v1::checked_u32(n)? + ty.index(),
                &[Expr::address(1, crate::generic_execution_v1::checked_i64(offset)?)],
                m,
            )?;
        }
        push(out, Event::ChildReturn, m)?;
        push(out, Event::End(Region::Child), m)?;
    }
    Ok(())
}
