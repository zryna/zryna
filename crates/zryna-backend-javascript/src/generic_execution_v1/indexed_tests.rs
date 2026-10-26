//! Genuine corrected originals reach JS through the existing independently issued NEW seal.
mod continuation_negatives;
#[path = "../../../zryna-driver/tests/generic_execution_v1_indexed_continuations_source_seal/source.rs"]
mod continuations;
#[path = "../../../zryna-driver/tests/generic_execution_v1_dynamic_index_source_seal/negative.rs"]
mod negative;
mod run;
#[path = "../../../zryna-driver/tests/generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
fn with_program(case: &str, provider: &str, check: impl FnOnce(&VerifiedExecution<'_>)) {
    if matches!(case, "called-indexed-shared-copy" | "indexed-push-copy") {
        assert!(!continuations::text(case).is_empty());
        continuations::with_program(case, provider, |program, _, _| check(program));
    } else {
        assert!(!source::text(case).is_empty());
        source::with_program(case, provider, |program, _, _| check(program));
    }
}
fn ordinary(case: &str) {
    for provider in source::PROVIDERS {
        with_program(case, provider, |program| run::execute(program, case, provider));
    }
}
fn observed_owner(case: &str) {
    for provider in source::PROVIDERS {
        with_program(case, provider, |program| {
            let artifact =
                super::emit_observed(program).expect("genuine indexed observed emitter owner");
            assert_eq!(
                artifact,
                super::emit_observed(program).expect("deterministic observed retry")
            );
            assert!(!artifact.as_source().is_empty());
        });
    }
}
macro_rules! cases {($($ordinary:ident,$observed:ident,$case:literal;)*)=>{$(
    #[test] fn $ordinary(){ordinary($case);}
    #[test] fn $observed(){observed_owner($case);}
)*};}
cases! {
    ordinary_array_exclusive_copy,observed_array_exclusive_copy,"array-exclusive-copy";
    ordinary_array_indexed_rhs_match,observed_array_indexed_rhs_match,"array-indexed-rhs-match";
    ordinary_array_shared_copy,observed_array_shared_copy,"array-shared-copy";
    ordinary_chained_shared_copy,observed_chained_shared_copy,"chained-shared-copy";
    ordinary_copy_read_control,observed_copy_read_control,"copy-read-control";
    ordinary_index_once_before_rhs,observed_index_once_before_rhs,"index-once-before-rhs";
    ordinary_vec_exclusive_replace,observed_vec_exclusive_replace,"vec-exclusive-replace";
    ordinary_vec_indexed_replace,observed_vec_indexed_replace,"vec-indexed-replace";
    ordinary_vec_shared_clone,observed_vec_shared_clone,"vec-shared-clone";
    ordinary_zero_length_bounds,observed_zero_length_bounds,"zero-length-bounds";
    ordinary_called_indexed_shared_copy,observed_called_indexed_shared_copy,"called-indexed-shared-copy";
    ordinary_indexed_push_copy,observed_indexed_push_copy,"indexed-push-copy";
}
#[test]
fn indexed_output_limit_exact_and_one_short_are_atomic() {
    with_program("array-shared-copy", "native", |program| {
        let artifact = super::emit(program).expect("genuine indexed output sizing");
        let bytes = artifact.source.len();
        assert_eq!(artifact, super::emit_with_budget(program, bytes).expect("exact output budget"));
        let error =
            super::emit_with_budget(program, bytes - 1).expect_err("one byte short rejects");
        assert_eq!(error.code(), "ZRYNA-J2003");
        assert_eq!(artifact, super::emit(program).expect("pristine retry after failed output"));
    });
}

mod observed;
