//! V5-only opaque structural clone lowering; successful leaves form a private rollback log.
use super::{Writer, emission_budget, internal};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::clone_v5::{
    CloneNodeKind, VerifiedCloneNode as CloneNode, VerifiedStructClone,
};

#[cfg(test)]
mod budget_tests;

pub(super) fn expression(clone: VerifiedStructClone<'_>) -> Result<String, Diagnostic> {
    let mut writer = Writer { source: String::new() };
    if clone.string_occurrences() != 0 {
        writer.text(&format!(
            "(() => {{ const $cp=new Array({}).fill(undefined); let $cc=0; try {{ return ",
            clone.string_occurrences()
        ))?;
    }
    node(&mut writer, clone.root(), &format!("$v{}", clone.source()))?;
    if clone.string_occurrences() != 0 {
        writer.text("; } catch ($ce) { while ($cc>0) $free($cp[--$cc]); throw $ce; } })()")?;
    }
    Ok(writer.source)
}

fn node(w: &mut Writer, view: CloneNode<'_>, source: &str) -> Result<(), Diagnostic> {
    match view.kind() {
        CloneNodeKind::Copy => w.text(source),
        CloneNodeKind::String => {
            w.text("(($s) => { const $h=$handle($clone($s)); try { $cp[$cc]=$h; } catch ($e) { $free($h); throw $e; } $cc++; return $h; })(")?;
            w.text(source)?;
            w.text(")")
        }
        CloneNodeKind::Struct => {
            w.text("(($s) => [")?;
            for (position, (ordinal, _, child)) in view.fields().enumerate() {
                if position != 0 {
                    w.text(",")?;
                }
                node(w, child, &format!("$s[{ordinal}]"))?;
            }
            w.text("])(")?;
            w.text(source)?;
            w.text(")")
        }
        CloneNodeKind::Enum => {
            w.text("(($s) => { switch ($s.tag) {")?;
            for (ordinal, _, payload) in view.variants() {
                w.line(format_args!("case {ordinal}: return {{tag:{ordinal},payload:"))?;
                if let Some(child) = payload {
                    node(w, child, "$s.payload")?;
                } else {
                    w.text("undefined")?;
                }
                w.text("};")?;
            }
            w.text("default: throw new Error('ZRYNA-I7001: invalid private discriminant'); } })(")?;
            w.text(source)?;
            w.text(")")
        }
    }
}

pub(super) fn preflight(
    clone: VerifiedStructClone<'_>,
    spent: &mut usize,
) -> Result<(), Diagnostic> {
    let mut work = *spent;
    let strings = usize::try_from(clone.string_occurrences()).map_err(|_| emission_budget())?;
    // Bound complete possible fanout before expanded traversal, literals or log allocation.
    charge(&mut work, strings.checked_mul(4).ok_or_else(emission_budget)?)?;
    u32::try_from(strings).map_err(|_| emission_budget())?;
    if (strings != 0) != clone.failure_step().is_some() {
        return Err(internal());
    }
    count(clone.root(), &mut work, 0)?;
    // Site wrapper and preallocated successful-handle slots, including reverse rollback.
    charge(&mut work, 768)?;
    *spent = work;
    Ok(())
}

fn count(node: CloneNode<'_>, work: &mut usize, depth: usize) -> Result<(), Diagnostic> {
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(internal());
    }
    // Each template is below 256 bytes. Pay expression, formatted line and final output copies.
    charge(work, 768)?;
    match node.kind() {
        CloneNodeKind::Struct => {
            charge(work, node.fields().len().checked_mul(96).ok_or_else(emission_budget)?)?;
            for (_, _, child) in node.fields() {
                count(child, work, depth + 1)?;
            }
        }
        CloneNodeKind::Enum => {
            charge(work, node.variants().len().checked_mul(384).ok_or_else(emission_budget)?)?;
            for (_, _, child) in node.variants() {
                if let Some(child) = child {
                    count(child, work, depth + 1)?;
                }
            }
        }
        CloneNodeKind::Copy | CloneNodeKind::String => {}
    }
    Ok(())
}

fn charge(work: &mut usize, count: usize) -> Result<(), Diagnostic> {
    *work = work
        .checked_add(count)
        .filter(|n| *n <= crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
        .ok_or_else(emission_budget)?;
    Ok(())
}
