//! Actual complete Clone/init instruction counts under the certified-kind work law.
use super::fixture;
use crate::generic_execution_v1::recursive::calls::Kind;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
#[test]
fn actual_iterative_clone_init_and_rollback_fit_the_source_prepaid_bound() {
    for case in ["clone", "controlled-cleanup", "shared-weak-recursive", "mixed"] {
        let mut actual = 0u64;
        let mut expected = None;
        for kind in [Kind::Init, Kind::Clone] {
            fixture::with_graph(case, kind, |program, function, _, _| {
                let mut bound = 77u64;
                let mut covered = [false; 8];
                let plan = program.containers().expect("genuine certified frontiers");
                for index in 0..program.program().type_count() {
                    let Some(node) = plan.clone_node(u32::try_from(index).expect("bounded type"))
                    else {
                        continue;
                    };
                    let (slot, cost) = model(node.kind());
                    covered[slot] = true;
                    bound += cost
                        + 16 * u64::try_from(node.fields().len() + node.variants().len())
                            .expect("members");
                }
                if case == "mixed" {
                    assert!(
                        covered.into_iter().all(|seen| seen),
                        "existing original covers every Clone kind"
                    );
                }
                assert!(expected.is_none_or(|old| old == bound));
                expected = Some(bound);
                actual += function
                    .layout
                    .blocks()
                    .map(|block| {
                        u64::try_from(function.layout.block_insts(block).count())
                            .expect("actual instruction count")
                    })
                    .sum::<u64>();
            });
        }
        assert!(actual <= expected.expect("source-derived bound"), "{case}: {actual}");
    }
}
fn model(kind: CloneNodeKind) -> (usize, u64) {
    match kind {
        CloneNodeKind::Copy => (0, 30),
        CloneNodeKind::String => (1, 101),
        CloneNodeKind::Struct => (2, 17),
        CloneNodeKind::Enum => (3, 20),
        CloneNodeKind::FixedArray { .. } => (4, 33),
        CloneNodeKind::VecEach => (5, 183),
        CloneNodeKind::SharedCount => (6, 99),
        CloneNodeKind::WeakCount => (7, 99),
    }
}
