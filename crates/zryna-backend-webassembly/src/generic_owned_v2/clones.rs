//! V5 clone preparation and a single local initialized-prefix failure region.
use super::{
    budget,
    bytes::Bytes,
    invariant,
    layout::{Layout, Locals, Value},
    runtime,
};
use wasm_encoder::{BlockType, Instruction as Op};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::clone_v5::{
    CloneNodeKind, VerifiedCloneNode as Node, VerifiedStructClone,
};

mod ranges;
mod rollback;
mod work;
pub(super) use work::preflight;

#[cfg(test)]
mod budget_tests;

struct Context<'l, 'p, 'a> {
    layout: &'l Layout<'p, 'a>,
    locals: &'l Locals,
    cursor: u32,
}
#[derive(Clone, Copy)]
struct Place {
    source: Value,
    destination: Value,
    base: u32,
}

pub(super) fn emit(
    body: &mut Bytes,
    layout: &Layout<'_, '_>,
    locals: &Locals,
    clone: VerifiedStructClone<'_>,
) -> Result<(), Diagnostic> {
    let source = *locals.values.get(clone.source() as usize).ok_or_else(invariant)?;
    let destination = *locals.values.get(clone.result() as usize).ok_or_else(invariant)?;
    if source.width != destination.width
        || destination.width != layout.width(zryna_ir::generic_v1::raw::Type::Stored(clone.ty()))?
    {
        return Err(invariant());
    }
    let cursor = if clone.string_occurrences() == 0 {
        0
    } else {
        locals.clone_cursor.ok_or_else(invariant)?
    };
    let context = Context { layout, locals, cursor };
    let place = Place { source, destination, base: 0 };
    if clone.string_occurrences() != 0 {
        body.op(&Op::I32Const(0))?;
        body.op(&Op::LocalSet(cursor))?;
        body.op(&Op::I32Const(0))?;
        body.op(&Op::LocalSet(locals.status))?;
    }
    for lane in 0..destination.width {
        body.op(&Op::I32Const(0))?;
        body.op(&Op::LocalSet(destination.start.checked_add(lane).ok_or_else(budget)?))?;
    }
    body.op(&Op::Block(BlockType::Empty))?;
    forward(body, &context, clone.root(), place, 0)?;
    body.op(&Op::End)?;
    if clone.string_occurrences() != 0 {
        body.op(&Op::LocalGet(locals.status))?;
        body.op(&Op::If(BlockType::Empty))?;
        rollback::emit(body, &context, clone.root(), place)?;
        body.op(&Op::End)?;
    }
    Ok(())
}

fn forward(
    body: &mut Bytes,
    context: &Context<'_, '_, '_>,
    node: Node<'_>,
    place: Place,
    depth: u32,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => {
            for lane in 0..place.source.width {
                body.op(&Op::LocalGet(place.source.start.checked_add(lane).ok_or_else(budget)?))?;
                body.op(&Op::LocalSet(
                    place.destination.start.checked_add(lane).ok_or_else(budget)?,
                ))?;
            }
        }
        CloneNodeKind::String => {
            if place.source.width != 3 || place.destination.width != 3 {
                return Err(invariant());
            }
            for lane in 0..3 {
                body.op(&Op::LocalGet(place.source.start.checked_add(lane).ok_or_else(budget)?))?;
            }
            runtime::record(body, context.locals.frame, 0)?;
            body.op(&Op::Call(1))?;
            body.op(&Op::LocalSet(context.locals.status))?;
            body.op(&Op::LocalGet(context.locals.status))?;
            body.op(&Op::BrIf(depth))?;
            // A failed call exits the enclosing clone block before any output-record load.
            for lane in 0..3 {
                runtime::load(body, context.locals.frame, lane)?;
                body.op(&Op::LocalSet(
                    place.destination.start.checked_add(lane).ok_or_else(budget)?,
                ))?;
            }
            body.op(&Op::I32Const(
                i32::try_from(place.base.checked_add(1).ok_or_else(budget)?)
                    .map_err(|_| budget())?,
            ))?;
            body.op(&Op::LocalSet(context.cursor))?;
        }
        CloneNodeKind::Struct => {
            for (child, child_place) in ranges::fields(context, node, place)? {
                forward(body, context, child, child_place, depth)?;
            }
        }
        CloneNodeKind::Enum => {
            body.op(&Op::LocalGet(place.source.start))?;
            body.op(&Op::LocalSet(place.destination.start))?;
            for (ordinal, relative, child) in node.variants() {
                active(body, place.source, ordinal)?;
                if let Some(child) = child {
                    forward(
                        body,
                        context,
                        child,
                        ranges::payload(context, child, place, relative)?,
                        depth.checked_add(1).ok_or_else(budget)?,
                    )?;
                }
                body.op(&Op::End)?;
            }
        }
    }
    Ok(())
}

fn active(body: &mut Bytes, source: Value, ordinal: u32) -> Result<(), Diagnostic> {
    body.op(&Op::LocalGet(source.start))?;
    body.op(&Op::I32Const(i32::try_from(ordinal).map_err(|_| budget())?))?;
    body.op(&Op::I32Eq)?;
    body.op(&Op::If(BlockType::Empty))
}
