//! Target-relative ranges derive only from the opaque recipe and this retained layout.
use super::{Context, Node, Place, Value, budget, invariant};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::raw;

pub(super) fn fields<'a>(
    context: &Context<'_, '_, '_>,
    node: Node<'a>,
    place: Place,
) -> Result<Vec<(Node<'a>, Place)>, Diagnostic> {
    let mut ranges = Vec::new();
    ranges.try_reserve(node.fields().len()).map_err(|_| budget())?;
    let mut prefix = 0u32;
    for (ordinal, relative, child) in node.fields() {
        if usize::try_from(ordinal).map_err(|_| invariant())? != ranges.len() {
            return Err(invariant());
        }
        let width = context.layout.width(raw::Type::Stored(child.ty()))?;
        let end = prefix.checked_add(width).ok_or_else(budget)?;
        if end > place.source.width || end > place.destination.width {
            return Err(invariant());
        }
        let source =
            Value { start: place.source.start.checked_add(prefix).ok_or_else(budget)?, width };
        let destination =
            Value { start: place.destination.start.checked_add(prefix).ok_or_else(budget)?, width };
        let base = place
            .base
            .checked_add(u32::try_from(relative).map_err(|_| budget())?)
            .ok_or_else(budget)?;
        ranges.push((child, Place { source, destination, base }));
        prefix = end;
    }
    if prefix != place.source.width || prefix != place.destination.width {
        return Err(invariant());
    }
    Ok(ranges)
}

pub(super) fn payload(
    context: &Context<'_, '_, '_>,
    node: Node<'_>,
    place: Place,
    relative: u64,
) -> Result<Place, Diagnostic> {
    let width = context.layout.width(raw::Type::Stored(node.ty()))?;
    let end = width.checked_add(1).ok_or_else(budget)?;
    if end > place.source.width || end > place.destination.width {
        return Err(invariant());
    }
    Ok(Place {
        source: Value { start: place.source.start.checked_add(1).ok_or_else(budget)?, width },
        destination: Value {
            start: place.destination.start.checked_add(1).ok_or_else(budget)?,
            width,
        },
        base: place
            .base
            .checked_add(u32::try_from(relative).map_err(|_| budget())?)
            .ok_or_else(budget)?,
    })
}
