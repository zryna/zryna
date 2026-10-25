//! Private NEW transport contract; no fabricated positive execution program.
use super::signature;
use cranelift_codegen::{
    ir::{AbiParam, types},
    isa::CallConv,
};

#[test]
fn private_source_call_signature_appends_shared_invocation_context() {
    for source_parameters in [0_u32, 1, 2, 8] {
        let actual = signature(source_parameters);
        let original_slots = usize::try_from(source_parameters).expect("small fixture") + 1;
        assert_eq!(
            actual.params.len(),
            original_slots + 1,
            "NEW private call needs output, unchanged source arguments, then shared context"
        );
        assert_eq!(actual.call_conv, CallConv::SystemV);
        for index in 0..original_slots {
            assert_eq!(
                actual.params[index],
                AbiParam::new(types::I64),
                "result slot zero and formal loan ordinal plus one stay unchanged"
            );
        }
        assert_eq!(actual.params[original_slots], AbiParam::new(types::I64));
        assert_eq!(actual.returns, vec![AbiParam::new(types::I32)]);
    }
}
