//! Existing frozen nonloan push/upgrade sources through the actual new whole execution seal.
#[path = "generic_execution_v1_push_upgrade/runtime.rs"]
mod runtime;
#[path = "generic_execution_v1_push_upgrade/source.rs"]
mod source;
use zryna_backend_webassembly::generic_execution_v1::{emit, emit_observed, memory};

#[test]
fn genuine_vector_push_commits_after_success_and_preserves_prior_owner_on_allocation_failure() {
    check("vec-push");
}
#[test]
fn genuine_live_weak_upgrade_publishes_one_shared_owner_and_cleans_up_before_reentry() {
    check("weak-live");
}
#[test]
fn genuine_expired_weak_upgrade_takes_only_expiration_without_publishing_shared_owner() {
    check("weak-expired");
}

fn check(case: &str) {
    assert!(source::SETTLED.contains(&case));
    let (_, native) = source::project(case, "native");
    let (_, typescript) = source::project(case, "typescript");
    assert_eq!(native.files(), typescript.files(), "complete authenticated provider DTO parity");
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let plan = memory::verify(
                memory::derive(program, false).expect("actual memory claim"),
                program,
            )
            .expect("independent same-program memory authority");
            assert_eq!(plan.claims().memory_pages, 256);
            assert_eq!(plan.claims().scratch_bytes, 32);
            assert!(
                plan.claims().heap_start >= plan.claims().frames_start + plan.claims().frames_bytes
            );
            let artifact =
                emit(program).expect("new nonloan operation must pass full target audit");
            assert_eq!(
                artifact.bytes(),
                emit(program).expect("deterministic exact-seal emission").bytes()
            );
            runtime::ordinary(case, artifact.bytes());
            if provider == "native" {
                let observed =
                    emit_observed(program).expect("distinct fully audited observed artifact");
                runtime::observed_existing_allocation_faults(case, observed.as_bytes());
            }
        });
    }
}
