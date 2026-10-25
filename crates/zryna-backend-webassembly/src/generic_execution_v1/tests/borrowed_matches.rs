//! Authenticated borrowed selections reach the Wasm byte owners without the JS target gate.
use super::{artifact, code, memory, model};
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, raw};
mod attacks;
mod authority;
mod budget;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_borrowed_match_source_seal/negative.rs"]
mod negative;
mod runtime;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_borrowed_match_source_seal/source.rs"]
mod source;
fn actual(case: &str) {
    assert_eq!(source::expected()["positive_cases"], 11);
    assert_eq!(source::expected()["hostile_cases"], 5);
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(!wire.is_empty(), "complete genuine execution carrier");
            for observed in [false, true] {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("genuine target address claim"),
                    program,
                )
                .expect("independently checked same-seal geometry");
                let model = model::Model::new(program, &plan).expect("retained source views");
                authority::check(&model);
                let code = code::build(&model).expect("actual selected payload loan lowering");
                let bytes =
                    artifact::encode(&model, &code).expect("actual ordinary/observed bytes");
                wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::WASM1)
                    .validate_all(&bytes)
                    .expect("actual target remains valid core Wasm");
                artifact::independent_audit(&bytes, &model)
                    .expect("decoded addresses, selected roles and cleanup from retained proof");
                artifact::audit(&bytes, &model, &code)
                    .expect("complete final sections, frame and decoded role owners");
                let public = if observed {
                    crate::generic_execution_v1::emit_observed(program)
                        .expect("observed public owner")
                        .as_bytes()
                        .to_vec()
                } else {
                    crate::generic_execution_v1::emit(program)
                        .expect("ordinary public owner")
                        .bytes()
                        .to_vec()
                };
                assert_eq!(public, bytes, "same actual body owners, not a separate test emitter");
                runtime::check(case, &bytes, observed);
            }
        });
    }
}
macro_rules! cases { ($($name:ident => $case:literal;)*) => { $(
    #[test] fn $name() { actual($case); }
)* }; }
cases! {
    nominal_shared_copy => "nominal-shared-copy";
    nominal_shared_clone => "nominal-shared-clone";
    nominal_exclusive_copy => "nominal-exclusive-copy";
    nominal_exclusive_replace => "nominal-exclusive-replace";
    option_shared_both => "option-shared-both";
    result_shared_opaque_bound => "result-shared-opaque-bound";
    result_exclusive_both => "result-exclusive-both";
    opaque_payload_forward => "opaque-payload-forward";
    nested_exclusive_result => "nested-exclusive-result";
    branch_arm_cleanup => "branch-arm-cleanup";
    shared_reborrow_from_exclusive => "shared-reborrow-from-exclusive";
}
