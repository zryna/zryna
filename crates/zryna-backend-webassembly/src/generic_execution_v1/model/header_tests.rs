//! Own-phase credits fund actual retained function visits before the first body lookup.
use super::*;
thread_local! { static HEADERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
pub(super) fn visit() {
    HEADERS.with(|count| count.set(count.get() + 1));
}
fn reset() {
    HEADERS.with(|count| count.set(0));
}
fn seen() -> usize {
    HEADERS.with(std::cell::Cell::get)
}
fn with_model(check: impl FnOnce(&Model<'_, '_, '_>)) {
    crate::generic_execution_v1::tests::loan_program("formal-copy", |program, _| {
        let plan = memory::verify(memory::derive(program, false).expect("genuine claim"), program)
            .expect("same-program memory");
        let model = Model::new(program, &plan).expect("actual source-bound model");
        check(&model);
    });
}
fn expected(functions: &[VerifiedFunction<'_, '_>]) -> usize {
    functions.len()
        + functions
            .iter()
            .map(|function| {
                function.blocks().len()
                    + function.blocks().map(|block| block.operations().len() * 2).sum::<usize>()
            })
            .sum::<usize>()
}
#[test]
fn function_feature_scan_pays_its_own_headers_at_exact_total_credit() {
    with_model(|model| {
        let cost = expected(&model.functions);
        let mut work = 0;
        reset();
        let baseline = continuations(&model.functions, &mut work).expect("actual original scan");
        assert_eq!(baseline, (model.continuation, model.concat));
        assert_eq!(work, cost, "block/operation credits cannot be reused for function headers");
        assert_eq!(seen(), model.functions.len());
        let mut work = crate::generic_execution_v1::MAX_BYTES - cost;
        continuations(&model.functions, &mut work).expect("exact own-phase credit");
        assert_eq!(work, crate::generic_execution_v1::MAX_BYTES);
        let mut work = crate::generic_execution_v1::MAX_BYTES - cost + 1;
        assert_eq!(
            continuations(&model.functions, &mut work)
                .expect_err("first missing total credit")
                .code(),
            "ZRYNA-W4101"
        );
        let mut work = 0;
        continuations(&model.functions, &mut work).expect("pristine retry");
        assert_eq!(work, cost);
    });
}
#[test]
fn missing_function_header_credit_rejects_before_any_retained_function_visit() {
    with_model(|model| {
        assert!(!model.functions.is_empty());
        for start in
            [crate::generic_execution_v1::MAX_BYTES - model.functions.len() + 1, usize::MAX]
        {
            let mut work = start;
            reset();
            assert_eq!(
                continuations(&model.functions, &mut work)
                    .expect_err("header debit or checked overflow")
                    .code(),
                "ZRYNA-W4101"
            );
            assert_eq!(seen(), 0, "no header may be visited before its own debit");
            assert_eq!(work, start, "first debit failure is atomic");
        }
        reset();
        let mut work = 0;
        continuations(&model.functions, &mut work).expect("no failed state escapes");
        assert_eq!(seen(), model.functions.len());
        assert_eq!(work, expected(&model.functions));
    });
}
