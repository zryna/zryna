//! Actual source-bound partial initialization pays its own repeated emission work.
use super::*;
#[path = "initialize_tests/source.rs"]
mod source;

#[test]
fn genuine_static_refill_exact_credit_first_extra_and_retry_emit_no_unpaid_artifact() {
    for provider in source::PROVIDERS {
        source::with_program("array-refill", provider, |program, _| {
            let context = Context::new(program).expect("genuine source/type/state/cleanup seal");
            let (function, target, value) = context
                .functions
                .iter()
                .find_map(|function| {
                    function.blocks.iter().flat_map(|block| block.operations()).find_map(|op| {
                        let raw::Operation::InitializePlace { place, value } = op.kind() else {
                            return None;
                        };
                        let target = function.place(*place).expect("same-seal place");
                        target.parent().is_some().then_some((function, target, *value))
                    })
                })
                .expect("actual affine array hole refill operation");
            let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
            context.work.set(0);
            let mut measured = Writer::counter(limit);
            emit(&context, function, target, value, &mut measured).expect("actual partial emit");
            let cost = context.work.get();
            let bytes = measured.bytes();
            assert!(cost > 0 && bytes > 0);
            context.work.set(limit - cost);
            let mut exact = Writer::counter(bytes);
            emit(&context, function, target, value, &mut exact).expect("exact work/output credit");
            assert_eq!(context.work.get(), limit);
            assert_eq!(exact.bytes(), bytes);
            context.work.set(limit - cost + 1);
            let mut short = Writer::counter(bytes);
            assert_eq!(
                emit(&context, function, target, value, &mut short)
                    .expect_err("first extra emission work")
                    .code(),
                "ZRYNA-J2003"
            );
            for start in [limit, usize::MAX] {
                context.work.set(start);
                let mut denied = Writer::counter(bytes);
                assert_eq!(
                    emit(&context, function, target, value, &mut denied)
                        .expect_err("first debit rejects before emission")
                        .code(),
                    "ZRYNA-J2003"
                );
                assert_eq!(context.work.get(), start);
                assert_eq!(denied.bytes(), 0);
            }
            context.work.set(0);
            let mut retry = Writer::counter(bytes);
            emit(&context, function, target, value, &mut retry).expect("pristine retry");
            assert_eq!(context.work.get(), cost);
            assert_eq!(retry.bytes(), bytes);
        });
    }
}
