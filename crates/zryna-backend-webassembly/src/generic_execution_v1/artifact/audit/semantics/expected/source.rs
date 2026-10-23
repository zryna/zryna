//! Per-operation and own-exit obligations derived without any emitter body recipes.
use super::{
    Diagnostic, Event, Expr, Model, Region, cleanup, copy, frame, invariant, push, raw, read,
    referent, value,
};
use zryna_ir::generic_v1::execution_v1::VerifiedBlock;
pub(super) fn events(
    f: usize,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    m.take(m.functions[f].blocks().len())?;
    for block in m.functions[f].blocks() {
        push(out, Event::Block(i64::from(block.id())), m)?;
        m.take(block.operations().len())?;
        for op in block.operations() {
            super::operations::emit(op, m, out)?;
        }
        terminator(f, block, m, out)?;
        push(out, Event::End(Region::Block), m)?;
    }
    push(out, Event::Trap, m)?;
    push(out, Event::Trap, m)?;
    Ok(())
}
fn edge(
    f: usize,
    e: &raw::Edge,
    skip: usize,
    depth: u32,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    m.take(e.target as usize + 1)?;
    let target = m.functions[f].blocks().nth(e.target as usize).ok_or_else(invariant)?;
    m.take(e.arguments.len())?;
    for (arg, param) in e.arguments.iter().zip(target.value_parameters().skip(skip)) {
        let src = value(m, f, *arg)?;
        copy(out, frame(src.offset), frame(value(m, f, param.id())?.offset), src.bytes, m)?;
    }
    push(out, Event::Edge(i64::from(e.target)), m)?;
    push(out, Event::Jump(depth), m)
}
fn terminator(
    f: usize,
    block: VerifiedBlock<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    match block.terminator() {
        raw::Terminator::Return { value: v, .. } => {
            if let Some(v) = v {
                let src = value(m, f, *v)?;
                copy(out, frame(src.offset), Expr::Local(1), src.bytes, m)?;
            }
            cleanup(out, block.exit_cleanup().ok_or_else(invariant)?, m)?;
            push(out, Event::SuccessReturn, m)?;
        }
        raw::Terminator::Trap { identity, .. } => {
            cleanup(out, block.exit_cleanup().ok_or_else(invariant)?, m)?;
            let n = match identity {
                raw::TrapIdentity::Bounds => 1,
                raw::TrapIdentity::Allocation => 2,
                raw::TrapIdentity::Capacity => 3,
                raw::TrapIdentity::Refcount => 4,
                raw::TrapIdentity::Utf8 => 5,
            };
            push(out, Event::Status(n), m)?;
            push(out, Event::TrapReturn(n), m)?;
        }
        raw::Terminator::Jump(e) => edge(f, e, 0, 1, m, out)?,
        raw::Terminator::Branch { condition, yes, no } => {
            read(out, frame(value(m, f, *condition)?.offset), 1, m)?;
            push(out, Event::Branch(i64::from(value(m, f, *condition)?.offset)), m)?;
            edge(f, yes, 0, 2, m, out)?;
            push(out, Event::End(Region::Branch), m)?;
            edge(f, no, 0, 1, m, out)?;
        }
        raw::Terminator::ClosedEnumMatch { ty, scrutinee, arms, .. } => {
            let src = referent(m, f, *scrutinee)?;
            m.take(arms.len())?;
            for arm in arms {
                read(out, frame(src.offset), 4, m)?;
                push(
                    out,
                    Event::Tag {
                        base: 0,
                        offset: i64::from(src.offset),
                        ordinal: i64::from(arm.ordinal),
                    },
                    m,
                )?;
                let skip = if let Some(binding) = &arm.binding {
                    let raw::MatchBinding::Value(v) = binding else { return Err(invariant()) };
                    let dst = value(m, f, v.id)?;
                    copy(
                        out,
                        frame(
                            src.offset
                                + crate::generic_execution_v1::checked_u32(
                                    m.types[*ty as usize].payload().ok_or_else(invariant)?.0,
                                )?,
                        ),
                        frame(dst.offset),
                        dst.bytes,
                        m,
                    )?;
                    1
                } else {
                    0
                };
                edge(f, &arm.edge, skip, 2, m, out)?;
                push(out, Event::End(Region::Tag), m)?;
            }
            push(out, Event::Trap, m)?;
        }
        raw::Terminator::WeakUpgradeBranch { .. } => return Err(invariant()),
    }
    Ok(())
}
