use super::*;

#[test]
fn cumulative_string_byte_budget_accepts_exact_and_rejects_plus_one() {
    assert!(!string_byte_budget_violation(
        zryna_ir::data_ownership_v1::MAX_STRING_LITERAL_BYTES - 1,
        1,
    ));
    assert!(
        string_byte_budget_violation(zryna_ir::data_ownership_v1::MAX_STRING_LITERAL_BYTES, 1,)
    );
}

#[test]
fn public_string_literal_result_remains_outside_scalar_abi() {
    let source = format!("export {STRING_SOURCE}");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(STRING_RESPONSE), 0, 7);
    let function = &mut raw.files[0].functions[0];
    function.span.start = 0;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 0, end: 6 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful public String v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public owned ABI");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert!(diagnostics[0].primary_span().is_some());
}

#[test]
fn unauthenticated_string_spelling_cannot_activate_literal_lowering() {
    let sources = sources_for(STRING_SOURCE);
    let mut raw = response_snapshot(STRING_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling } =
        &mut raw.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("String literal")
    };
    *spelling = "'x'".to_owned();
    assert!(verify_snapshot(raw, &sources).is_err());
}
