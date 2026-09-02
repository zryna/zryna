use super::*;

const OWNED_TYPES_SOURCE: &str = "interface OwnedBox extends ZrynaStruct { value: String; }\ninterface Node extends ZrynaStruct { children: Vec<Node>; }\nfunction inspect(a: Vec<String>, b: Vec<String>, box: OwnedBox, node: Node): i32 { const xs: Vec<String> = Vec<String>([\"x\"]); return 0; }";
const OWNED_TYPES_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":48,"end":54},"kind":{"kind":"string","keyword_span":{"file":0,"start":48,"end":54}}},{"span":{"file":0,"start":109,"end":113},"kind":{"kind":"named","name":{"text":"Node","span":{"file":0,"start":109,"end":113}}}},{"span":{"file":0,"start":105,"end":114},"kind":{"kind":"vec","keyword_span":{"file":0,"start":105,"end":108},"less_than_span":{"file":0,"start":108,"end":109},"argument":1,"greater_than_span":{"file":0,"start":113,"end":114}}},{"span":{"file":0,"start":142,"end":148},"kind":{"kind":"string","keyword_span":{"file":0,"start":142,"end":148}}},{"span":{"file":0,"start":138,"end":149},"kind":{"kind":"vec","keyword_span":{"file":0,"start":138,"end":141},"less_than_span":{"file":0,"start":141,"end":142},"argument":3,"greater_than_span":{"file":0,"start":148,"end":149}}},{"span":{"file":0,"start":158,"end":164},"kind":{"kind":"string","keyword_span":{"file":0,"start":158,"end":164}}},{"span":{"file":0,"start":154,"end":165},"kind":{"kind":"vec","keyword_span":{"file":0,"start":154,"end":157},"less_than_span":{"file":0,"start":157,"end":158},"argument":5,"greater_than_span":{"file":0,"start":164,"end":165}}},{"span":{"file":0,"start":172,"end":180},"kind":{"kind":"named","name":{"text":"OwnedBox","span":{"file":0,"start":172,"end":180}}}},{"span":{"file":0,"start":188,"end":192},"kind":{"kind":"named","name":{"text":"Node","span":{"file":0,"start":188,"end":192}}},{"span":{"file":0,"start":195,"end":198},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":195,"end":198}}},{"span":{"file":0,"start":215,"end":221},"kind":{"kind":"string","keyword_span":{"file":0,"start":215,"end":221}}},{"span":{"file":0,"start":211,"end":222},"kind":{"kind":"vec","keyword_span":{"file":0,"start":211,"end":214},"less_than_span":{"file":0,"start":214,"end":215},"argument":10,"greater_than_span":{"file":0,"start":221,"end":222}}},{"span":{"file":0,"start":229,"end":235},"kind":{"kind":"string","keyword_span":{"file":0,"start":229,"end":235}}},{"span":{"file":0,"start":225,"end":236},"kind":{"kind":"vec","keyword_span":{"file":0,"start":225,"end":228},"less_than_span":{"file":0,"start":228,"end":229},"argument":12,"greater_than_span":{"file":0,"start":235,"end":236}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":57},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"OwnedBox","span":{"file":0,"start":10,"end":18}},"extends_span":{"file":0,"start":19,"end":26},"marker_span":{"file":0,"start":27,"end":38},"open_brace_span":{"file":0,"start":39,"end":40},"close_brace_span":{"file":0,"start":56,"end":57},"fields":[{"span":{"file":0,"start":41,"end":55},"name":{"text":"value","span":{"file":0,"start":41,"end":46}},"colon_span":{"file":0,"start":46,"end":47},"semicolon_span":{"file":0,"start":54,"end":55},"type_syntax":0}]}},{"span":{"file":0,"start":58,"end":117},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":58,"end":67},"name":{"text":"Node","span":{"file":0,"start":68,"end":72}},"extends_span":{"file":0,"start":73,"end":80},"marker_span":{"file":0,"start":81,"end":92},"open_brace_span":{"file":0,"start":93,"end":94},"close_brace_span":{"file":0,"start":116,"end":117},"fields":[{"span":{"file":0,"start":95,"end":115},"name":{"text":"children","span":{"file":0,"start":95,"end":103}},"colon_span":{"file":0,"start":103,"end":104},"semicolon_span":{"file":0,"start":114,"end":115},"type_syntax":2}]}}],"functions":[{"span":{"file":0,"start":118,"end":256},"export_span":null,"function_span":{"file":0,"start":118,"end":126},"name":{"text":"inspect","span":{"file":0,"start":127,"end":134}},"parameters":[{"span":{"file":0,"start":135,"end":149},"name":{"text":"a","span":{"file":0,"start":135,"end":136}},"type_syntax":4},{"span":{"file":0,"start":151,"end":165},"name":{"text":"b","span":{"file":0,"start":151,"end":152}},"type_syntax":6},{"span":{"file":0,"start":167,"end":180},"name":{"text":"box","span":{"file":0,"start":167,"end":170}},"type_syntax":7},{"span":{"file":0,"start":182,"end":192},"name":{"text":"node","span":{"file":0,"start":182,"end":186}},"type_syntax":8}],"result_type":9,"body":{"span":{"file":0,"start":199,"end":256},"root_block":0,"blocks":[{"span":{"file":0,"start":199,"end":256},"open_brace_span":{"file":0,"start":199,"end":200},"statements":[0,1],"close_brace_span":{"file":0,"start":255,"end":256}}],"statements":[{"span":{"file":0,"start":201,"end":244},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":201,"end":206},"mutable":false,"name":{"text":"xs","span":{"file":0,"start":207,"end":209}},"type_syntax":11,"equals_span":{"file":0,"start":223,"end":224},"initializer":1,"semicolon_span":{"file":0,"start":243,"end":244}}},{"span":{"file":0,"start":245,"end":254},"kind":{"kind":"return","keyword_span":{"file":0,"start":245,"end":251},"value":2,"semicolon_span":{"file":0,"start":253,"end":254}}}],"expressions":[{"span":{"file":0,"start":238,"end":241},"kind":{"kind":"string-literal","spelling":"\"x\""}},{"span":{"file":0,"start":225,"end":243},"kind":{"kind":"vec-construction","type_syntax":13,"open_paren_span":{"file":0,"start":236,"end":237},"open_bracket_span":{"file":0,"start":237,"end":238},"elements":[0],"close_bracket_span":{"file":0,"start":241,"end":242},"close_paren_span":{"file":0,"start":242,"end":243}}},{"span":{"file":0,"start":252,"end":253},"kind":{"kind":"i32-literal","spelling":"0"}}]}}]}],"diagnostics":[]}}"#;
fn owned_types_snapshot() -> RawProjectSyntaxSnapshot {
    let response = OWNED_TYPES_RESPONSE
        .replacen(
            "\"end\":192}}},{\"span\":{\"file\":0,\"start\":195",
            "\"end\":192}}}},{\"span\":{\"file\":0,\"start\":195",
            1,
        )
        .replacen(
            "\"end\":198}}},{\"span\":{\"file\":0,\"start\":215",
            "\"end\":198}}}},{\"span\":{\"file\":0,\"start\":215",
            1,
        );
    response_snapshot(&response)
}

#[test]
fn public_aggregate_result_is_rejected() {
    let mut source = PAIR_SOURCE.to_owned();
    source.insert_str(50, "export ");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(decode_snapshot(PAIR_JSON).expect("Pair JSON"), 50, 7);
    let function = &mut raw.files[0].functions[0];
    function.span.start = 50;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 50, end: 56 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public aggregate result");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
}

#[test]
fn public_aggregate_parameter_is_rejected() {
    let mut source = ARRAY_VALID_SOURCE.to_owned();
    source.insert_str(0, "export ");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 0, 7);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "1".to_owned();
    let function = &mut raw.files[0].functions[0];
    function.span.start = 0;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 0, end: 6 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported array parameter");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public aggregate parameter");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
}

#[test]
fn semantic_diagnostics_replay_deterministically() {
    let sources = sources_for(OWNED_TYPES_SOURCE);
    let syntax = verify_snapshot(owned_types_snapshot(), &sources).expect("owned v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("first rejection");
    let second = lower(pair_input(&syntax, &sources)).expect_err("second rejection");
    let summarize = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|d| {
                (
                    d.code().to_owned(),
                    d.primary_span().map(|s| (s.start(), s.end())),
                    d.message().to_owned(),
                    d.guidance().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(summarize(&first), summarize(&second));
}

#[test]
fn authenticated_owned_types_have_canonical_sealed_capabilities() {
    let sources = sources_for(OWNED_TYPES_SOURCE);
    let syntax = verify_snapshot(owned_types_snapshot(), &sources).expect("owned v4");
    let input = pair_input(&syntax, &sources);
    let string = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let first_vec = authenticated_type_capabilities(input, 0, 4).expect("first Vec<String>");
    let second_vec = authenticated_type_capabilities(input, 0, 6).expect("second Vec<String>");
    let owned_box = authenticated_type_capabilities(input, 0, 7).expect("owned struct");
    let recursive_node = authenticated_type_capabilities(input, 0, 8).expect("recursive Vec node");
    let constructed_vec =
        authenticated_type_capabilities(input, 0, 13).expect("Vec construction type");

    assert_eq!(first_vec.layout, second_vec.layout);
    assert_eq!(first_vec.layout, constructed_vec.layout);
    for ty in [string, first_vec, owned_box, recursive_node] {
        assert!(!ty.is_copy());
        assert!(ty.is_clone());
        assert_ne!(ty.drop_kind, 0);
    }
    assert_ne!(string.runtime_kind, 0);
    assert_ne!(first_vec.runtime_kind, 0);
}

#[test]
fn unsupported_owned_vec_shape_uses_vec_diagnostic_family() {
    let sources = sources_for(OWNED_TYPES_SOURCE);
    let syntax = verify_snapshot(owned_types_snapshot(), &sources).expect("owned v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unsupported Vec shape");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3013");
}
