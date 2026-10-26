//! Physical inline-record transport preserves leaf handles; it grants no Clone/Copy authority.
use super::super::{Context, Writer, context::charge, internal};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::raw;
use zryna_layout::{TypeCategory, generic_v1::TypeView};

fn inline(ty: TypeView<'_>) -> bool {
    matches!(ty.category(), TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray)
}
pub(in crate::generic_execution_v1) fn begin(
    context: &Context<'_, '_>,
    ty: raw::ValueType,
    writer: &mut Writer,
) -> Result<bool, Diagnostic> {
    if let raw::ValueType::Stored(id) = ty
        && inline(context.ty(id)?)
    {
        writer.format(format_args!("$record{id}("))?;
        return Ok(true);
    }
    Ok(false)
}
fn source(
    ty: TypeView<'_>,
    expression: std::fmt::Arguments<'_>,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    if inline(ty) {
        writer.format(format_args!("$record{}(", ty.id().index()))?;
    }
    writer.format(expression)?;
    if inline(ty) {
        writer.text(")")?;
    }
    Ok(())
}
pub(super) fn declarations(
    context: &Context<'_, '_>,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    charge(&context.work, context.types.len())?;
    for ty in &context.types {
        if !inline(*ty) {
            continue;
        }
        writer.format(format_args!("function $record{}($s) {{\n", ty.id().index()))?;
        match ty.category() {
            TypeCategory::Struct => {
                writer.text("return $owner([")?;
                let fields = ty.fields();
                charge(&context.work, fields.len())?;
                for (index, (ordinal, child, _)) in fields.enumerate() {
                    if index != 0 {
                        writer.text(",")?;
                    }
                    source(context.ty(child.index())?, format_args!("$s[{ordinal}]"), writer)?;
                }
                writer.text("]);\n")?;
            }
            TypeCategory::Enum => {
                writer.text("switch($s.tag) {\n")?;
                let variants = ty.variants();
                charge(&context.work, variants.len())?;
                for (ordinal, payload) in variants {
                    writer.format(format_args!("case {ordinal}: return $enumOwner({ordinal},"))?;
                    if let Some(child) = payload {
                        source(context.ty(child.index())?, format_args!("$s.payload"), writer)?;
                    } else {
                        writer.text("undefined")?;
                    }
                    writer.text(");\n")?;
                }
                writer.text("default: throw new Error('ZRYNA-RT-TAG');\n}\n")?;
            }
            TypeCategory::FixedArray => {
                let length = ty.array_length().ok_or_else(internal)?;
                let child = context.ty(ty.referenced_type().ok_or_else(internal)?.index())?;
                writer.format(format_args!("const $out=$owner(new Array({length})); for(let $i=0;$i<{length};$i++) $out[$i]="))?;
                source(child, format_args!("$s[$i]"), writer)?;
                writer.text("; return $out;\n")?;
            }
            _ => return Err(internal()),
        }
        writer.text("}\n")?;
    }
    Ok(())
}

pub(in crate::generic_execution_v1) fn value(
    context: &Context<'_, '_>,
    function: &super::super::context::Function<'_, '_>,
    id: u32,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    let ty = *function.values.get(id as usize).ok_or_else(internal)?;
    let wrapped = begin(context, ty, writer)?;
    writer.format(format_args!("$v[{id}]"))?;
    if wrapped {
        writer.text(")")?;
    }
    Ok(())
}
