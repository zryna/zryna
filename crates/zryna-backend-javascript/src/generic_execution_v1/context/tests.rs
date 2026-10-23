//! Actual same-seal discovery work, including payloadless variants and cached children.
use super::{CloneNodeKind, Context};
use crate::generic_execution_v1::tests::with_program;
use crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;

#[test]
fn genuine_signal_prepays_every_variant_before_any_child_even_when_children_are_cached() {
    for provider in ["native", "typescript"] {
        with_program("nominal-enum-clone", provider, |program| {
            let root = program
                .operations()
                .find_map(zryna_ir::generic_v1::execution_v1::VerifiedOperation::clone_frontier)
                .expect("genuine original Signal Clone")
                .frontier();
            assert_eq!(root.kind(), CloneNodeKind::Enum);
            assert_eq!(root.variants().len(), 3, "fixed original Signal inventory");
            assert_eq!(root.variants().filter(|(_, child)| child.is_none()).count(), 1);
            let mut context = Context::new(program).expect("actual same-seal indices");
            for (_, child) in root.variants() {
                if let Some(child) = child {
                    assert!(
                        context.clones[child.ty() as usize].is_some(),
                        "actual shared String child already discovered"
                    );
                }
            }
            context.clones[root.ty() as usize] = None;
            // One node visit plus three variants is the minimum credit before
            // inspecting any payload. The two cached child visits remain later work.
            let start = MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - 3;
            context.work.set(start);
            let error = context
                .mark_clone(root, 0)
                .expect_err("full variant census must fail before a child is inspected");
            assert_eq!(error.code(), "ZRYNA-J2003");
            assert_eq!(
                context.work.get(),
                start + 1,
                "failed upfront census publishes no partial per-variant debit"
            );
            context.clones[root.ty() as usize] = None;
            context.work.set(MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - 6);
            context.mark_clone(root, 0).expect("node1 + variants3 + two cached child visits2");
            assert_eq!(context.work.get(), MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
            context.clones[root.ty() as usize] = None;
            context.work.set(MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - 5);
            context
                .mark_clone(root, 0)
                .expect_err("first-extra work cannot disappear behind cached children");
            Context::new(program).expect("pristine retry re-derives genuine discovery");
        });
    }
}

#[test]
fn genuine_previsited_copy_root_pays_its_visit_and_does_not_repeat_child_census() {
    with_program("scalar-i32-clone", "native", |program| {
        let clone = program
            .operations()
            .find_map(zryna_ir::generic_v1::execution_v1::VerifiedOperation::clone_frontier)
            .expect("genuine original i32 Clone");
        assert_eq!(clone.frontier().kind(), CloneNodeKind::Copy);
        let mut context = Context::new(program).expect("genuine Copy discovery");
        assert!(context.copies[clone.ty() as usize]);
        context.work.set(MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - 1);
        context.mark_copy(clone.ty(), 0).expect("one actual cached-root visit");
        assert_eq!(context.work.get(), MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
        context.mark_copy(clone.ty(), 0).expect_err("first-extra cached visit remains a debit");
    });
}
