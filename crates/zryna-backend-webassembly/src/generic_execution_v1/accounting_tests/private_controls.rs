//! Actual adapter bytes under explicit private seeds; harness is untrusted and never a source seal.
use crate::generic_execution_v1::{artifact, code, memory, model::Model};
mod attacks;
pub(super) mod harness;
use harness::Harness;
fn with_empty(check: impl FnOnce(&Model<'_, '_, '_>, &[u8])) {
    with_case("empty-call-loop", check);
}
fn with_case(case: &str, check: impl FnOnce(&Model<'_, '_, '_>, &[u8])) {
    super::source::with_program(case, "native", |program, wire| {
        let memory =
            memory::verify(memory::derive(program, false).expect("genuine memory claim"), program)
                .expect("independent owned memory");
        let model = Model::new(program, &memory).expect("genuine String model");
        assert!(model.accounted(), "actual same-seal String accounting must activate");
        check(&model, wire);
    });
}

mod held;
mod roles;
