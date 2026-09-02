use super::*;

#[test]
fn struct_unknown_and_missing_field_is_rejected_at_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(137..141, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[0].kind else {
        panic!("left reference")
    };
    name.text = "nope".to_owned();
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("Pair constructor")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[0].kind
    else {
        panic!("shorthand")
    };
    name.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unknown field");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown/missing field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
    assert_eq!(diagnostics[0].primary_span().expect("initializer span").start(), 137);
}

#[test]
fn duplicate_struct_field_is_rejected_at_later_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(143..148, "left ");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[1].kind else {
        panic!("right reference")
    };
    name.text = "left".to_owned();
    name.span.end = 147;
    expressions[1].span.end = 147;
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("Pair constructor")
    };
    fields[1].span.end = 147;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[1].kind
    else {
        panic!("shorthand")
    };
    name.text = "left".to_owned();
    name.span.end = 147;
    let diagnostics = verify_snapshot(raw, &sources)
        .expect_err("v4 rejects duplicate initializer names before semantics");
    assert_eq!(diagnostics[0].code(), "ZRYNA-Y4002");
}

#[test]
fn unknown_field_access_is_rejected_at_the_use_not_declaration() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(165..169, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { field, .. } =
        &mut raw.files[0].functions[0].body.expressions[4].kind
    else {
        panic!("field access")
    };
    field.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unknown field access");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    assert_eq!(diagnostics[0].primary_span().expect("field use").start(), 165);
}

#[test]
fn by_value_recursive_struct_is_rejected_by_layout_authority() {
    let mut source = PAIR_SOURCE.to_owned();
    source.replace_range(43..46, "Pair");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(decode_snapshot(PAIR_JSON).expect("Pair JSON"), 46, 1);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[0].kind else {
        panic!("field type")
    };
    name.text = "Pair".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("by-value recursion");
    assert!(diagnostics[0].code().starts_with("ZRYNA-L3"));
}

#[test]
fn fixed_array_mediated_recursive_struct_is_rejected_by_layout_authority() {
    let text = "interface Loop extends ZrynaStruct { items: FixedArray<Loop, 1>; }\n";
    let sources = sources_for(text);
    let span = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        diagnostics: Vec::new(),
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![
                RawTypeSyntax {
                    span: span(55, 59),
                    kind: RawTypeSyntaxKind::Named {
                        name: RawIdentifierSyntax { text: "Loop".to_owned(), span: span(55, 59) },
                    },
                },
                RawTypeSyntax {
                    span: span(44, 63),
                    kind: RawTypeSyntaxKind::FixedArray {
                        keyword_span: span(44, 54),
                        less_than_span: span(54, 55),
                        element: 0,
                        comma_span: span(59, 60),
                        length_span: span(61, 62),
                        length_spelling: "1".to_owned(),
                        length: 1,
                        greater_than_span: span(62, 63),
                    },
                },
            ],
            data_declarations: vec![RawDataDeclaration {
                span: span(0, 66),
                export_span: None,
                kind: RawDataDeclarationKind::Struct {
                    interface_span: span(0, 9),
                    name: RawIdentifierSyntax { text: "Loop".to_owned(), span: span(10, 14) },
                    extends_span: span(15, 22),
                    marker_span: span(23, 34),
                    open_brace_span: span(35, 36),
                    fields: vec![RawDataField {
                        span: span(37, 64),
                        name: RawIdentifierSyntax { text: "items".to_owned(), span: span(37, 42) },
                        colon_span: span(42, 43),
                        type_syntax: 1,
                        semicolon_span: span(63, 64),
                    }],
                    close_brace_span: span(65, 66),
                },
            }],
            functions: Vec::new(),
        }],
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array recursion");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("array-mediated recursion");
    assert!(diagnostics[0].code().starts_with("ZRYNA-L3"));
}

#[test]
fn mistyped_struct_initializer_is_rejected_at_the_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(99..102, "bool");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 102, 1);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[3].kind else {
        panic!("right parameter type")
    };
    name.text = "bool".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful mistyped Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mistyped initializer");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    assert!(diagnostics[0].primary_span().is_some());
}

#[test]
fn portable_field_name_collision_is_rejected() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(48..53, "LEFT ");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let RawDataDeclarationKind::Struct { fields, .. } = &mut raw.files[0].data_declarations[0].kind
    else {
        panic!("Pair struct")
    };
    fields[1].name.text = "LEFT".to_owned();
    fields[1].name.span.end = 52;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful case collision");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("portable collision");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("later field").start(), 48);
}
