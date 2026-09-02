use super::*;

const ARRAY_CONSTRUCT_SOURCE: &str =
    "function make(): FixedArray<i32, 2> { return FixedArray<i32, 2>([1, 2]); }";
const ARRAY_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":56,"end":59},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":56,"end":59}}}},{"span":{"file":0,"start":45,"end":63},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":45,"end":55},"less_than_span":{"file":0,"start":55,"end":56},"element":2,"comma_span":{"file":0,"start":59,"end":60},"length_span":{"file":0,"start":61,"end":62},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":62,"end":63}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":74},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":36,"end":74},"root_block":0,"blocks":[{"span":{"file":0,"start":36,"end":74},"open_brace_span":{"file":0,"start":36,"end":37},"statements":[0],"close_brace_span":{"file":0,"start":73,"end":74}}],"statements":[{"span":{"file":0,"start":38,"end":72},"kind":{"kind":"return","keyword_span":{"file":0,"start":38,"end":44},"value":2,"semicolon_span":{"file":0,"start":71,"end":72}}}],"expressions":[{"span":{"file":0,"start":65,"end":66},"kind":{"kind":"i32-literal","spelling":"1"}},{"span":{"file":0,"start":68,"end":69},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":45,"end":71},"kind":{"kind":"fixed-array-construction","type_syntax":3,"open_paren_span":{"file":0,"start":63,"end":64},"open_bracket_span":{"file":0,"start":64,"end":65},"elements":[0,1],"close_bracket_span":{"file":0,"start":69,"end":70},"close_paren_span":{"file":0,"start":70,"end":71}}}]}}]}],"diagnostics":[]}}"#;

#[test]
fn fixed_array_constant_projection_accepts_last_index() {
    let sources = sources_for(ARRAY_VALID_SOURCE);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index literal")
    };
    *spelling = "1".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful fixed array v4");
    let program = lower(pair_input(&syntax, &sources)).expect("last fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 1, .. }
    )));
}

#[test]
fn fixed_array_constant_projection_accepts_zero_index() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "0".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful zero index");
    let program = lower(pair_input(&syntax, &sources)).expect("zero fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(
        function.parameters().len()
            + block.parameters().len()
            + block.instructions().filter(|instruction| instruction.result().is_some()).count(),
        2,
        "fixed-array constant index spelling is not emitted as a runtime value",
    );
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 0, .. }
    )));
}

#[test]
fn fixed_array_constructor_requires_exact_count_and_element_type() {
    let sources = sources_for(ARRAY_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), &sources)
        .expect("array constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact fixed-array constructor");

    let mut missing_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    missing_source.replace_range(66..69, "   ");
    let missing_sources = sources_for(&missing_source);
    let mut missing = response_snapshot(ARRAY_CONSTRUCT_RESPONSE);
    let body = &mut missing.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut body.expressions[1].kind
    else {
        panic!("array constructor")
    };
    elements.pop();
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 1;
    let syntax = verify_snapshot(missing, &missing_sources).expect("source-faithful short array");
    assert_eq!(
        lower(pair_input(&syntax, &missing_sources)).expect_err("short array")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(65..66, "true");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), 66, 3);
    typed.files[0].functions[0].body.expressions[0].kind =
        zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true };
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped array");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped array");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped element child");
    assert_eq!((primary.start(), primary.end()), (65, 69));
}

#[test]
fn fixed_array_index_equal_to_length_is_rejected() {
    let sources = sources_for(ARRAY_OOB_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_RESPONSE), &sources)
        .expect("source-faithful fixed array v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("index N is out of bounds");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn dynamic_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "x");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1].kind =
        zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "x".to_owned(), span },
        };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful dynamic index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("dynamic index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("dynamic index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn negative_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "-1");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 55, 1);
    let body = &mut raw.files[0].functions[0].body;
    let mut index = body.expressions.pop().expect("index expression");
    let literal = &mut body.expressions[1];
    literal.span.start = 55;
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } = &mut literal.kind else {
        panic!("literal")
    };
    *spelling = "1".to_owned();
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 56 },
        kind: zryna_syntax::v4::RawExpressionKind::Negation {
            operator_span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 55 },
            operand: 1,
        },
    });
    let zryna_syntax::v4::RawExpressionKind::Index { index: index_id, .. } = &mut index.kind else {
        panic!("index")
    };
    *index_id = 2;
    body.expressions.push(index);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 3;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful negative index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("negative index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("negative index child");
    assert_eq!((primary.start(), primary.end()), (54, 56));
}
