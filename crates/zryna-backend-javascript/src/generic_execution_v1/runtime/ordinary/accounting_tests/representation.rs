//! Context shape, private identity and hostile inherited properties remain unchanged.
use super::accounting;
use crate::generic_execution_v1::accounting_tests;
#[test]
fn private_context_retains_null_prototype_sealed_fields_and_exclusive_active_identity() {
    accounting_tests::with_program("empty-call-loop", "native", |program, _| {
        assert!(program.operations().any(|op| matches!(
            op.kind(),
            zryna_ir::generic_v1::execution_v1::raw::Operation::StringFromUtf8 { .. }
        )));
        let text = format!("{}\n{}", accounting::LEDGER, include_str!("representation.mjs"));
        let result = accounting_tests::private_boundary("context-representation", &text);
        assert_eq!(result["samePrivateContextContract"], true);
        assert_eq!(result["prototypeGetterSetterCalls"], 0);
    });
}
