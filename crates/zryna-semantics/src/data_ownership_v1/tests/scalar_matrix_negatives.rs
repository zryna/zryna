use super::*;
use zryna_diagnostics::Diagnostic;

#[test]
fn mixed_scalar_inferred_output_mismatch_uses_reviewed_scalar_expression_diagnostic() {
    for count_mismatch in [true, false] {
        let (source, snapshot) = if count_mismatch {
            fixture(BOOL_EQ, BOOL_EQ)
        } else {
            fixture(ARITHMETIC, ARITHMETIC)
        };
        // Fixed arena: count mismatch is Equal at4; flag mismatch is Add at17.
        let result = if count_mismatch { 4 } else { 17 };
        let operation_span = snapshot.files[0].functions[0].body.expressions[result].span;
        assert_eq!(
            &source[usize::try_from(operation_span.start).expect("start")
                ..usize::try_from(operation_span.end).expect("end")],
            if count_mismatch { "true === false" } else { "7 - 3 * 2 + -1" }
        );
        let sources = sources_for(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("authenticated result mismatch");
        // Reviewed NEW mixed-scalar boundary; legacy Copy struct-field diagnostics differ.
        let expected = [Diagnostic::error_at(
            "ZRYNA-M3007",
            span(&sources, operation_span),
            "scalar result has a different exact aggregate type",
            "use a value with the exact declared type",
        )];
        for _ in 0..2 {
            let actual = lower(pair_input(&syntax, &sources))
                .expect_err("inferred scalar result does not fit declared context");
            assert_eq!(actual, expected);
        }
    }
}
