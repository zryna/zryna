//! Genuine v5 providers, original source, and independently issued NEW whole authority.
use super::source;
use zryna_ir::generic_v1::execution_v1::{VerifiedLoanOrigin, raw};

fn whole(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let mut loans = 0;
            let mut formal = 0;
            let mut begin = 0;
            let mut end = 0;
            for function in program.functions() {
                for loan in function.loans() {
                    loans += 1;
                    if matches!(loan.origin(), Some(VerifiedLoanOrigin::Formal(_))) {
                        formal += 1;
                    }
                }
                for operation in function
                    .blocks()
                    .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedBlock::operations)
                {
                    match operation.kind() {
                        raw::Operation::BeginLoan { .. } => begin += 1,
                        raw::Operation::EndLoan { .. } => end += 1,
                        _ => {}
                    }
                }
            }
            assert!(loans > 0, "genuine source loans survive closed replay");
            assert!(end > 0, "formal and lexical permissions end before return cleanup");
            assert!(formal > 0 || begin > 0, "entry authority or real BeginLoan required");
        });
    }
}
#[test]
fn formal_copy_mixed_parameters() {
    whole("formal-copy");
}
#[test]
fn opaque_generic_loan_forwarding() {
    whole("opaque-forward");
}
#[test]
fn lexical_shared_reborrow_and_referent_clone() {
    whole("shared-reborrow-clone");
}
#[test]
fn exclusive_reborrow_and_vector_append() {
    whole("exclusive-reborrow-push");
}
#[test]
fn exclusive_generic_replacement() {
    whole("generic-replace");
}
#[test]
fn precise_disjoint_static_place_identity() {
    whole("static-disjoint");
}
#[test]
fn conditional_lexical_cleanup_joins() {
    whole("branch-cleanup");
}
#[test]
fn fallible_referent_and_caller_cleanup() {
    whole("fallible-cleanup");
}
#[test]
fn every_admitted_stored_referent_shape() {
    whole("referent-catalogue");
}

#[test]
fn explicit_execution_roles_cannot_enter_legacy_issuers() {
    use zryna_semantics::bounded_generics_v1::{self as semantics, body_types, instantiation};
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project("formal-copy", provider);
        let entry = sources.verify_file_id(0).expect("actual entry");
        let declarations = semantics::resolve_declarations(
            semantics::SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
        )
        .expect("original declarations");
        let body_types::BodyTypeFailure::Diagnostics(legacy) =
            body_types::check_body_types(&declarations)
                .expect_err("legacy role admission is unchanged")
        else {
            panic!("the legacy body checker owns its original mismatch");
        };
        assert!(legacy.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3007"));
        let execution = body_types::check_execution_body_types(&declarations)
            .expect("same declaration brand with explicit execution loan roles");
        assert!(std::ptr::eq(execution.declarations(), &raw const declarations));
        assert!(matches!(
            instantiation::discover(&execution),
            Err(instantiation::InstantiationFailure::InternalFailure)
        ));
        assert!(matches!(
            instantiation::owned_v4::discover(&execution),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        assert!(matches!(
            instantiation::owned_v5::discover(&execution),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        instantiation::execution_v1::discover(&execution)
            .expect("new execution domain accepts its own mode");
    }
}
