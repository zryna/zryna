//! Reconstruct only missing inline ancestors of an independently checked empty target.
use super::super::{
    Context, Writer,
    context::{Function, charge},
    emission_budget, internal, shapes,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedPlace, raw};
use zryna_layout::TypeCategory;

pub(in crate::generic_execution_v1) fn emit(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    target: VerifiedPlace<'_, '_>,
    value: u32,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    charge(&context.work, 1)?;
    if target.parent().is_none() {
        // Keep the existing root initialization representation byte-for-byte.
        super::place(context, function, target, writer)?;
        writer.text("=")?;
        return shapes::records::value(context, function, value, writer);
    }
    writer.text("(() => {const $initial=")?;
    shapes::records::value(context, function, value, writer)?;
    writer.text(";\n")?;
    parents(context, function, target, writer, 0)?;
    super::place(context, function, target, writer)?;
    writer.text("=$initial; })()")
}

fn parents(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    child: VerifiedPlace<'_, '_>,
    writer: &mut Writer,
    depth: usize,
) -> Result<(), Diagnostic> {
    charge(&context.work, 1)?;
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(emission_budget());
    }
    let Some(parent) = child.parent() else {
        return Ok(());
    };
    parents(context, function, parent, writer, depth + 1)?;
    writer.text("if(")?;
    super::place(context, function, parent, writer)?;
    writer.text("===undefined) ")?;
    super::place(context, function, parent, writer)?;
    writer.text("=")?;
    let ty = context.ty(parent.ty())?;
    match ty.category() {
        TypeCategory::Struct => writer.text("$owner(Object.create(null))")?,
        TypeCategory::FixedArray => {
            charge(&context.work, 1)?;
            let length = ty.array_length().ok_or_else(internal)?;
            writer.format(format_args!(
                " $owner(Object.assign(Object.create(null),{{length:{length}}}))"
            ))?;
        }
        TypeCategory::Enum => {
            let raw::PlaceKind::EnumPayload { base, variant } = child.kind() else {
                return Err(internal());
            };
            if base != parent.id() {
                return Err(internal());
            }
            writer.format(format_args!(" $owner({{tag:{variant},payload:undefined}})"))?;
        }
        _ => return Err(internal()),
    }
    writer.text(";\n")
}

#[cfg(test)]
#[path = "initialize_tests.rs"]
mod tests;
