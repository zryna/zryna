//! Reverse successful String occurrences, guarded before reading inactive payload lanes.
use super::{Bytes, CloneNodeKind, Context, Node, Op, Place, active, budget, ranges};
use wasm_encoder::BlockType;
use zryna_diagnostics::Diagnostic;

pub(super) fn emit(
    body: &mut Bytes,
    context: &Context<'_, '_, '_>,
    node: Node<'_>,
    place: Place,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => {}
        CloneNodeKind::String => {
            body.op(&Op::LocalGet(context.cursor))?;
            body.op(&Op::I32Const(
                i32::try_from(place.base.checked_add(1).ok_or_else(budget)?)
                    .map_err(|_| budget())?,
            ))?;
            body.op(&Op::I32GeU)?;
            body.op(&Op::If(BlockType::Empty))?;
            for lane in 0..3 {
                body.op(&Op::LocalGet(
                    place.destination.start.checked_add(lane).ok_or_else(budget)?,
                ))?;
            }
            body.op(&Op::Call(2))?;
            body.op(&Op::If(BlockType::Empty))?;
            body.op(&Op::Unreachable)?;
            body.op(&Op::End)?;
            body.op(&Op::End)?;
        }
        CloneNodeKind::Struct => {
            for (child, child_place) in ranges::fields(context, node, place)?.into_iter().rev() {
                emit(body, context, child, child_place)?;
            }
        }
        CloneNodeKind::Enum => {
            for (ordinal, relative, child) in node.variants() {
                active(body, place.source, ordinal)?;
                if let Some(child) = child {
                    emit(body, context, child, ranges::payload(context, child, place, relative)?)?;
                }
                body.op(&Op::End)?;
            }
        }
    }
    Ok(())
}
