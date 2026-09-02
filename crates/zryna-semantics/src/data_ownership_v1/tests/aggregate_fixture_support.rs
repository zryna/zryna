use super::*;

pub(super) const ENUM_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction get(x: Maybe): i32 { return match(x, { \"Maybe.none\": () => 0, \"Maybe.some\": (value) => value }); }";
pub(super) const ENUM_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":82,"end":87},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":82,"end":87}}}},{"span":{"file":0,"start":90,"end":93},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":90,"end":93}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":173},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"get","span":{"file":0,"start":75,"end":78}},"parameters":[{"span":{"file":0,"start":79,"end":87},"name":{"text":"x","span":{"file":0,"start":79,"end":80}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":94,"end":173},"root_block":0,"blocks":[{"span":{"file":0,"start":94,"end":173},"open_brace_span":{"file":0,"start":94,"end":95},"statements":[0],"close_brace_span":{"file":0,"start":172,"end":173}}],"statements":[{"span":{"file":0,"start":96,"end":171},"kind":{"kind":"return","keyword_span":{"file":0,"start":96,"end":102},"value":3,"semicolon_span":{"file":0,"start":170,"end":171}}}],"expressions":[{"span":{"file":0,"start":109,"end":110},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":109,"end":110}}}},{"span":{"file":0,"start":134,"end":135},"kind":{"kind":"i32-literal","spelling":"0"}},{"span":{"file":0,"start":162,"end":167},"kind":{"kind":"reference","name":{"text":"value","span":{"file":0,"start":162,"end":167}}}},{"span":{"file":0,"start":103,"end":170},"kind":{"kind":"match","keyword_span":{"file":0,"start":103,"end":108},"open_paren_span":{"file":0,"start":108,"end":109},"scrutinee":0,"close_paren_span":{"file":0,"start":169,"end":170},"open_brace_span":{"file":0,"start":112,"end":113},"arms":[{"span":{"file":0,"start":114,"end":135},"type_name":{"text":"Maybe","span":{"file":0,"start":115,"end":120}},"dot_span":{"file":0,"start":120,"end":121},"variant":{"text":"none","span":{"file":0,"start":121,"end":125}},"binding":null,"arrow_span":{"file":0,"start":131,"end":133},"value":1},{"span":{"file":0,"start":137,"end":167},"type_name":{"text":"Maybe","span":{"file":0,"start":138,"end":143}},"dot_span":{"file":0,"start":143,"end":144},"variant":{"text":"some","span":{"file":0,"start":144,"end":148}},"binding":{"text":"value","span":{"file":0,"start":152,"end":157}},"arrow_span":{"file":0,"start":159,"end":161},"value":2}],"close_brace_span":{"file":0,"start":168,"end":169}}}]}}]}],"diagnostics":[]}}"#;
pub(super) const ARRAY_OOB_SOURCE: &str =
    "function get(xs: FixedArray<i32, 2>): i32 { return xs[2]; }";
pub(super) const ARRAY_VALID_SOURCE: &str =
    "function get(xs: FixedArray<i32, 2>): i32 { return xs[1]; }";
pub(super) const ARRAY_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":38,"end":41},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":38,"end":41}}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":59},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"get","span":{"file":0,"start":9,"end":12}},"parameters":[{"span":{"file":0,"start":13,"end":35},"name":{"text":"xs","span":{"file":0,"start":13,"end":15}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":42,"end":59},"root_block":0,"blocks":[{"span":{"file":0,"start":42,"end":59},"open_brace_span":{"file":0,"start":42,"end":43},"statements":[0],"close_brace_span":{"file":0,"start":58,"end":59}}],"statements":[{"span":{"file":0,"start":44,"end":57},"kind":{"kind":"return","keyword_span":{"file":0,"start":44,"end":50},"value":2,"semicolon_span":{"file":0,"start":56,"end":57}}}],"expressions":[{"span":{"file":0,"start":51,"end":53},"kind":{"kind":"reference","name":{"text":"xs","span":{"file":0,"start":51,"end":53}}}},{"span":{"file":0,"start":54,"end":55},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":51,"end":56},"kind":{"kind":"index","base":0,"open_bracket_span":{"file":0,"start":53,"end":54},"index":1,"close_bracket_span":{"file":0,"start":55,"end":56}}}]}}]}],"diagnostics":[]}}"#;

pub(super) fn owned_pair_projected_return_snapshot(
    field: &str,
) -> (String, RawProjectSyntaxSnapshot) {
    let replacement = format!("OwnedPair({{ flag: p.flag, first: p.{field} }})");
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    let start = source.rfind("p;").expect("Pair return value");
    source.replace_range(start..=start, &replacement);
    let start = u32::try_from(start).expect("Pair return offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        start + 1,
        u32::try_from(replacement.len() - 1).expect("Pair replacement length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    body.expressions[3] = RawExpressionSyntax {
        span: s(start + 18, start + 19),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start + 18, start + 19) },
        },
    };
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 18, start + 24),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 3,
            dot_span: s(start + 19, start + 20),
            field: RawIdentifierSyntax { text: "flag".to_owned(), span: s(start + 20, start + 24) },
        },
    });
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 33, start + 34),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start + 33, start + 34) },
        },
    });
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 33, start + 34 + u32::try_from(field.len()).expect("field length") + 1),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 5,
            dot_span: s(start + 34, start + 35),
            field: RawIdentifierSyntax {
                text: field.to_owned(),
                span: s(start + 35, start + 35 + u32::try_from(field.len()).expect("field length")),
            },
        },
    });
    let end = start + u32::try_from(replacement.len()).expect("Pair replacement end");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(start, start + 9),
            },
            open_paren_span: s(start + 9, start + 10),
            open_brace_span: s(start + 10, start + 11),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(start + 12, start + 24),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: s(start + 12, start + 16),
                        },
                        colon_span: s(start + 16, start + 17),
                        value: 4,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(start + 26, end - 3),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "first".to_owned(),
                            span: s(start + 26, start + 31),
                        },
                        colon_span: s(start + 31, start + 32),
                        value: 6,
                    },
                },
            ],
            close_brace_span: s(end - 2, end - 1),
            close_paren_span: s(end - 1, end),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("Pair return")
    };
    *value = 7;
    (source, raw)
}
pub(super) fn struct_index_wrong_base_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_pair_projected_return_snapshot("first");
    let start = source.rfind("p.first").expect("Struct wrong-base projection");
    source.replace_range(start..start + 7, "p[0]");
    let start = u32::try_from(start).expect("Struct projection offset");
    let mut raw = shift_snapshot_signed(raw, start + 7, -3);
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    body.expressions[6] = RawExpressionSyntax {
        span: s(start + 2, start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    body.expressions.insert(
        7,
        RawExpressionSyntax {
            span: s(start, start + 4),
            kind: zryna_syntax::v4::RawExpressionKind::Index {
                base: 5,
                open_bracket_span: s(start + 1, start + 2),
                index: 6,
                close_bracket_span: s(start + 3, start + 4),
            },
        },
    );
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut body.expressions[8].kind
    else {
        panic!("Struct wrong-base result")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } = &mut fields[1].kind
    else {
        panic!("Struct wrong-base initializer")
    };
    *value = 7;
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("Struct wrong-base return")
    };
    *value = 8;
    (source, raw)
}
pub(super) fn fixed_array_field_wrong_base_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) =
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Disjoint);
    let start = source.rfind("a[0]").expect("array wrong-base projection");
    source.replace_range(start..start + 4, "a.foo");
    let start = u32::try_from(start).expect("array projection offset");
    let mut raw = shift_snapshot(raw, start + 4, 1);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions.remove(4);
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    body.expressions[4] = RawExpressionSyntax {
        span: s(start, start + 5),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 3,
            dot_span: s(start + 1, start + 2),
            field: RawIdentifierSyntax { text: "foo".to_owned(), span: s(start + 2, start + 5) },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::Index { base, index, .. } =
        &mut body.expressions[7].kind
    else {
        panic!("second array projection")
    };
    *base = 5;
    *index = 6;
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut body.expressions[8].kind
    else {
        panic!("array wrong-base result")
    };
    *elements = vec![4, 7];
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array wrong-base return")
    };
    *value = 8;
    (source, raw)
}
