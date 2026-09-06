use super::PrivateOwnedAggregateLowerer;
use super::constructor_resources::tests::with_snapshot;
use super::structured_cfg::resources::parameter;
use super::structured_checkpoint::StructuredCheckpoint;
use super::structured_graph::{with_held_resources, with_resource_limits};
use crate::data_ownership_v1::tests::structured_owned_fixture::{
    Payload, match_fixture, mixed_variant_fixture, nested_variant_fixture,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{self as ir, raw};

type DiagnosticTrace<'a> = Vec<(&'a str, &'a str, Option<(u32, u32)>)>;

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

fn lowerer_state(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> String {
    format!(
        "{:?}",
        (
            lowerer.preparation_checkpoint(),
            &lowerer.bindings,
            &lowerer.places,
            &lowerer.instructions,
            &lowerer.constructor_types,
            &lowerer.owners,
            &lowerer.preparation_facts,
        )
    )
}

fn trace(errors: &[Diagnostic]) -> DiagnosticTrace<'_> {
    errors
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code(),
                diagnostic.message(),
                diagnostic.primary_span().map(|span| (span.start(), span.end())),
            )
        })
        .collect()
}

#[test]
fn authenticated_graph_hits_exact_and_first_extra_block_and_edge_limits() {
    let (source, snapshot) = match_fixture(Payload::Struct, true, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        let parameters = parameter(lowerer);
        let initial = StructuredCheckpoint::capture(lowerer);
        let pristine = lowerer
            .lower_structured_cfg(&parameters, result)
            .expect("authenticated source-produced graph");
        let used = [pristine.len(), edge_count(&pristine)];
        initial.restore(lowerer);

        for resource in 0..2 {
            let mut exact = [ir::MAX_BLOCKS_PER_FUNCTION, ir::MAX_CFG_EDGES_PER_FUNCTION];
            exact[resource] = used[resource];
            let before = StructuredCheckpoint::capture(lowerer);
            assert_eq!(
                with_resource_limits(exact, || lowerer.lower_structured_cfg(&parameters, result)),
                Some(pristine.clone()),
                "source graph reaches exact resource {resource}"
            );
            before.restore(lowerer);

            let before_state = lowerer_state(lowerer);
            let before = StructuredCheckpoint::capture(lowerer);
            exact[resource] -= 1;
            assert!(
                with_resource_limits(exact, || lowerer.lower_structured_cfg(&parameters, result))
                    .is_none(),
                "source graph crosses first-extra resource {resource}"
            );
            assert_eq!(lowerer_state(lowerer), before_state, "atomic resource {resource}");
            assert_eq!(
                lowerer.lower_structured_cfg(&parameters, result),
                Some(pristine.clone()),
                "same-instance recovery after source limit rejection"
            );
            before.restore(lowerer);
        }
    });
    assert_eq!(
        trace(&errors),
        [
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

#[test]
fn synthetic_held_block_and_edge_overflow_is_checked_and_recovers() {
    let (source, snapshot) = match_fixture(Payload::Struct, true, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        let parameters = parameter(lowerer);
        let initial = StructuredCheckpoint::capture(lowerer);
        let pristine = lowerer
            .lower_structured_cfg(&parameters, result)
            .expect("authenticated source-produced graph");
        initial.restore(lowerer);

        for resource in 0..2 {
            let before_state = lowerer_state(lowerer);
            let before = StructuredCheckpoint::capture(lowerer);
            let mut held = [0, 0];
            held[resource] = usize::MAX;
            assert!(
                with_held_resources(held, || lowerer.lower_structured_cfg(&parameters, result))
                    .is_none(),
                "checked held overflow {resource}"
            );
            assert_eq!(lowerer_state(lowerer), before_state, "atomic overflow {resource}");
            assert_eq!(
                lowerer.lower_structured_cfg(&parameters, result),
                Some(pristine.clone()),
                "same-instance recovery after synthetic overflow"
            );
            before.restore(lowerer);
        }
    });
    assert_eq!(
        trace(&errors),
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
        ]
    );
}

#[test]
fn complete_enum_matches_bound_graph_resources_atomically_and_recover() {
    for (label, (source, snapshot)) in
        [("three-arm", mixed_variant_fixture()), ("nested", nested_variant_fixture())]
    {
        let errors = with_snapshot(&source, snapshot, |lowerer, result| {
            let parameters = parameter(lowerer);
            let initial = StructuredCheckpoint::capture(lowerer);
            let pristine = lowerer
                .lower_structured_cfg(&parameters, result)
                .expect("authenticated complete enum-match graph");
            let used = [pristine.len(), edge_count(&pristine)];
            initial.restore(lowerer);

            for resource in 0..2 {
                let mut exact = [ir::MAX_BLOCKS_PER_FUNCTION, ir::MAX_CFG_EDGES_PER_FUNCTION];
                exact[resource] = used[resource];
                let checkpoint = StructuredCheckpoint::capture(lowerer);
                assert_eq!(
                    with_resource_limits(exact, || {
                        lowerer.lower_structured_cfg(&parameters, result)
                    }),
                    Some(pristine.clone()),
                    "{label} reaches exact graph resource {resource}"
                );
                checkpoint.restore(lowerer);

                for held in [
                    used[resource].checked_sub(1).map(|remaining| {
                        [ir::MAX_BLOCKS_PER_FUNCTION, ir::MAX_CFG_EDGES_PER_FUNCTION][resource]
                            - remaining
                    }),
                    Some(usize::MAX),
                ] {
                    let mut occupied = [0, 0];
                    occupied[resource] = held.expect("complete match uses each graph resource");
                    let before = lowerer_state(lowerer);
                    let checkpoint = StructuredCheckpoint::capture(lowerer);
                    assert!(
                        with_held_resources(occupied, || {
                            lowerer.lower_structured_cfg(&parameters, result)
                        })
                        .is_none(),
                        "{label} rejects first-extra or overflow graph resource {resource}"
                    );
                    assert_eq!(
                        lowerer_state(lowerer),
                        before,
                        "{label} rejects graph resource {resource} atomically"
                    );
                    assert_eq!(
                        lowerer.lower_structured_cfg(&parameters, result),
                        Some(pristine.clone()),
                        "{label} recovers after graph resource {resource} rejection"
                    );
                    checkpoint.restore(lowerer);
                }
            }
        });
        assert_eq!(errors.len(), 4, "{label} records two failures for each graph resource");
        assert!(errors.iter().all(|error| error.code() == "ZRYNA-M3201"));
    }
}
