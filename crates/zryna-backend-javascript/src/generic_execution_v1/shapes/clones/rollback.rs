//! Discard borrowed pending slots, then only initialized owned children in exact reverse order.
use super::{CloneNodeKind, Context, Diagnostic, Writer, argument, charge, internal};

pub(super) fn cases(c: &Context<'_, '_>, w: &mut Writer) -> Result<(), Diagnostic> {
    charge(&c.work, c.clones.len())?;
    for node in c.clones.iter().flatten().copied() {
        if !matches!(
            node.kind(),
            CloneNodeKind::Struct
                | CloneNodeKind::Enum
                | CloneNodeKind::FixedArray { .. }
                | CloneNodeKind::VecEach
        ) {
            continue;
        }
        w.format(format_args!("case {}: {{\n", node.ty()))?;
        match node.kind() {
            CloneNodeKind::Struct => {
                w.text("if($done in $out) $out[$done]=undefined;\n")?;
                let fields = fields(c, node)?;
                for (ordinal, child) in fields.into_iter().rev() {
                    if c.ty(child)?.drop_kind() != 0 {
                        w.format(format_args!(
                            "if($done>{ordinal}) {{ $dropLoop($out[{ordinal}],{child}"
                        ))?;
                        argument(c, w)?;
                        w.text("); }\n")?;
                    }
                }
            }
            CloneNodeKind::Enum => {
                w.text("if($done>0) { switch($out.tag) {\n")?;
                charge(&c.work, node.variants().len())?;
                for (ordinal, child) in node.variants() {
                    w.format(format_args!("case {ordinal}: "))?;
                    if let Some(child) = child
                        && c.ty(child.ty())?.drop_kind() != 0
                    {
                        w.format(format_args!("$dropLoop($out.payload,{}", child.ty()))?;
                        argument(c, w)?;
                        w.text("); ")?;
                    }
                    w.text("break;\n")?;
                }
                super::super::invalid_tag(w)?;
                w.text("}\n$out.payload=undefined;\n")?;
            }
            CloneNodeKind::FixedArray { .. } | CloneNodeKind::VecEach => {
                let child = node.element().ok_or_else(internal)?.ty();
                let items =
                    if node.kind() == CloneNodeKind::VecEach { "$out.$items" } else { "$out" };
                w.format(format_args!("if($done in {items}) {items}[$done]=undefined;\n"))?;
                if c.ty(child)?.drop_kind() != 0 {
                    w.format(format_args!(
                        "for(let $index=$done;$index>0;) {{ $dropLoop({items}[--$index],{child}"
                    ))?;
                    argument(c, w)?;
                    w.text("); }\n")?;
                }
                if node.kind() == CloneNodeKind::VecEach {
                    if c.accounting.is_some() {
                        w.format(format_args!(
                            "$vecStorageFree($el{child},$out.$storage,$ctx);\n"
                        ))?;
                    } else {
                        w.format(format_args!(
                            "$status_vecFree($vecFree($el{child},$out.$storage));\n"
                        ))?;
                    }
                }
            }
            _ => return Err(internal()),
        }
        w.text("break; }\n")?;
    }
    Ok(())
}

fn fields(
    c: &Context<'_, '_>,
    node: super::VerifiedCloneNode<'_, '_>,
) -> Result<Vec<(u32, u32)>, Diagnostic> {
    // Eight bytes per compact pair, its extraction, and the later reverse visit.
    let cost =
        node.fields().len().checked_mul(10).ok_or_else(super::super::super::emission_budget)?;
    charge(&c.work, cost)?;
    let mut fields = Vec::new();
    #[cfg(test)]
    RESERVATIONS.with(|n| n.set(n.get() + 1));
    fields
        .try_reserve_exact(node.fields().len())
        .map_err(|_| super::super::super::emission_budget())?;
    fields.extend(node.fields().map(|(ordinal, child)| {
        #[cfg(test)]
        VISITS.with(|n| n.set(n.get() + 1));
        (ordinal, child.ty())
    }));
    Ok(fields)
}
#[cfg(test)]
thread_local! {
    static RESERVATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_execution_v1::loan_tests;
    #[test]
    fn genuine_struct_rollback_prepays_compact_scratch_and_visits_before_reserve() {
        assert_eq!(size_of::<(u32, u32)>(), 8);
        for provider in ["native", "typescript"] {
            loan_tests::catalogue(provider, |program, _| {
                let context = Context::new(program).expect("genuine source/execution owner");
                let node = context
                    .clones
                    .iter()
                    .flatten()
                    .copied()
                    .find(|n| n.kind() == CloneNodeKind::Struct)
                    .expect("actual Pair<String> Clone node");
                context.work.set(0);
                let baseline = fields(&context, node).expect("compact actual certified fields");
                let cost = context.work.get();
                assert_eq!(cost, baseline.len() * 10);
                let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
                context.work.set(limit - cost);
                fields(&context, node).expect("exact remaining scratch and scans");
                assert_eq!(context.work.get(), limit);
                for start in [limit - cost + 1, usize::MAX] {
                    RESERVATIONS.with(|n| n.set(0));
                    VISITS.with(|n| n.set(0));
                    context.work.set(start);
                    let error =
                        fields(&context, node).expect_err("short before reserve or traversal");
                    assert_eq!(error.code(), "ZRYNA-J2003");
                    assert_eq!(context.work.get(), start);
                    RESERVATIONS.with(|n| assert_eq!(n.get(), 0));
                    VISITS.with(|n| assert_eq!(n.get(), 0));
                }
                context.work.set(0);
                assert_eq!(fields(&context, node).expect("pristine retry"), baseline);
                RESERVATIONS.with(|n| assert_eq!(n.get(), 1));
                VISITS.with(|n| assert_eq!(n.get(), baseline.len()));
            });
        }
    }
}
