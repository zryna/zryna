//! Recursive checkpoint glue over finite sealed type graphs; no deep-stack receipt is implied.
use super::{Context, Writer, internal};
use zryna_diagnostics::Diagnostic;
use zryna_layout::{TypeCategory, generic_v1::TypeView};
pub(super) fn drop_body(
    c: &Context<'_, '_>,
    ty: TypeView<'_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    let child = ty.referenced_type().ok_or_else(internal)?.index();
    match ty.category() {
        TypeCategory::Vec => {
            c.element(child)?;
            if c.ty(child)?.drop_kind() != 0 {
                w.format(format_args!(
                    "for(let $i=$s.$length;$i>0;) $drop{child}($s.$items[--$i]);\n"
                ))?;
            }
            w.format(format_args!("$status_vecFree($vecFree($el{child},$s.$storage));\n"))
        }
        TypeCategory::Shared => {
            c.control(child)?;
            w.text("if($boolResult($strongBegin($s),$status_strongBegin)) {\n")?;
            if c.ty(child)?.drop_kind() != 0 {
                w.format(format_args!("$drop{child}($s.$payload);\n"))?;
            }
            w.text("$s.$payload=undefined; $status_strongFinish($strongFinish($s));\n}\n")
        }
        TypeCategory::Weak => {
            c.control(child)?;
            w.text("$boolResult($weakFree($s),$status_weakFree);\n")
        }
        _ => Err(internal()),
    }
}
pub(super) fn clone_vec(c: &Context<'_, '_>, child: u32, w: &mut Writer) -> Result<(), Diagnostic> {
    c.element(child)?;
    // Allocate before child cloning. The private result is never published until complete.
    w.format(format_args!("const $out=$vecNew($el{child},new Array($s.$length),0); try {{\nfor(let $i=0;$i<$s.$length;$i++) {{ const $child=$clone{child}($s.$items[$i]); $out.$items[$i]=$child; $out.$length++; }}\nreturn $out; }} catch($failure) {{\n"))?;
    if c.ty(child)?.drop_kind() != 0 {
        w.format(format_args!(
            "while($out.$length>0) $drop{child}($out.$items[--$out.$length]);\n"
        ))?;
    }
    w.format(format_args!(
        "$status_vecFree($vecFree($el{child},$out.$storage)); throw $failure;\n}}\n"
    ))
}
