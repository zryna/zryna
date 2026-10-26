//! Genuine indexed source views reach actual Wasm owners without another target masking RED.
use super::{artifact, code, memory, model};
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, raw};
mod attacks;
mod continuation_negative;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_indexed_continuations_source_seal/source.rs"]
mod continuations;
mod controls;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_dynamic_index_source_seal/negative.rs"]
mod negative;
mod runtime;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
pub(super) fn program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&VerifiedExecution<'_>, &[u8]),
) {
    if matches!(case, "called-indexed-shared-copy" | "indexed-push-copy") {
        assert!(!continuations::text(case).is_empty());
        continuations::with_program(case, provider, |program, _, wire| check(program, wire));
    } else {
        assert!(!source::text(case).is_empty());
        source::with_program(case, provider, |program, _, wire| check(program, wire));
    }
}
fn actual(case: &str, observed: bool) {
    for provider in source::PROVIDERS {
        program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let plan = memory::verify(
                memory::derive(program, observed)
                    .expect("genuine indexed memory owner, no fake Place"),
                program,
            )
            .expect("independent exact indexed pointer cells");
            let model = model::Model::new(program, &plan).expect("same-seal indexed model");
            controls::identities(&model, case);
            let code = code::build(&model).expect("actual indexed Begin bounds/address/cleanup");
            let bytes = artifact::encode(&model, &code).expect("actual module");
            wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::WASM1)
                .validate_all(&bytes)
                .expect("actual core Wasm");
            artifact::independent_audit(&bytes, &model).expect("independent decoded indexed roles");
            artifact::audit(&bytes, &model, &code).expect("full artifact owner");
            let public = if observed {
                crate::generic_execution_v1::emit_observed(program)
                    .expect("observed public emitter")
                    .as_bytes()
                    .to_vec()
            } else {
                crate::generic_execution_v1::emit(program)
                    .expect("ordinary public emitter")
                    .bytes()
                    .to_vec()
            };
            assert_eq!(bytes, public);
            runtime::check(case, provider, wire, &bytes, observed);
        });
    }
}
macro_rules! cases{($($ordinary:ident,$observed:ident,$case:literal;)*)=>{$(#[test]fn $ordinary(){actual($case,false);}#[test]fn $observed(){actual($case,true);})*};}
cases! {
 ordinary_array_exclusive_copy,observed_array_exclusive_copy,"array-exclusive-copy";
 ordinary_array_rhs_match,observed_array_rhs_match,"array-indexed-rhs-match";
 ordinary_array_shared_copy,observed_array_shared_copy,"array-shared-copy";
 ordinary_chained_shared_copy,observed_chained_shared_copy,"chained-shared-copy";
 ordinary_copy_read_control,observed_copy_read_control,"copy-read-control";
 ordinary_index_once_rhs,observed_index_once_rhs,"index-once-before-rhs";
 ordinary_vec_exclusive_replace,observed_vec_exclusive_replace,"vec-exclusive-replace";
 ordinary_vec_indexed_replace,observed_vec_indexed_replace,"vec-indexed-replace";
 ordinary_vec_shared_clone,observed_vec_shared_clone,"vec-shared-clone";
 ordinary_zero_length_bounds,observed_zero_length_bounds,"zero-length-bounds";
 ordinary_called_indexed,observed_called_indexed,"called-indexed-shared-copy";
 ordinary_indexed_push,observed_indexed_push,"indexed-push-copy";
}

mod temporal;
