//! Certified Clone nodes drive pending destinations; only committed prefixes own children.
use super::{Context, Writer, charge, internal};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedCloneNode};
mod build;
mod rollback;

pub(super) fn declarations(c: &Context<'_, '_>, w: &mut Writer) -> Result<(), Diagnostic> {
    w.text("function $cloneLoop($s,$type,$ctx) { let $parent=null,$resume=false,$ready=false,$readyType=-1,$value; try { for(;;) {\nif($ready) { if($parent===null) return $value; switch($parent.$dropType) {\n")?;
    attach(c, w)?;
    w.text("default: throw new Error('ZRYNA-RT-TAG');\n} $ready=false; $resume=true; $type=$parent.$dropType; }\nswitch($type) {\n")?;
    charge(&c.work, c.clones.len())?;
    for node in c.clones.iter().flatten().copied() {
        w.format(format_args!("case {}: {{\n", node.ty()))?;
        build::body(c, node, w)?;
        w.text("}\n")?;
    }
    w.text("default: throw new Error('ZRYNA-RT-TAG');\n}\n} } catch($failure) {\nif($ready) { switch($readyType) {\n")?;
    charge(&c.work, c.clones.len())?;
    for node in c.clones.iter().flatten().copied() {
        if c.ty(node.ty())?.drop_kind() != 0 {
            w.format(format_args!("case {}: $dropLoop($value,{}", node.ty(), node.ty()))?;
            argument(c, w)?;
            w.text("); break;\n")?;
        }
    }
    w.text("} }\nwhile($parent!==null) { const $out=$parent; $parent=$out.$dropParent; const $done=$out.$dropIndex; const $kind=$out.$dropType;\n$out.$dropParent=null; $out.$dropType=-1; $out.$dropIndex=-1;\nswitch($kind) {\n")?;
    rollback::cases(c, w)?;
    w.text("default: throw new Error('ZRYNA-RT-TAG');\n} }\nthrow $failure;\n} }\n")
}

fn attach(c: &Context<'_, '_>, w: &mut Writer) -> Result<(), Diagnostic> {
    charge(&c.work, c.clones.len())?;
    for node in c.clones.iter().flatten().copied() {
        let slot = match node.kind() {
            CloneNodeKind::Struct | CloneNodeKind::FixedArray { .. } => {
                "$parent[$parent.$dropIndex]"
            }
            CloneNodeKind::Enum => "$parent.payload",
            CloneNodeKind::VecEach => "$parent.$items[$parent.$length]",
            _ => continue,
        };
        w.format(format_args!(
            "case {}: $s={slot}; {slot}=$value; $parent.$dropIndex++; ",
            node.ty()
        ))?;
        if node.kind() == CloneNodeKind::VecEach {
            w.text("$parent.$length++; ")?;
        }
        w.text("break;\n")?;
    }
    Ok(())
}

fn argument(c: &Context<'_, '_>, w: &mut Writer) -> Result<(), Diagnostic> {
    super::super::runtime::accounting::argument(c, w, true)
}
fn complete(w: &mut Writer, node: VerifiedCloneNode<'_, '_>) -> Result<(), Diagnostic> {
    w.format(format_args!("$value=$parent; $parent=$value.$dropParent; $value.$dropParent=null; $value.$dropType=-1; $value.$dropIndex=-1;\n$readyType={}; $ready=true; $resume=false; continue;\n", node.ty()))
}
