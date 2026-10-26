//! Actual element prefixes and last-strong payloads precede unchanged canonical release leaves.
use super::{Context, Writer, descend, internal};
use zryna_diagnostics::Diagnostic;
use zryna_layout::{TypeCategory, generic_v1::TypeView};

pub(super) fn body(
    c: &Context<'_, '_>,
    ty: TypeView<'_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    let child = ty.referenced_type().ok_or_else(internal)?.index();
    match ty.category() {
        TypeCategory::Vec => {
            c.element(child)?;
            if c.ty(child)?.drop_kind() != 0 {
                w.text("if($index<0) $index=$s.$length;\nif($index>0) { --$index;\n")?;
                descend(w, child, format_args!("$s.$items[$index]"), format_args!("$index"))?;
                w.text("}\n")?;
            }
            if c.accounting.is_some() {
                w.format(format_args!("$vecStorageFree($el{child},$s.$storage,$ctx);\n"))
            } else {
                w.format(format_args!("$status_vecFree($vecFree($el{child},$s.$storage));\n"))
            }
        }
        TypeCategory::Shared => {
            c.control(child)?;
            w.text("if($index<0) { if(!")?;
            if c.accounting.is_some() {
                w.text("$accountStrongBegin($s,$ctx)")?;
            } else {
                w.text("$boolResult($strongBegin($s),$status_strongBegin)")?;
            }
            w.text(") break;\n")?;
            if c.ty(child)?.drop_kind() != 0 {
                descend(w, child, format_args!("$s.$payload"), format_args!("0"))?;
            }
            w.text("}\n$s.$payload=undefined;\n")?;
            if c.accounting.is_some() {
                w.text("$accountStrongFinish($s,$ctx);\n")
            } else {
                w.text("$status_strongFinish($strongFinish($s));\n")
            }
        }
        TypeCategory::Weak => {
            c.control(child)?;
            if c.accounting.is_some() {
                w.text("$accountWeakFree($s,$ctx);\n")
            } else {
                w.text("$boolResult($weakFree($s),$status_weakFree);\n")
            }
        }
        _ => Err(internal()),
    }
}
