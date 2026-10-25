//! Dead private carriers retain their continuation before descending; release creates no worklist.
use super::{Context, Writer, charge, fields, internal};
use std::fmt::Arguments;
use zryna_diagnostics::Diagnostic;
use zryna_layout::{TypeCategory, generic_v1::TypeView};
mod containers;

pub(super) fn declarations(c: &Context<'_, '_>, w: &mut Writer) -> Result<(), Diagnostic> {
    // Every aggregate producer calls this before publishing its private carrier. These
    // properties are separate from canonical String/storage tokens and control headers.
    w.text("function $owner($s) { Object.setPrototypeOf($s,null); $s.$dropParent=null; $s.$dropType=-1; $s.$dropIndex=-1; return $s; }\n")?;
    w.text("function $dropLoop($s,$type,$ctx) { let $parent=null,$index=-1; for(;;) { switch($type) {\n")?;
    charge(&c.work, c.types.len())?;
    for ty in &c.types {
        if ty.drop_kind() == 0 {
            continue;
        }
        w.format(format_args!("case {}: {{\n", ty.id().index()))?;
        body(c, *ty, w)?;
        w.text("break; }\n")?;
    }
    w.text("default: throw new Error('ZRYNA-RT-TAG');\n}\nif($parent===null) return;\nconst $resume=$parent; $parent=$resume.$dropParent; $type=$resume.$dropType; $index=$resume.$dropIndex; $s=$resume;\n$resume.$dropParent=null; $resume.$dropType=-1; $resume.$dropIndex=-1;\n} }\n")
}

fn body(c: &Context<'_, '_>, ty: TypeView<'_>, w: &mut Writer) -> Result<(), Diagnostic> {
    match ty.category() {
        TypeCategory::String => {
            w.text("$free($s")?;
            super::super::runtime::accounting::argument(c, w, true)?;
            w.text(");\n")
        }
        TypeCategory::Struct => {
            w.format(format_args!("if($index<0) $index={};\n", ty.fields().len()))?;
            for (ordinal, child) in fields(ty, c)?.into_iter().rev() {
                if c.ty(child)?.drop_kind() != 0 {
                    w.format(format_args!("if($index>{ordinal}) {{\n"))?;
                    descend(w, child, format_args!("$s[{ordinal}]"), format_args!("{ordinal}"))?;
                    w.text("}\n")?;
                }
            }
            Ok(())
        }
        TypeCategory::Enum => {
            w.text("if($index<0) { switch($s.tag) {\n")?;
            charge(&c.work, ty.variants().len())?;
            for (ordinal, child) in ty.variants() {
                w.format(format_args!("case {ordinal}: "))?;
                if let Some(child) = child
                    && c.ty(child.index())?.drop_kind() != 0
                {
                    descend(w, child.index(), format_args!("$s.payload"), format_args!("0"))?;
                } else {
                    w.text("break;\n")?;
                }
            }
            super::invalid_tag(w)?;
            w.text("}\n")
        }
        TypeCategory::FixedArray => {
            let child = ty.referenced_type().ok_or_else(internal)?.index();
            let length = ty.array_length().ok_or_else(internal)?;
            w.format(format_args!("if($index<0) $index={length};\n"))?;
            if c.ty(child)?.drop_kind() != 0 {
                w.text("if($index>0) { --$index;\n")?;
                descend(w, child, format_args!("$s[$index]"), format_args!("$index"))?;
                w.text("}\n")?;
            }
            Ok(())
        }
        TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => containers::body(c, ty, w),
        _ => Err(internal()),
    }
}

pub(super) fn descend(
    w: &mut Writer,
    child: u32,
    value: Arguments<'_>,
    next: Arguments<'_>,
) -> Result<(), Diagnostic> {
    w.format(format_args!("$s.$dropParent=$parent; $s.$dropType=$type; $s.$dropIndex={next};\n$parent=$s; $s={value}; $type={child}; $index=-1; continue;\n"))
}
