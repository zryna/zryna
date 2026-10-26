//! Existing genuine control3 source: ordinary public rows and explicitly untrusted private transport.
mod harness;
mod hostiles;
mod ordinary;
mod private_source;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_native_accounting_control/source.rs"]
mod source;
use crate::generic_execution_v1::{memory, model::Model};
pub(super) fn with_program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    source::with_program(case, provider, check);
}
fn with_model(case: &str, provider: &str, check: impl FnOnce(&Model<'_, '_, '_>, &[u8])) {
    with_program(case, provider, |program, wire| {
        let memory = memory::verify(
            memory::derive(program, false).expect("genuine source memory claim"),
            program,
        )
        .expect("same-seal memory owner");
        let model = Model::new(program, &memory).expect("actual same-program Wasm model");
        check(&model, wire);
    });
}

mod adapter_attacks;
mod held;
mod status_sets;
