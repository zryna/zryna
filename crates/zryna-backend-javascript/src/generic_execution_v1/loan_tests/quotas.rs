use super::{concat_source, source};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;

fn check(program: &VerifiedExecution<'_>, case: &str) {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("RUNTIME_ORACLE.json")).expect("pre-output quota oracle");
    let row = oracle["ordinary_quota_upper_bounds"]
        .as_array()
        .expect("frozen rows")
        .iter()
        .find(|row| row["case"] == case)
        .expect("exact original case");
    let context = super::super::Context::new(program).expect("same retained target indices");
    let quota = super::super::runtime::ordinary::quota::prove(&context)
        .expect("finite same-seal referent effect proof");
    assert_eq!(u64::try_from(quota.allocations).expect("bounded quota"), row["physical_attempts"]);
    assert_eq!(
        u64::try_from(quota.transitions).expect("bounded quota"),
        row["canonical_transitions_with_cleanup"]
    );
}
#[test]
fn loans_preserve_physical_alias_and_mutation_effect_funding() {
    for case in [
        "formal-copy",
        "opaque-forward",
        "shared-reborrow-clone",
        "exclusive-reborrow-push",
        "generic-replace",
        "static-disjoint",
        "branch-cleanup",
        "fallible-cleanup",
        "referent-catalogue",
    ] {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _wire| check(program, case));
        }
    }
}
#[test]
fn concat_funds_only_fresh_result_and_eventual_release() {
    for case in ["local-observation", "same-source", "nested-temporaries", "formal-observation"] {
        for provider in ["native", "typescript"] {
            concat_source::with_program(case, provider, |program, _wire| check(program, case));
        }
    }
}
