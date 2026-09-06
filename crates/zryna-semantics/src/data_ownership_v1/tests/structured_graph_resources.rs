use super::constructor_resources::tests::with_snapshot;
use super::structured_cfg::resources::parameter;
use super::structured_checkpoint::StructuredCheckpoint;
use super::structured_graph::with_held_resources;
use crate::data_ownership_v1::tests::structured_owned_fixture::{Payload, match_fixture};
use zryna_ir::data_ownership_v1::{self as ir, raw};

fn edge_count(blocks: &[raw::Block]) -> usize {
    blocks
        .iter()
        .map(|block| match &block.terminators[0].kind {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
            raw::Terminator::Jump(_) => 1,
            raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
            raw::Terminator::EnumMatch { arms, .. } => arms.len(),
        })
        .sum()
}

#[test]
fn structured_graph_held_blocks_and_edges_are_checked_and_recover_on_the_same_lowerer() {
    let (source, snapshot) = match_fixture(Payload::Struct, true, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        let parameters = parameter(lowerer);
        let initial = StructuredCheckpoint::capture(lowerer);
        let pristine = lowerer
            .lower_structured_cfg(&parameters, result)
            .expect("authenticated source-produced graph");
        let used = [pristine.len(), edge_count(&pristine)];
        initial.restore(lowerer);
        for (resource, maximum) in
            [(0, ir::MAX_BLOCKS_PER_FUNCTION), (1, ir::MAX_CFG_EDGES_PER_FUNCTION)]
        {
            for extra in [0, 1, usize::MAX] {
                let held =
                    if extra == usize::MAX { extra } else { maximum - used[resource] + extra };
                let before = StructuredCheckpoint::capture(lowerer);
                let mut held_resources = [0, 0];
                held_resources[resource] = held;
                let output = with_held_resources(held_resources, || {
                    lowerer.lower_structured_cfg(&parameters, result)
                });
                before.restore(lowerer);
                if extra == 0 {
                    assert_eq!(output, Some(pristine.clone()), "exact resource {resource}");
                } else {
                    assert!(output.is_none(), "resource {resource}, extra {extra}");
                }
            }
        }

        assert_eq!(
            lowerer.lower_structured_cfg(&parameters, result),
            Some(pristine),
            "valid graph recovers on the same lowerer"
        );
    });
    let trace = errors
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code(),
                diagnostic.message(),
                diagnostic.primary_span().map(|span| (span.start(), span.end())),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        trace,
        [
            (
                "ZRYNA-M3201",
                "structured ownership blocks exceed the checked limit",
                Some((193, 338)),
            ),
            (
                "ZRYNA-M3201",
                "structured ownership edges exceed the checked limit",
                Some((193, 338)),
            ),
            (
                "ZRYNA-M3201",
                "structured ownership blocks exceed the checked limit",
                Some((220, 320)),
            ),
            (
                "ZRYNA-M3201",
                "structured ownership edges exceed the checked limit",
                Some((220, 320)),
            ),
        ]
    );
}
