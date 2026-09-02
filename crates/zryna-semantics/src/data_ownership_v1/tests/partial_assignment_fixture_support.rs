use super::*;

pub(super) fn owned_pair_partial_then_root_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const text: String = p.first; ";
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    let insertion = source.find("return p;").expect("Pair return insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("Pair insertion offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        insertion,
        u32::try_from(LOCAL.len()).expect("projected local length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 12, insertion + 18),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 12, insertion + 18) },
    });
    let body = &mut raw.files[0].functions[0].body;
    let base = u32::try_from(body.expressions.len()).expect("projected base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    });
    let projected = u32::try_from(body.expressions.len()).expect("projected value id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 28),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base,
            dot_span: s(insertion + 22, insertion + 23),
            field: RawIdentifierSyntax {
                text: "first".to_owned(),
                span: s(insertion + 23, insertion + 28),
            },
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 29),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "text".to_owned(),
                    span: s(insertion + 6, insertion + 10),
                },
                type_syntax: string_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: projected,
                semicolon_span: s(insertion + 28, insertion + 29),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[allow(clippy::too_many_lines)]
pub(super) fn owned_pair_partial_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const INSERT: &str = "let q: OwnedPair = OwnedPair({ flag: false, first: \"old\" }); q = p; ";
    let (mut source, mut raw) = owned_pair_partial_then_root_snapshot();
    let insertion = source.find("return p;").expect("partial assignment insertion");
    source.insert_str(insertion, INSERT);
    let return_value = source.rfind("p;").expect("partial assignment return value");
    source.replace_range(return_value..=return_value, "q");
    let insertion = u32::try_from(insertion).expect("partial assignment offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(INSERT.len()).expect("partial assignment length"),
    );
    let s = |start: u32, end: u32| zryna_source::UntrustedSpan {
        file: 0,
        start: insertion + start,
        end: insertion + end,
    };
    let pair_type = u32::try_from(raw.files[0].type_syntax.len()).expect("Pair type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(7, 16),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "OwnedPair".to_owned(), span: s(7, 16) },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let flag = u32::try_from(body.expressions.len()).expect("assignment flag value");
    body.expressions.push(RawExpressionSyntax {
        span: s(37, 42),
        kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: false },
    });
    let old = u32::try_from(body.expressions.len()).expect("assignment old String value");
    body.expressions.push(RawExpressionSyntax {
        span: s(51, 56),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"old\"".to_owned() },
    });
    let target_initializer =
        u32::try_from(body.expressions.len()).expect("assignment target initializer");
    body.expressions.push(RawExpressionSyntax {
        span: s(19, 59),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax { text: "OwnedPair".to_owned(), span: s(19, 28) },
            open_paren_span: s(28, 29),
            open_brace_span: s(29, 30),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(31, 42),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax { text: "flag".to_owned(), span: s(31, 35) },
                        colon_span: s(35, 36),
                        value: flag,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(44, 56),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax { text: "first".to_owned(), span: s(44, 49) },
                        colon_span: s(49, 50),
                        value: old,
                    },
                },
            ],
            close_brace_span: s(57, 58),
            close_paren_span: s(58, 59),
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("assignment target value");
    body.expressions.push(RawExpressionSyntax {
        span: s(61, 62),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "q".to_owned(), span: s(61, 62) },
        },
    });
    let partial_source = u32::try_from(body.expressions.len()).expect("partial assignment source");
    body.expressions.push(RawExpressionSyntax {
        span: s(65, 66),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(65, 66) },
        },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(0, 60),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(0, 3),
                mutable: true,
                name: RawIdentifierSyntax { text: "q".to_owned(), span: s(4, 5) },
                type_syntax: pair_type,
                equals_span: s(17, 18),
                initializer: target_initializer,
                semicolon_span: s(59, 60),
            },
        },
    );
    body.statements.insert(
        3,
        RawStatementSyntax {
            span: s(61, 67),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(63, 64),
                value: partial_source,
                semicolon_span: s(66, 67),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3, 4];
    let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
        panic!("partial assignment return")
    };
    let return_value = u32::try_from(return_value).expect("partial assignment return offset");
    body.expressions[value as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_value, end: return_value + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: return_value,
                    end: return_value + 1,
                },
            },
        },
    };
    (source, raw)
}

pub(super) fn owned_pair_partial_assignment_old_source_return_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_partial_assignment_snapshot();
    let start = source.rfind("q;").expect("assigned-root return");
    source.replace_range(start..=start, "p");
    let start = u32::try_from(start).expect("old-source return offset");
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
        panic!("partial assignment return")
    };
    body.expressions[value as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
            },
        },
    };
    (source, raw)
}

pub(super) fn owned_pair_partial_self_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_partial_assignment_snapshot();
    let start = source.find("q = p;").expect("partial assignment target");
    source.replace_range(start..=start, "p");
    let start = u32::try_from(start).expect("partial self-assignment offset");
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { target, .. } = body.statements[3].kind else {
        panic!("partial assignment statement")
    };
    body.expressions[target as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
            },
        },
    };
    (source, raw)
}

pub(super) fn owned_array_partial_then_root_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const text: String = a[0]; ";
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    let insertion = source.find("return a;").expect("array return insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("array insertion offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        insertion,
        u32::try_from(LOCAL.len()).expect("array projected local length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 12, insertion + 18),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 12, insertion + 18) },
    });
    let body = &mut raw.files[0].functions[0].body;
    let base = u32::try_from(body.expressions.len()).expect("array projected base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    });
    let index = u32::try_from(body.expressions.len()).expect("array projected index id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 23, insertion + 24),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let projected = u32::try_from(body.expressions.len()).expect("array projected value id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 25),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base,
            open_bracket_span: s(insertion + 22, insertion + 23),
            index,
            close_bracket_span: s(insertion + 24, insertion + 25),
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 26),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "text".to_owned(),
                    span: s(insertion + 6, insertion + 10),
                },
                type_syntax: string_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: projected,
                semicolon_span: s(insertion + 25, insertion + 26),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[allow(clippy::too_many_lines)]
pub(super) fn owned_array_partial_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const INSERT: &str =
        "let b: FixedArray<String, 2> = FixedArray<String, 2>([\"old0\", \"old1\"]); b = a; ";
    let (mut source, mut raw) = owned_array_partial_then_root_snapshot();
    let insertion = source.find("return a;").expect("partial array assignment insertion");
    source.insert_str(insertion, INSERT);
    let return_value = source.rfind("a;").expect("partial array assignment return");
    source.replace_range(return_value..=return_value, "b");
    let insertion = u32::try_from(insertion).expect("partial array assignment offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(INSERT.len()).expect("partial array assignment length"),
    );
    let s = |start: u32, end: u32| zryna_source::UntrustedSpan {
        file: 0,
        start: insertion + start,
        end: insertion + end,
    };
    let annotation_element =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array annotation element type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(18, 24),
        kind: RawTypeSyntaxKind::String { keyword_span: s(18, 24) },
    });
    let annotation_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array annotation type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(7, 28),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(7, 17),
            less_than_span: s(17, 18),
            element: annotation_element,
            comma_span: s(24, 25),
            length_span: s(26, 27),
            length: 2,
            length_spelling: "2".to_owned(),
            greater_than_span: s(27, 28),
        },
    });
    let constructor_element =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array constructor element type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(42, 48),
        kind: RawTypeSyntaxKind::String { keyword_span: s(42, 48) },
    });
    let constructor_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("array constructor type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(31, 52),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(31, 41),
            less_than_span: s(41, 42),
            element: constructor_element,
            comma_span: s(48, 49),
            length_span: s(50, 51),
            length: 2,
            length_spelling: "2".to_owned(),
            greater_than_span: s(51, 52),
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let old0 = u32::try_from(body.expressions.len()).expect("old array element zero");
    body.expressions.push(RawExpressionSyntax {
        span: s(54, 60),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
            spelling: "\"old0\"".to_owned(),
        },
    });
    let old1 = u32::try_from(body.expressions.len()).expect("old array element one");
    body.expressions.push(RawExpressionSyntax {
        span: s(62, 68),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
            spelling: "\"old1\"".to_owned(),
        },
    });
    let initializer = u32::try_from(body.expressions.len()).expect("old array initializer");
    body.expressions.push(RawExpressionSyntax {
        span: s(31, 70),
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: constructor_type,
            open_paren_span: s(52, 53),
            open_bracket_span: s(53, 54),
            elements: vec![old0, old1],
            close_bracket_span: s(68, 69),
            close_paren_span: s(69, 70),
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("partial array assignment target");
    body.expressions.push(RawExpressionSyntax {
        span: s(72, 73),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "b".to_owned(), span: s(72, 73) },
        },
    });
    let partial_source =
        u32::try_from(body.expressions.len()).expect("partial array assignment source");
    body.expressions.push(RawExpressionSyntax {
        span: s(76, 77),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(76, 77) },
        },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(0, 71),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(0, 3),
                mutable: true,
                name: RawIdentifierSyntax { text: "b".to_owned(), span: s(4, 5) },
                type_syntax: annotation_type,
                equals_span: s(29, 30),
                initializer,
                semicolon_span: s(70, 71),
            },
        },
    );
    body.statements.insert(
        3,
        RawStatementSyntax {
            span: s(72, 78),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(74, 75),
                value: partial_source,
                semicolon_span: s(77, 78),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3, 4];
    let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
        panic!("partial array assignment return")
    };
    let return_value = u32::try_from(return_value).expect("partial array return offset");
    body.expressions[value as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_value, end: return_value + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "b".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: return_value,
                    end: return_value + 1,
                },
            },
        },
    };
    (source, raw)
}
