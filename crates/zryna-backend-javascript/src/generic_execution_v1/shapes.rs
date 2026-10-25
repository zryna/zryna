//! Type-directed representation work after operation/source capability authorization.
use super::{Context, Writer, context::charge, internal};
use zryna_diagnostics::Diagnostic;
mod clones;
pub(super) mod records;
mod release;
use zryna_layout::{TypeCategory, generic_v1::TypeView};

pub(super) fn declarations(c: &Context<'_, '_>, w: &mut Writer) -> Result<(), Diagnostic> {
    release::declarations(c, w)?;
    clones::declarations(c, w)?;
    for (id, ty) in c.types.iter().enumerate() {
        charge(&c.work, 1)?;
        if ty.drop_kind() != 0 {
            w.format(format_args!("function $drop{id}($s"))?;
            super::runtime::accounting::argument(c, w, true)?;
            w.text(") {\n")?;
            w.format(format_args!("$dropLoop($s,{id}"))?;
            super::runtime::accounting::argument(c, w, true)?;
            w.text(");\n")?;
            w.text("}\n")?;
        }
        if c.copies[id] {
            w.format(format_args!("function $copy{id}($s) {{\n"))?;
            copy_body(c, *ty, w)?;
            w.text("}\n")?;
        }
        if c.clones[id].is_some() {
            w.format(format_args!("function $clone{id}($s"))?;
            super::runtime::accounting::argument(c, w, true)?;
            w.text(") {\n")?;
            w.format(format_args!("return $cloneLoop($s,{id}"))?;
            super::runtime::accounting::argument(c, w, true)?;
            w.text(");\n")?;
            w.text("}\n")?;
        }
    }
    records::declarations(c, w)
}
fn fields(ty: TypeView<'_>, c: &Context<'_, '_>) -> Result<Vec<(u32, u32)>, Diagnostic> {
    charge(&c.work, ty.fields().len().checked_mul(8).ok_or_else(super::emission_budget)?)?;
    let mut fields = Vec::new();
    fields.try_reserve_exact(ty.fields().len()).map_err(|_| super::emission_budget())?;
    fields.extend(ty.fields().map(|(ordinal, ty, _)| (ordinal, ty.index())));
    Ok(fields)
}
fn copy_body(c: &Context<'_, '_>, ty: TypeView<'_>, w: &mut Writer) -> Result<(), Diagnostic> {
    match ty.category() {
        TypeCategory::Bool | TypeCategory::I32 => w.text("return $s;\n")?,
        TypeCategory::Struct => {
            w.text("return $owner([")?;
            for (position, (ordinal, child)) in fields(ty, c)?.into_iter().enumerate() {
                if position != 0 {
                    w.text(",")?;
                }
                w.format(format_args!("$copy{child}($s[{ordinal}])"))?;
            }
            w.text("]);\n")?;
        }
        TypeCategory::Enum => {
            w.text("switch ($s.tag) {\n")?;
            for (ordinal, child) in ty.variants() {
                charge(&c.work, 1)?;
                w.format(format_args!("case {ordinal}: return $owner({{tag:{ordinal},payload:"))?;
                if let Some(child) = child {
                    w.format(format_args!("$copy{}($s.payload)", child.index()))?;
                } else {
                    w.text("undefined")?;
                }
                w.text("});\n")?;
            }
            invalid_tag(w)?;
        }
        TypeCategory::FixedArray => {
            let child = ty.referenced_type().ok_or_else(internal)?.index();
            let length = ty.array_length().ok_or_else(internal)?;
            w.format(format_args!("const $out=$owner([]); for(let $i=0;$i<{length};$i++) $out[$i]=$copy{child}($s[$i]); return $out;\n"))?;
        }
        _ => return Err(internal()),
    }
    Ok(())
}
fn invalid_tag(w: &mut Writer) -> Result<(), Diagnostic> {
    w.text("default: throw new Error('ZRYNA-RT-TAG');\n}\n")
}
