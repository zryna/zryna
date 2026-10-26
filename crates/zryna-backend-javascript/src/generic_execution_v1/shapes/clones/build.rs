//! Save the source parent only in the next uninitialized, nonowning destination slot.
use super::{
    CloneNodeKind, Context, Diagnostic, VerifiedCloneNode, Writer, argument, charge, complete,
    internal,
};

pub(super) fn body(
    c: &Context<'_, '_>,
    node: VerifiedCloneNode<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => {
            w.format(format_args!("$value=$copy{}($s);\n", node.ty()))?;
            ready(w, node)
        }
        CloneNodeKind::String | CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => {
            let helper = match node.kind() {
                CloneNodeKind::String => "$stringClone",
                CloneNodeKind::SharedCount => "$countStrong",
                _ => "$countWeak",
            };
            w.format(format_args!("$value={helper}($s"))?;
            argument(c, w)?;
            w.text(");\n")?;
            ready(w, node)
        }
        CloneNodeKind::Struct => {
            begin(w, node, "$owner([])")?;
            charge(&c.work, node.fields().len())?;
            for (ordinal, child) in node.fields() {
                w.format(format_args!("if($parent.$dropIndex==={ordinal}) {{ const $next=$s[{ordinal}]; $parent[{ordinal}]=$s; $s=$next; $type={}; $resume=false; continue; }}\n", child.ty()))?;
            }
            complete(w, node)
        }
        CloneNodeKind::Enum => {
            begin(w, node, "$enumOwner($s.tag,undefined)")?;
            w.text("if($parent.$dropIndex===0) { switch($s.tag) {\n")?;
            charge(&c.work, node.variants().len())?;
            for (ordinal, child) in node.variants() {
                w.format(format_args!("case {ordinal}: "))?;
                if let Some(child) = child {
                    w.format(format_args!("{{ const $next=$s.payload; $parent.payload=$s; $s=$next; $type={}; $resume=false; continue; }}\n",child.ty()))?;
                } else {
                    w.text("break;\n")?;
                }
            }
            super::super::invalid_tag(w)?;
            w.text("}\n")?;
            complete(w, node)
        }
        CloneNodeKind::FixedArray { length } => {
            begin(w, node, "$owner([])")?;
            let child = node.element().ok_or_else(internal)?.ty();
            w.format(format_args!("if($parent.$dropIndex<{length}) {{ const $index=$parent.$dropIndex; const $next=$s[$index]; $parent[$index]=$s; $s=$next; $type={child}; $resume=false; continue; }}\n"))?;
            complete(w, node)
        }
        CloneNodeKind::VecEach => {
            let child = node.element().ok_or_else(internal)?.ty();
            c.element(child)?;
            w.format(format_args!(
                "if(!$resume) {{ const $out=$vecNew($el{child},new Array($s.$length),0"
            ))?;
            argument(c, w)?;
            w.text("); ")?;
            push(w, node)?;
            w.text("}\n")?;
            w.format(format_args!("if($parent.$length<$s.$length) {{ const $index=$parent.$length; const $next=$s.$items[$index]; $parent.$items[$index]=$s; $s=$next; $type={child}; $resume=false; continue; }}\n"))?;
            complete(w, node)
        }
    }
}
fn ready(w: &mut Writer, node: VerifiedCloneNode<'_, '_>) -> Result<(), Diagnostic> {
    w.format(format_args!("$readyType={}; $ready=true; $resume=false; continue;\n", node.ty()))
}
fn begin(
    w: &mut Writer,
    node: VerifiedCloneNode<'_, '_>,
    expression: &str,
) -> Result<(), Diagnostic> {
    w.format(format_args!("if(!$resume) {{ const $out={expression}; "))?;
    push(w, node)?;
    w.text("}\n")
}
fn push(w: &mut Writer, node: VerifiedCloneNode<'_, '_>) -> Result<(), Diagnostic> {
    w.format(format_args!(
        "$out.$dropParent=$parent; $out.$dropType={}; $out.$dropIndex=0; $parent=$out; ",
        node.ty()
    ))
}
