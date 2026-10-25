//! Genuine source producer and new-seal pending-obligation ordering.
#[path = "generic_execution_v1_source_anchors/mod.rs"]
mod source;
use source::with_claim;
use zryna_ir::generic_v1::execution_v1::raw;
fn parameter(function: &raw::Function, ordinal: u32) -> u32 {
    function
        .places
        .iter()
        .find(|p| p.kind == raw::PlaceKind::Parameter(ordinal))
        .expect("exact original parameter")
        .id
}
fn prepared(function: &raw::Function) -> (u32, u32) {
    let first = parameter(function, 0);
    function
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find_map(|i| match i.operation {
            raw::Operation::ReadPlace { place, mode: raw::ReadMode::Move } if place == first => {
                let result = i.result.as_ref().expect("prepared moved first argument");
                let owner = function
                    .places
                    .iter()
                    .find(|p| p.kind == raw::PlaceKind::Temporary(result.id))
                    .expect("exact prepared owner")
                    .id;
                Some((result.id, owner))
            }
            _ => None,
        })
        .expect("actual source-produced move")
}
#[test]
fn whole_read_and_later_local_initialization_keep_the_original_pending_slot() {
    for provider in ["native", "typescript"] {
        with_claim("local-transfer", provider, |claim| {
            let f = claim
                .functions
                .iter()
                .find(|f| f.parameters.len() == 3)
                .expect("original three-affine-parameter function");
            let (value, _) = prepared(f);
            let local = f
                .blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .find_map(|i| match i.operation {
                    raw::Operation::InitializePlace { place, value: rhs } if rhs == value => {
                        Some(place)
                    }
                    _ => None,
                })
                .expect("later exact local initialization");
            let expected = vec![
                raw::CleanupAction::DropPlace(parameter(f, 2)),
                raw::CleanupAction::DropPlace(parameter(f, 1)),
                raw::CleanupAction::DropPlace(local),
            ];
            let cleanup = f
                .blocks
                .iter()
                .find_map(|b| match b.terminator {
                    raw::Terminator::Return { cleanup, .. } => Some(cleanup),
                    _ => None,
                })
                .expect("original final return");
            assert_eq!(
                f.cleanup_plans[cleanup as usize].actions, expected,
                "M3 reverse pending order is independent of later destination publication"
            );
        });
    }
}
#[test]
fn prepared_first_call_argument_retains_its_slot_during_later_clone_failure() {
    for provider in ["native", "typescript"] {
        with_claim("prepared-call", provider, |claim| {
            let f = claim
                .functions
                .iter()
                .find(|f| f.parameters.len() == 3)
                .expect("three parameter preparing caller");
            let (_, owner) = prepared(f);
            let expected = vec![
                raw::CleanupAction::DropPlace(parameter(f, 2)),
                raw::CleanupAction::DropPlace(parameter(f, 1)),
                raw::CleanupAction::DropPlace(owner),
            ];
            let (outer, prefix) = f
                .blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .find_map(|i| match i.operation {
                    raw::Operation::Clone {
                        outer_failure: Some(outer),
                        prefix_failure: Some(prefix),
                        ..
                    } => Some((outer, prefix)),
                    _ => None,
                })
                .expect("actual second argument fallible Clone");
            assert_eq!(
                f.cleanup_plans[outer as usize].actions, expected,
                "prepared argument transfers its original pending obligation"
            );
            let actions = &f.cleanup_plans[prefix as usize].actions;
            assert!(matches!(
                actions.first(),
                Some(raw::CleanupAction::DropInitializedPrefix { .. })
            ));
            assert_eq!(
                &actions[1..],
                expected.as_slice(),
                "local prefix rollback precedes unchanged normative outer cleanup"
            );
        });
    }
}
