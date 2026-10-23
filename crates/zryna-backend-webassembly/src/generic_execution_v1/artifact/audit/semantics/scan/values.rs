//! Fixed-address, scalar-store and exact index-source provenance from decoded operands.
use super::{Diagnostic, Event, Expr, Model, O, State, invariant, push};
pub(super) fn step(
    s: &mut State,
    op: &O<'_>,
    events: &mut Vec<Event>,
    m: &Model<'_, '_, '_>,
) -> Result<bool, Diagnostic> {
    match *op {
        O::I64Const { value } => s.push(Expr::Const(value))?,
        O::I64ExtendI32U => {
            let value = s.pop()?;
            s.push(if let Expr::Local(local) = value {
                Expr::UnsignedLocal(local)
            } else {
                Expr::Unknown
            })?;
        }
        O::I32Const { value } => s.push(Expr::Const(i64::from(value)))?,
        O::LocalGet { local_index } => {
            s.push(if (s.source() && local_index == 4) || (s.shared() && local_index == 1) {
                s.alias
            } else {
                Expr::Local(local_index)
            })?;
        }
        O::GlobalGet { global_index } => s.push(Expr::Global(global_index))?,
        O::LocalSet { local_index } => {
            let value = s.pop()?;
            if (s.source() && local_index == 4) || (s.shared() && local_index == 1) {
                s.alias = value;
            }
            if s.source() && local_index == 2 {
                let Expr::Const(target) = value else { return Err(invariant()) };
                push(events, Event::Edge(target), m)?;
            }
        }
        O::LocalTee { local_index } => {
            let value = s.pop()?;
            if ((s.source() && local_index == 3) || (s.child() && local_index == 2))
                && (s.last_call || value == Expr::PackedStatus)
            {
                s.pending = true;
            }
            if s.source() && local_index == 5 {
                push(events, Event::Index(value), m)?;
            }
            s.push(Expr::Local(local_index))?;
        }
        O::GlobalSet { global_index } => {
            let value = s.pop()?;
            if global_index == 8 && s.track_live() {
                push(events, Event::LiveOwners(value), m)?;
            }
            if global_index == 1 {
                let Expr::Const(status) = value else { return Err(invariant()) };
                s.status = Some(value);
                push(events, Event::Status(status), m)?;
            }
        }
        O::I32Load { memarg } => {
            let address = s.pop()?;
            if s.read {
                push(
                    events,
                    Event::Read {
                        address: address.add(Expr::Const(memarg.offset.cast_signed())),
                        width: 4,
                    },
                    m,
                )?;
            }
            let value = if s.source()
                && address == Expr::Const(i64::from(m.memory.claims().scratch_start + 4))
            {
                Expr::PackedStatus
            } else {
                address.load(memarg.offset, 4)
            };
            s.push(value)?;
        }
        O::I32Load8U { memarg } => {
            let address = s.pop()?;
            if s.read {
                push(
                    events,
                    Event::Read {
                        address: address.add(Expr::Const(memarg.offset.cast_signed())),
                        width: 1,
                    },
                    m,
                )?;
            }
            let value = address.load(memarg.offset, 1);
            s.push(value)?;
        }
        O::I32Store { memarg } | O::I32Store8 { memarg } | O::I64Store { memarg } => {
            stored(s, op, memarg, events, m)?;
        }
        _ => return numeric(s, op),
    }
    Ok(true)
}

fn arithmetic(op: &O<'_>, left: Expr, right: Expr, source: bool, vector: bool) -> Expr {
    if vector
        && matches!(op, O::I32Eq)
        && left == Expr::Local(0)
        && let Expr::Const(n) = right
    {
        return Expr::Element(n);
    }
    match *op {
        O::I32GtU if vector => match (left, right) {
            (Expr::Local(1 | 3), Expr::Const(n)) => Expr::Capacity(n),
            (Expr::Local(2), Expr::Local(3)) => Expr::Length,
            _ => Expr::Unknown,
        },
        O::I32Add => left.add(right),
        O::I32Sub => left.sub(right),
        O::I32Mul => left.mul(right),
        O::I32Eq => left.eq(right),
        O::I32GeU if source && left == Expr::Local(5) => {
            if let Expr::Const(n) = right {
                Expr::Bounds(n)
            } else if let Expr::Load(base, offset, 4) = right {
                Expr::VecBounds(base, offset)
            } else {
                Expr::Unknown
            }
        }
        _ => Expr::Unknown,
    }
}

fn stored(
    s: &mut State,
    op: &O<'_>,
    memarg: wasmparser::MemArg,
    events: &mut Vec<Event>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let value = s.pop()?;
    let address = s.pop()?.add(Expr::Const(memarg.offset.cast_signed()));
    push(
        events,
        Event::Store {
            address,
            value,
            width: if matches!(op, O::I32Store8 { .. }) {
                1
            } else if matches!(op, O::I64Store { .. }) {
                8
            } else {
                4
            },
        },
        m,
    )?;
    Ok(())
}

fn numeric(s: &mut State, op: &O<'_>) -> Result<bool, Diagnostic> {
    match *op {
        O::I32Add | O::I32Sub | O::I32Mul | O::I32Eq | O::I32GeU | O::I32GtU => {
            let right = s.pop()?;
            let left = s.pop()?;
            let value = arithmetic(op, left, right, s.source(), s.vector());
            s.push(value)?;
        }
        O::I32Eqz => {
            let value = s.pop()?;
            s.push(if s.counts() && matches!(value, Expr::Load(0, 0 | 4, 4)) {
                let Expr::Load(_, offset, _) = value else {
                    return Err(invariant());
                };
                Expr::ControlZero(offset)
            } else if s.counts() && value == Expr::ControlPair {
                Expr::NoOwners
            } else if s.vector() && value == Expr::Local(1) {
                Expr::Empty
            } else {
                Expr::Unknown
            })?;
        }
        O::I32Or if s.counts() => {
            let right = s.pop()?;
            let left = s.pop()?;
            s.push(if matches!((left, right), (Expr::Load(0, 0, 4), Expr::Load(0, 4, 4))) {
                Expr::ControlPair
            } else {
                Expr::Unknown
            })?;
        }
        O::I32Ne
        | O::I32LtS
        | O::I32LeS
        | O::I32GtS
        | O::I32GeS
        | O::I32LtU
        | O::I32Or
        | O::I32And
        | O::I32ShrU
        | O::I64Or
        | O::I64Shl
        | O::I32LeU => {
            s.pop()?;
            s.pop()?;
            s.push(Expr::Unknown)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
