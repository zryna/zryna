use super::*;

const ENUM_CONSTRUCT_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction make(x: i32): Maybe { return Maybe.some(x); }";
const ENUM_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":83,"end":86},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":83,"end":86}}}},{"span":{"file":0,"start":89,"end":94},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":89,"end":94}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":120},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"make","span":{"file":0,"start":75,"end":79}},"parameters":[{"span":{"file":0,"start":80,"end":86},"name":{"text":"x","span":{"file":0,"start":80,"end":81}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":95,"end":120},"root_block":0,"blocks":[{"span":{"file":0,"start":95,"end":120},"open_brace_span":{"file":0,"start":95,"end":96},"statements":[0],"close_brace_span":{"file":0,"start":119,"end":120}}],"statements":[{"span":{"file":0,"start":97,"end":118},"kind":{"kind":"return","keyword_span":{"file":0,"start":97,"end":103},"value":1,"semicolon_span":{"file":0,"start":117,"end":118}}}],"expressions":[{"span":{"file":0,"start":115,"end":116},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":115,"end":116}}}},{"span":{"file":0,"start":104,"end":117},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":104,"end":109}},"dot_span":{"file":0,"start":109,"end":110},"variant":{"text":"some","span":{"file":0,"start":110,"end":114}},"open_paren_span":{"file":0,"start":114,"end":115},"payload":0,"close_paren_span":{"file":0,"start":116,"end":117}}}]}}]}],"diagnostics":[]}}"#;

#[test]
fn nonexhaustive_enum_match_is_rejected() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(114..137, "                       ");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &mut body.expressions[2].kind
    else {
        panic!("match")
    };
    arms.remove(0);
    arms[0].value = 1;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 2;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nonexhaustive match");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nonexhaustive match");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3009");
}

#[test]
fn v4_match_child_order_regression_authenticates_source_faithful_adapter_output() {
    let sources = sources_for(ENUM_SOURCE);
    verify_snapshot(response_snapshot(ENUM_RESPONSE), &sources)
        .expect("call-style match close parenthesis follows the complete arm object");
}

#[test]
fn enum_payload_binding_mismatch_is_rejected() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(152..157, "     ");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } =
        &mut raw.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("match")
    };
    arms[1].binding = None;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful payload mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("payload binding mismatch");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3009");
}

#[test]
fn reversed_enum_match_arms_lower_in_variant_ordinal_order() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(114..167, "\"Maybe.some\": (value) => value, \"Maybe.none\": () => 0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 139, end: 144 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "value".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 139, end: 144 },
            },
        },
    };
    body.expressions[2] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 166, end: 167 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &mut body.expressions[3].kind
    else {
        panic!("match")
    };
    arms.swap(0, 1);
    let some = &mut arms[0];
    some.span = zryna_source::UntrustedSpan { file: 0, start: 114, end: 144 };
    some.type_name.span = zryna_source::UntrustedSpan { file: 0, start: 115, end: 120 };
    some.dot_span = zryna_source::UntrustedSpan { file: 0, start: 120, end: 121 };
    some.variant.span = zryna_source::UntrustedSpan { file: 0, start: 121, end: 125 };
    let binding = some.binding.as_mut().expect("some binding");
    binding.span = zryna_source::UntrustedSpan { file: 0, start: 129, end: 134 };
    some.arrow_span = zryna_source::UntrustedSpan { file: 0, start: 136, end: 138 };
    some.value = 1;
    let none = &mut arms[1];
    none.span = zryna_source::UntrustedSpan { file: 0, start: 146, end: 167 };
    none.type_name.span = zryna_source::UntrustedSpan { file: 0, start: 147, end: 152 };
    none.dot_span = zryna_source::UntrustedSpan { file: 0, start: 152, end: 153 };
    none.variant.span = zryna_source::UntrustedSpan { file: 0, start: 153, end: 157 };
    none.arrow_span = zryna_source::UntrustedSpan { file: 0, start: 163, end: 165 };
    none.value = 2;

    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed match");
    let program = lower(pair_input(&syntax, &sources)).expect("reversed match must lower");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let entry = function.blocks().next().expect("entry");
    assert_eq!(
        entry.terminator().enum_arms().map(|arm| arm.variant()).collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn enum_constructor_payload_shape_type_and_name_are_exact() {
    let sources = sources_for(ENUM_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ENUM_CONSTRUCT_RESPONSE), &sources)
        .expect("enum constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact enum constructor");

    let mut absent_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    absent_source.replace_range(115..116, " ");
    let absent_sources = sources_for(&absent_source);
    let mut absent = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let body = &mut absent.files[0].functions[0].body;
    body.expressions.remove(0);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { payload, .. } =
        &mut body.expressions[0].kind
    else {
        panic!("enum constructor")
    };
    *payload = None;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 0;
    let syntax = verify_snapshot(absent, &absent_sources).expect("source-faithful absent payload");
    assert_eq!(
        lower(pair_input(&syntax, &absent_sources)).expect_err("absent payload")[0].code(),
        "ZRYNA-M3005"
    );

    let mut extra_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    extra_source.replace_range(110..114, "none");
    let extra_sources = sources_for(&extra_source);
    let mut extra = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut extra.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    variant.text = "none".to_owned();
    let syntax = verify_snapshot(extra, &extra_sources).expect("source-faithful extra payload");
    assert_eq!(
        lower(pair_input(&syntax, &extra_sources)).expect_err("extra payload")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(83..86, "bool");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ENUM_CONSTRUCT_RESPONSE), 86, 1);
    let RawTypeSyntaxKind::Named { name } = &mut typed.files[0].type_syntax[1].kind else {
        panic!("parameter type")
    };
    name.text = "bool".to_owned();
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped payload");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped payload child");
    assert_eq!((primary.start(), primary.end()), (116, 117));

    let mut variant_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    variant_source.replace_range(110..114, "nope");
    let variant_sources = sources_for(&variant_source);
    let mut variant_snapshot = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut variant_snapshot.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    variant.text = "nope".to_owned();
    let syntax = verify_snapshot(variant_snapshot, &variant_sources)
        .expect("source-faithful unknown variant");
    let diagnostics = lower(pair_input(&syntax, &variant_sources)).expect_err("unknown variant");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
    let primary = diagnostics[0].primary_span().expect("variant token");
    assert_eq!((primary.start(), primary.end()), (110, 114));

    let mut unknown_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    unknown_source.replace_range(104..109, "Other");
    let unknown_sources = sources_for(&unknown_source);
    let mut unknown = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { type_name, .. } =
        &mut unknown.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    type_name.text = "Other".to_owned();
    let syntax = verify_snapshot(unknown, &unknown_sources).expect("source-faithful unknown enum");
    assert_eq!(
        lower(pair_input(&syntax, &unknown_sources)).expect_err("unknown enum")[0].code(),
        "ZRYNA-M3005"
    );
}
