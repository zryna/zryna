//! Container-stage budgets must be reached with real source authority and retained helpers.
use super::{check_boundary, fixture, lower};
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, raw};
#[test]
fn genuine_settled_container_helpers_exact_extra_overflow_and_metadata_retry() {
    for case in fixture::CONTAINERS {
        fixture::with_program(case, |program| {
            let mir = lower(program).expect("real whole-source container native owner");
            assert_eq!(mir.containers().is_some(), case != "opaque-count-unused");
            if let Some(plan) = mir.containers() {
                let mut roots = Vec::new();
                for op in program.operations() {
                    if let Some(clone) = op.clone_frontier() {
                        roots.push(clone.frontier().kind());
                        assert_eq!(
                            plan.clone_node(clone.ty())
                                .expect("retained operation frontier")
                                .kind(),
                            clone.frontier().kind()
                        );
                    }
                    if let Some(view) = op.vec_construct() {
                        let metadata = plan.element(view.ty()).expect("exact element metadata");
                        assert_eq!(metadata.element().index(), view.element_type());
                    }
                    if let Some(view) = op.shared_construct() {
                        let metadata = plan.control(view.ty()).expect("exact control metadata");
                        assert_eq!(metadata.payload().index(), view.referent_type());
                    }
                }
                if case == "shared-weak-count" {
                    assert!(roots.contains(&CloneNodeKind::SharedCount));
                    assert!(roots.contains(&CloneNodeKind::WeakCount));
                    assert!(!roots.contains(&CloneNodeKind::VecEach));
                } else {
                    assert!(roots.contains(&CloneNodeKind::VecEach));
                    assert!(
                        program
                            .operations()
                            .any(|op| matches!(op.kind(), raw::Operation::VecConstruct { .. }))
                    );
                }
                assert!(
                    plan.clone_symbols().flatten().next().is_some(),
                    "helper gate genuinely reached"
                );
                assert!(plan.drop_symbols().flatten().next().is_some());
            }
            check_boundary(program, case);
        });
    }
}
