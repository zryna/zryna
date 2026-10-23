//! Successful-child rollback follows retained Clone graph nodes only.
use super::{Context, Writer, charge, internal};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedCloneNode};
pub(super) fn body(
    c: &Context<'_, '_>,
    node: VerifiedCloneNode<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => w.format(format_args!("return $copy{}($s);\n", node.ty()))?,
        CloneNodeKind::String => w.text("return $stringClone($s);\n")?,
        CloneNodeKind::Enum => {
            w.text("switch ($s.tag) {\n")?;
            for (ordinal, child) in node.variants() {
                charge(&c.work, 1)?;
                w.format(format_args!("case {ordinal}: {{ "))?;
                if let Some(child) = child {
                    let id = child.ty();
                    w.format(format_args!("const $payload=$clone{id}($s.payload); try {{ return {{tag:{ordinal},payload:$payload}}; }} catch($e) {{ "))?;
                    if c.ty(id)?.drop_kind() != 0 {
                        w.format(format_args!("$drop{id}($payload); "))?;
                    }
                    w.text("throw $e; } }\n")?;
                } else {
                    w.format(format_args!("return {{tag:{ordinal},payload:undefined}}; }}\n"))?;
                }
            }
            super::invalid_tag(w)?;
        }
        CloneNodeKind::Struct => clone_struct(c, node, w)?,
        CloneNodeKind::FixedArray { length } => {
            let child = node.element().ok_or_else(internal)?.ty();
            w.format(format_args!("const $out=[]; let $done=0; try {{ for(;$done<{length};$done++) {{ const $child=$clone{child}($s[$done]); try {{ $out[$done]=$child; }} catch($e) {{ "))?;
            if c.ty(child)?.drop_kind() != 0 {
                w.format(format_args!("$drop{child}($child); "))?;
            }
            w.text("throw $e; } } return $out; } catch($e) { ")?;
            if c.ty(child)?.drop_kind() != 0 {
                w.format(format_args!("while($done>0) $drop{child}($out[--$done]); "))?;
            }
            w.text("throw $e; }\n")?;
        }
        CloneNodeKind::VecEach => {
            super::containers::clone_vec(c, node.element().ok_or_else(internal)?.ty(), w)?;
        }
        CloneNodeKind::SharedCount => w.text("return $countStrong($s);\n")?,
        CloneNodeKind::WeakCount => w.text("return $countWeak($s);\n")?,
    }
    Ok(())
}
fn clone_struct(
    c: &Context<'_, '_>,
    node: VerifiedCloneNode<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    let ty = c.ty(node.ty())?;
    w.text("const $out=[]; let $done=0; try {\n")?;
    for (ordinal, child) in node.fields() {
        let id = child.ty();
        charge(&c.work, 1)?;
        w.format(format_args!("{{ const $child=$clone{id}($s[{ordinal}]); try {{ $out[{ordinal}]=$child; }} catch($e) {{ "))?;
        if c.ty(id)?.drop_kind() != 0 {
            w.format(format_args!("$drop{id}($child); "))?;
        }
        w.text("throw $e; } $done++; }\n")?;
    }
    w.text("return $out; } catch($e) {\n")?;
    for (ordinal, id) in super::fields(ty, c)?.into_iter().rev() {
        if c.ty(id)?.drop_kind() != 0 {
            w.format(format_args!("if($done>{ordinal}) $drop{id}($out[{ordinal}]);\n"))?;
        }
    }
    w.text("throw $e; }\n")
}
