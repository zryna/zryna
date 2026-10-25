//! Source originals own the exact scoped dynamic borrowed-match continuation.
#[path = "generic_execution_v1_indexed_borrowed_match_source_seal/checks.rs"]
mod checks;
#[path = "generic_execution_v1_indexed_borrowed_match_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_indexed_borrowed_match_source_seal/source.rs"]
mod source;
fn whole(case: &str) {
    for provider in source::PROVIDERS {
        assert!(!source::text(case).is_empty());
        let (sources, syntax) = source::project(case, provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect("independent symbolic original permission before closed composition");
        source::with_program(case, provider, |program, claim, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert_eq!(program.functions().len(), claim.functions.len());
            checks::claim(case, claim);
        });
    }
}
#[test]
fn indexed_nominal_shared_own_join() {
    whole("indexed-nominal-shared-join");
}
#[test]
fn indexed_option_exclusive_own_join() {
    whole("indexed-option-exclusive-join");
}
#[test]
fn chained_indexed_shared_own_join() {
    whole("indexed-chain-nominal-shared-join");
}
