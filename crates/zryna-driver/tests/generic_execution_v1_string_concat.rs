//! Inherited concat source spelling must reach the genuine NEW owner and native object.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_string_concat/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::raw;
fn positive(case: &str, count: usize) {
    for provider in ["native", "typescript"] {
        source::with_claim(case, provider, |claim| {
            let operations = claim
                .functions
                .iter()
                .flat_map(|f| &f.blocks)
                .flat_map(|b| &b.instructions)
                .filter(|i| matches!(i.operation, raw::Operation::StringConcat { .. }))
                .count();
            assert_eq!(
                operations, count,
                "actual original-to-claim builtin observations, not user calls"
            );
            for function in &claim.functions {
                for block in &function.blocks {
                    for instruction in &block.instructions {
                        if let raw::Operation::StringConcat { failure, .. } = instruction.operation
                        {
                            let result = instruction.result.as_ref().expect("concat result");
                            let destination = function
                                .places
                                .iter()
                                .find(|p| p.kind == raw::PlaceKind::Temporary(result.id))
                                .expect("pending concat destination")
                                .id;
                            assert!(
                                !function.cleanup_plans[failure as usize]
                                    .actions
                                    .contains(&raw::CleanupAction::DropPlace(destination)),
                                "failure excludes unpublished concat result"
                            );
                        }
                    }
                }
            }
        });
    }
}
#[test]
fn named_operands_remain_available_after_concat() {
    positive("local-observation", 1);
}
#[test]
fn the_same_readable_string_can_supply_both_operands() {
    positive("same-source", 1);
}
#[test]
fn nested_temporary_observations_keep_exact_preparation_lifetimes() {
    positive("nested-temporaries", 2);
}
#[test]
fn formal_shared_referents_support_nonconsuming_concat_observations() {
    positive("formal-observation", 1);
}
