//! TLS selected-invocation proof over existing genuine recursive originals.
use crate::generic_execution_v1::invocation::{plan::Plan, wrapper};
use zryna_abi::{Invocation, ScalarType, ScalarValue};
#[test]
fn genuine_recursive_invoker_binds_reset_before_scalar_publication() {
    for case in ["drop-only", "clone", "controlled-cleanup"] {
        super::source::with_program(case, |p| {
            let mir = zryna_native_mir::generic_execution_v1::lower(p).expect("real MIR");
            assert!(mir.requires_recursive_arena());
            let export = p.scalar_abi().exports().next().expect("scalar entry");
            let arguments = export
                .parameters()
                .iter()
                .map(|ty| match ty {
                    ScalarType::Bool => ScalarValue::Bool(false),
                    ScalarType::I32 => ScalarValue::I32(0),
                })
                .collect();
            let request = Invocation::new(export.logical_name().as_str().to_owned(), arguments);
            let plan = Plan::derive(&mir, request).expect("actual same-owner invocation plan");
            wrapper::tests::check(&mir, &plan);
        });
    }
}
