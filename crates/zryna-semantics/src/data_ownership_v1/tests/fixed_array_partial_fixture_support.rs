use super::*;

#[derive(Clone, Copy)]
pub(super) enum OwnedArrayProjectionCase {
    Disjoint,
    Repeat,
    Dynamic,
    Negative,
    OutOfBounds,
}
#[allow(clippy::too_many_lines)]
pub(super) fn owned_array_projected_return_snapshot(
    case: OwnedArrayProjectionCase,
) -> (String, RawProjectSyntaxSnapshot) {
    let indexes = match case {
        OwnedArrayProjectionCase::Disjoint => ("0", "1"),
        OwnedArrayProjectionCase::Repeat => ("0", "0"),
        OwnedArrayProjectionCase::Dynamic => ("a", "1"),
        OwnedArrayProjectionCase::Negative => ("-1", "1"),
        OwnedArrayProjectionCase::OutOfBounds => ("2", "1"),
    };
    let replacement = format!("FixedArray<String, 2>([a[{}], a[{}]])", indexes.0, indexes.1);
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    let start = source.rfind("a;").expect("array return value");
    source.replace_range(start..=start, &replacement);
    let start = u32::try_from(start).expect("array return offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        start + 1,
        u32::try_from(replacement.len() - 1).expect("array replacement length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(start + 11, start + 17),
        kind: RawTypeSyntaxKind::String { keyword_span: s(start + 11, start + 17) },
    });
    let array_type = u32::try_from(raw.files[0].type_syntax.len()).expect("array type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(start, start + 21),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(start, start + 10),
            less_than_span: s(start + 10, start + 11),
            element: string_type,
            comma_span: s(start + 17, start + 18),
            length_span: s(start + 19, start + 20),
            length_spelling: "2".to_owned(),
            length: 2,
            greater_than_span: s(start + 20, start + 21),
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let first_base_start = start + 23;
    body.expressions[3] = RawExpressionSyntax {
        span: s(first_base_start, first_base_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(first_base_start, first_base_start + 1),
            },
        },
    };
    let first_index_start = first_base_start + 2;
    let first_index = match case {
        OwnedArrayProjectionCase::Dynamic => {
            let id = u32::try_from(body.expressions.len()).expect("dynamic index id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start, first_index_start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "a".to_owned(),
                        span: s(first_index_start, first_index_start + 1),
                    },
                },
            });
            id
        }
        OwnedArrayProjectionCase::Negative => {
            let literal = u32::try_from(body.expressions.len()).expect("negative literal id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start + 1, first_index_start + 2),
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() },
            });
            let id = u32::try_from(body.expressions.len()).expect("negative index id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start, first_index_start + 2),
                kind: zryna_syntax::v4::RawExpressionKind::Negation {
                    operator_span: s(first_index_start, first_index_start + 1),
                    operand: literal,
                },
            });
            id
        }
        _ => {
            let id = u32::try_from(body.expressions.len()).expect("constant index id");
            body.expressions.push(RawExpressionSyntax {
                span: s(first_index_start, first_index_start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal {
                    spelling: indexes.0.to_owned(),
                },
            });
            id
        }
    };
    let first_index_len = u32::try_from(indexes.0.len()).expect("first index length");
    let first_projection = u32::try_from(body.expressions.len()).expect("first projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(first_base_start, first_index_start + first_index_len + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: 3,
            open_bracket_span: s(first_base_start + 1, first_base_start + 2),
            index: first_index,
            close_bracket_span: s(
                first_index_start + first_index_len,
                first_index_start + first_index_len + 1,
            ),
        },
    });
    let second_base_start = first_index_start + first_index_len + 3;
    let second_base = u32::try_from(body.expressions.len()).expect("second base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(second_base_start, second_base_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(second_base_start, second_base_start + 1),
            },
        },
    });
    let second_index = u32::try_from(body.expressions.len()).expect("second index id");
    body.expressions.push(RawExpressionSyntax {
        span: s(second_base_start + 2, second_base_start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: indexes.1.to_owned() },
    });
    let second_projection = u32::try_from(body.expressions.len()).expect("second projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(second_base_start, second_base_start + 4),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: second_base,
            open_bracket_span: s(second_base_start + 1, second_base_start + 2),
            index: second_index,
            close_bracket_span: s(second_base_start + 3, second_base_start + 4),
        },
    });
    let end = start + u32::try_from(replacement.len()).expect("array replacement end");
    let result = u32::try_from(body.expressions.len()).expect("array result id");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, end),
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: array_type,
            open_paren_span: s(start + 21, start + 22),
            open_bracket_span: s(start + 22, start + 23),
            elements: vec![first_projection, second_projection],
            close_bracket_span: s(end - 2, end - 1),
            close_paren_span: s(end - 1, end),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array return")
    };
    *value = result;
    (source, raw)
}
pub(super) fn owned_array_projected_clone_return_snapshot(
    case: OwnedArrayProjectionCase,
    ordinal: usize,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_array_projected_return_snapshot(case);
    let body = &raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: result, .. } = body.statements[1].kind else {
        panic!("array clone return")
    };
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &body.expressions[result as usize].kind
    else {
        panic!("array clone construction")
    };
    let projection = elements[ordinal];
    let projection_span = body.expressions[projection as usize].span;
    let start = projection_span.start;
    let end = projection_span.end;
    source.insert_str(usize::try_from(start).expect("clone start"), "clone(");
    source.insert(usize::try_from(end + 6).expect("clone end"), ')');
    let raw = shift_snapshot(raw, start, 6);
    let mut raw = shift_snapshot(raw, end + 6, 1);
    let body = &mut raw.files[0].functions[0].body;
    let projected = &mut body.expressions[projection as usize];
    projected.span.end -= 1;
    let zryna_syntax::v4::RawExpressionKind::Index { close_bracket_span, .. } = &mut projected.kind
    else {
        panic!("array clone projection")
    };
    close_bracket_span.end -= 1;
    assert_eq!(result as usize + 1, body.expressions.len());
    let mut construction = body.expressions.pop().expect("array clone construction");
    let cloned = u32::try_from(body.expressions.len()).expect("array clone expression");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: projection,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
        open_bracket_span,
        elements,
        ..
    } = &mut construction.kind
    else {
        panic!("array clone construction")
    };
    if ordinal == 0 {
        open_bracket_span.end -= 6;
    }
    elements[ordinal] = cloned;
    let rebuilt = u32::try_from(body.expressions.len()).expect("rebuilt array construction");
    body.expressions.push(construction);
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array clone return")
    };
    *value = rebuilt;
    (source, raw)
}
#[allow(clippy::too_many_lines)]
pub(super) fn owned_array_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCALS: &str = "const text: String = a[0]; const b: FixedArray<String, 2> = a; ";
    let (mut source, mut raw) =
        owned_array_projected_clone_return_snapshot(OwnedArrayProjectionCase::Disjoint, 1);
    let insertion = source.find("return FixedArray").expect("array transfer insertion");
    source.insert_str(insertion, LOCALS);
    let insertion = u32::try_from(insertion).expect("array transfer offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(LOCALS.len()).expect("array transfer locals length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 12, insertion + 18),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 12, insertion + 18) },
    });
    let element_type = u32::try_from(raw.files[0].type_syntax.len()).expect("element type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 47, insertion + 53),
        kind: RawTypeSyntaxKind::String { keyword_span: s(insertion + 47, insertion + 53) },
    });
    let array_type = u32::try_from(raw.files[0].type_syntax.len()).expect("array type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 36, insertion + 57),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: s(insertion + 36, insertion + 46),
            less_than_span: s(insertion + 46, insertion + 47),
            element: element_type,
            comma_span: s(insertion + 53, insertion + 54),
            length_span: s(insertion + 55, insertion + 56),
            length_spelling: "2".to_owned(),
            length: 2,
            greater_than_span: s(insertion + 56, insertion + 57),
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: result, .. } = body.statements[1].kind else {
        panic!("array transfer return")
    };
    assert_eq!(result as usize + 1, body.expressions.len());
    let mut construction = body.expressions.pop().expect("array transfer construction");
    let (first_result, second_clone) = match &construction.kind {
        zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } => {
            (elements[0], elements[1])
        }
        _ => panic!("array transfer result"),
    };
    let zryna_syntax::v4::RawExpressionKind::Index { base: old_base, index: old_index, .. } =
        body.expressions[first_result as usize].kind
    else {
        panic!("first array result projection")
    };
    let first_result_span = body.expressions[first_result as usize].span;
    body.expressions[old_base as usize] = RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    };
    body.expressions[old_index as usize] = RawExpressionSyntax {
        span: s(insertion + 23, insertion + 24),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    body.expressions[first_result as usize] = RawExpressionSyntax {
        span: s(insertion + 21, insertion + 25),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: old_base,
            open_bracket_span: s(insertion + 22, insertion + 23),
            index: old_index,
            close_bracket_span: s(insertion + 24, insertion + 25),
        },
    };
    let first_start = usize::try_from(first_result_span.start).expect("first result offset");
    source.replace_range(first_start..first_start + 4, "text");
    let return_text = u32::try_from(body.expressions.len()).expect("return text id");
    body.expressions.push(RawExpressionSyntax {
        span: first_result_span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "text".to_owned(), span: first_result_span },
        },
    });
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut construction.kind
    else {
        unreachable!("array transfer result already matched")
    };
    elements[0] = return_text;
    let transfer_source = u32::try_from(body.expressions.len()).expect("transfer source id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 60, insertion + 61),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 60, insertion + 61),
            },
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
                initializer: first_result,
                semicolon_span: s(insertion + 25, insertion + 26),
            },
        },
    );
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(insertion + 27, insertion + 62),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion + 27, insertion + 32),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "b".to_owned(),
                    span: s(insertion + 33, insertion + 34),
                },
                type_syntax: array_type,
                equals_span: s(insertion + 58, insertion + 59),
                initializer: transfer_source,
                semicolon_span: s(insertion + 61, insertion + 62),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3];
    let zryna_syntax::v4::RawExpressionKind::Clone { value: cloned, .. } =
        body.expressions[second_clone as usize].kind
    else {
        panic!("second array result clone")
    };
    let zryna_syntax::v4::RawExpressionKind::Index { base: second_base, .. } =
        body.expressions[cloned as usize].kind
    else {
        panic!("second array result projection")
    };
    let second_span = body.expressions[second_base as usize].span;
    let second_start = usize::try_from(second_span.start).expect("second result offset");
    source.replace_range(second_start..=second_start, "b");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut body.expressions[second_base as usize].kind
    else {
        panic!("second array result base")
    };
    name.text = "b".to_owned();
    let rebuilt = u32::try_from(body.expressions.len()).expect("rebuilt array transfer result");
    body.expressions.push(construction);
    let RawStatementKind::Return { value, .. } = &mut body.statements[3].kind else {
        panic!("array transfer return")
    };
    *value = rebuilt;
    (source, raw)
}
pub(super) fn fixed_array_oob_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const FINAL_RETURN: &str = "return a; ";
    let (mut source, mut raw) =
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::OutOfBounds);
    let fresh = source.rfind("a[1]").expect("fresh array assignment element");
    source.replace_range(fresh..fresh + 4, "\"b\"");
    let fresh = u32::try_from(fresh).expect("fresh element offset");
    raw = shift_snapshot_signed(raw, fresh + 4, -1);
    {
        let body = &mut raw.files[0].functions[0].body;
        body.expressions.remove(6);
        body.expressions.remove(6);
        body.expressions[6] = RawExpressionSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: fresh, end: fresh + 3 },
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: "\"b\"".to_owned(),
            },
        };
        let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
            &mut body.expressions[7].kind
        else {
            panic!("fresh array assignment result")
        };
        *elements = vec![5, 6];
        let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
            panic!("fresh array assignment return")
        };
        *value = 7;
    }
    source.replace_range(41..46, "let  ");
    let assignment = source.find("return FixedArray").expect("array assignment return");
    source.replace_range(assignment..assignment + 7, "a = ");
    let assignment = u32::try_from(assignment).expect("array assignment offset");
    let mut raw = shift_snapshot_signed(raw, assignment + 7, -3);
    let insertion = source.rfind('}').expect("array function close");
    source.insert_str(insertion, FINAL_RETURN);
    let insertion = u32::try_from(insertion).expect("final return offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(FINAL_RETURN.len()).expect("final return length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("array assignment local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let RawStatementKind::Return { value: replacement, semicolon_span, .. } =
        body.statements[1].kind
    else {
        panic!("array replacement expression")
    };
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let target = u32::try_from(body.expressions.len()).expect("array assignment target");
    body.expressions.push(RawExpressionSyntax {
        span: s(assignment, assignment + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(assignment, assignment + 1) },
        },
    });
    body.statements[1] = RawStatementSyntax {
        span: s(assignment, semicolon_span.end),
        kind: RawStatementKind::Assignment {
            target,
            equals_span: s(assignment + 2, assignment + 3),
            value: replacement,
            semicolon_span,
        },
    };
    let returned = u32::try_from(body.expressions.len()).expect("array final return");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 7, insertion + 8),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: s(insertion + 7, insertion + 8),
            },
        },
    });
    body.statements.push(RawStatementSyntax {
        span: s(insertion, insertion + 9),
        kind: RawStatementKind::Return {
            keyword_span: s(insertion, insertion + 6),
            value: returned,
            semicolon_span: s(insertion + 8, insertion + 9),
        },
    });
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}
