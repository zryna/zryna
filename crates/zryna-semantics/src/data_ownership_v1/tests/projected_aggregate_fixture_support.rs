pub(super) use super::projected_aggregate_fixture_data::*;
use super::*;

pub(super) fn projected_inner_child_after_parent_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let mut source = PROJECTED_INNER_MOVE_SOURCE.to_owned();
    let start = source.find("\"c\"").expect("return tail literal");
    source.replace_range(start..start + 3, "o.inner.text");
    let start = u32::try_from(start).expect("tail projection offset");
    let mut raw = shift_snapshot(response_snapshot(PROJECTED_INNER_MOVE_RESPONSE), start, 9);
    let body = &mut raw.files[0].functions[0].body;
    let moved_reference = body.expressions[7].clone();
    let mut outer = body.expressions[8].clone();
    let s = |from, to| zryna_source::UntrustedSpan { file: 0, start: from, end: to };
    body.expressions[6] = RawExpressionSyntax {
        span: s(start, start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "o".to_owned(), span: s(start, start + 1) },
        },
    };
    body.expressions[7] = RawExpressionSyntax {
        span: s(start, start + 7),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 6,
            dot_span: s(start + 1, start + 2),
            field: RawIdentifierSyntax { text: "inner".to_owned(), span: s(start + 2, start + 7) },
        },
    };
    body.expressions[8] = RawExpressionSyntax {
        span: s(start, start + 12),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 7,
            dot_span: s(start + 7, start + 8),
            field: RawIdentifierSyntax { text: "text".to_owned(), span: s(start + 8, start + 12) },
        },
    };
    let moved = u32::try_from(body.expressions.len()).expect("shifted moved reference id");
    body.expressions.push(moved_reference);
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } = &mut outer.kind
    else {
        panic!("shifted Outer construction")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value: tail, .. } =
        &mut fields[0].kind
    else {
        panic!("explicit tail field")
    };
    *tail = 8;
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value: inner, .. } =
        &mut fields[1].kind
    else {
        panic!("explicit inner field")
    };
    *inner = moved;
    let result = u32::try_from(body.expressions.len()).expect("shifted Outer result id");
    body.expressions.push(outer);
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("shifted return")
    };
    *value = result;
    (source, raw)
}

pub(super) fn clone_final_return_snapshot(
    source: &str,
    response: &str,
) -> (String, RawProjectSyntaxSnapshot) {
    let raw = response_snapshot(response);
    let body = &raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: reference_value, .. } =
        body.statements.last().expect("return").kind
    else {
        panic!("return")
    };
    let reference = body.expressions[reference_value as usize].clone();
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &reference.kind else {
        panic!("final reference")
    };
    let start = reference.span.start;
    let end = reference.span.end;
    let mut updated_source = source.to_owned();
    updated_source.replace_range(
        usize::try_from(start).expect("start")..usize::try_from(end).expect("end"),
        &format!("clone({})", name.text),
    );
    let raw = shift_snapshot(raw, start, 6);
    let mut raw = shift_snapshot(raw, end + 6, 1);
    let body = &mut raw.files[0].functions[0].body;
    let reference = &mut body.expressions[reference_value as usize];
    reference.span.end -= 1;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut reference.kind else {
        panic!("shifted final reference")
    };
    name.span.end -= 1;
    let new_value = u32::try_from(body.expressions.len()).expect("expression id");
    let RawStatementKind::Return { value, .. } =
        &mut body.statements.last_mut().expect("return").kind
    else {
        panic!("return")
    };
    *value = new_value;
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: reference_value,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    (updated_source, raw)
}

fn exclude_inserted_close(value: &mut serde_json::Value, close: u32) {
    match value {
        serde_json::Value::Object(object)
            if object.contains_key("file")
                && object.contains_key("start")
                && object.contains_key("end") =>
        {
            let from = object["start"].as_u64().expect("span start");
            let to = object["end"].as_u64().expect("span end");
            if from < u64::from(close) && to == u64::from(close + 1) {
                object.insert("end".to_owned(), serde_json::Value::from(close));
            }
        }
        serde_json::Value::Object(object) => {
            for child in object.values_mut() {
                exclude_inserted_close(child, close);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                exclude_inserted_close(child, close);
            }
        }
        _ => {}
    }
}

pub(super) fn projected_aggregate_clone_local_snapshot(
    source: &str,
    response: &str,
) -> (String, RawProjectSyntaxSnapshot) {
    let raw = response_snapshot(response);
    let body = &raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { initializer: operand_id, .. } =
        body.statements[1].kind
    else {
        panic!("projected aggregate local")
    };
    let operand = body.expressions[operand_id as usize].clone();
    let start = operand.span.start;
    let end = operand.span.end;
    let mut updated_source = source.to_owned();
    let spelling = updated_source
        .get(
            usize::try_from(start).expect("operand start")
                ..usize::try_from(end).expect("operand end"),
        )
        .expect("projected operand")
        .to_owned();
    updated_source.replace_range(
        usize::try_from(start).expect("operand start")..usize::try_from(end).expect("operand end"),
        &format!("clone({spelling})"),
    );
    let raw = shift_snapshot(raw, start, 6);
    let mut raw = shift_snapshot(raw, end + 6, 1);
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    exclude_inserted_close(&mut value, end + 6);
    raw = serde_json::from_value(value).expect("clone operand snapshot");
    let body = &mut raw.files[0].functions[0].body;
    let clone = u32::try_from(body.expressions.len()).expect("clone expression id");
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("projected aggregate local")
    };
    *initializer = clone;
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: operand_id,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    (updated_source, raw)
}

pub(super) fn projected_aggregate_clone_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot)
{
    let mut raw = response_snapshot(PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE);
    let RawStatementKind::Assignment { value: operand_id, .. } =
        raw.files[0].functions[0].body.statements[2].kind
    else {
        panic!("projected aggregate assignment")
    };
    let operand = raw.files[0].functions[0].body.expressions[operand_id as usize].clone();
    let start = operand.span.start;
    let end = operand.span.end;
    let mut source = PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE.to_owned();
    let spelling = source
        .get(
            usize::try_from(start).expect("operand start")
                ..usize::try_from(end).expect("operand end"),
        )
        .expect("assignment operand")
        .to_owned();
    source.replace_range(
        usize::try_from(start).expect("operand start")..usize::try_from(end).expect("operand end"),
        &format!("clone({spelling})"),
    );
    raw = shift_snapshot(raw, start, 6);
    raw = shift_snapshot(raw, end + 6, 1);
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    exclude_inserted_close(&mut value, end + 6);
    raw = serde_json::from_value(value).expect("clone assignment snapshot");
    let body = &mut raw.files[0].functions[0].body;
    let clone = u32::try_from(body.expressions.len()).expect("clone expression id");
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[2].kind else {
        panic!("projected aggregate assignment")
    };
    *value = clone;
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: operand_id,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    (source, raw)
}

pub(super) fn two_projected_aggregate_clone_sites_snapshot(
    local_before_assignment: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    const INSERT: &str = "const copy: Inner = clone(src.inner); ";
    let mut raw = response_snapshot(PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE);
    let insertion_statement = if local_before_assignment { 2 } else { 3 };
    let insertion = raw.files[0].functions[0].body.statements[insertion_statement].span.start;
    let mut source = PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.to_owned();
    source.insert_str(usize::try_from(insertion).expect("insertion offset"), INSERT);
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(INSERT.len()).expect("inserted declaration length"),
    );
    let s = |start: u32, end: u32| zryna_source::UntrustedSpan {
        file: 0,
        start: insertion + start,
        end: insertion + end,
    };
    let inner_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("projected clone local type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(12, 17),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Inner".to_owned(), span: s(12, 17) },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let source_reference = u32::try_from(body.expressions.len()).expect("source reference");
    body.expressions.push(RawExpressionSyntax {
        span: s(26, 29),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "src".to_owned(), span: s(26, 29) },
        },
    });
    let source_projection = u32::try_from(body.expressions.len()).expect("source projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(26, 35),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: source_reference,
            dot_span: s(29, 30),
            field: RawIdentifierSyntax { text: "inner".to_owned(), span: s(30, 35) },
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("projected clone");
    body.expressions.push(RawExpressionSyntax {
        span: s(20, 36),
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: s(20, 25),
            open_paren_span: s(25, 26),
            value: source_projection,
            close_paren_span: s(35, 36),
        },
    });
    body.statements.insert(
        insertion_statement,
        RawStatementSyntax {
            span: s(0, 37),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(0, 5),
                mutable: false,
                name: RawIdentifierSyntax { text: "copy".to_owned(), span: s(6, 10) },
                type_syntax: inner_type,
                equals_span: s(18, 19),
                initializer: cloned,
                semicolon_span: s(36, 37),
            },
        },
    );
    body.blocks[0].statements = (0..body.statements.len())
        .map(|index| u32::try_from(index).expect("statement id"))
        .collect();
    (source, raw)
}

pub(super) fn projected_aggregate_clone_direct_return_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let mut raw = response_snapshot(PROJECTED_INNER_DIRECT_RETURN_RESPONSE);
    let RawStatementKind::Return { value: operand_id, .. } =
        raw.files[0].functions[0].body.statements[1].kind
    else {
        panic!("projected aggregate return")
    };
    let operand = raw.files[0].functions[0].body.expressions[operand_id as usize].clone();
    let start = operand.span.start;
    let end = operand.span.end;
    let mut source = PROJECTED_INNER_DIRECT_RETURN_SOURCE.to_owned();
    let spelling = source
        .get(
            usize::try_from(start).expect("return start")
                ..usize::try_from(end).expect("return end"),
        )
        .expect("projected return operand")
        .to_owned();
    source.replace_range(
        usize::try_from(start).expect("return start")..usize::try_from(end).expect("return end"),
        &format!("clone({spelling})"),
    );
    raw = shift_snapshot(raw, start, 6);
    raw = shift_snapshot(raw, end + 6, 1);
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    exclude_inserted_close(&mut value, end + 6);
    raw = serde_json::from_value(value).expect("direct clone snapshot");
    let body = &mut raw.files[0].functions[0].body;
    let clone = u32::try_from(body.expressions.len()).expect("clone expression id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: end + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 5 },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 5,
                end: start + 6,
            },
            value: operand_id,
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: end + 6, end: end + 7 },
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("projected aggregate return")
    };
    *value = clone;
    (source, raw)
}

pub(super) fn projected_aggregate_direct_return_with_parameter_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let mut source = PROJECTED_INNER_DIRECT_RETURN_SOURCE.to_owned();
    let insertion = u32::try_from(source.find("()").expect("empty parameter list") + 1)
        .expect("parameter insertion");
    source.insert_str(usize::try_from(insertion).expect("parameter insertion"), "flag: i32");
    let mut raw =
        shift_snapshot(response_snapshot(PROJECTED_INNER_DIRECT_RETURN_RESPONSE), insertion, 9);
    let file = &mut raw.files[0];
    file.type_syntax.insert(
        3,
        RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: insertion + 6, end: insertion + 9 },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 6,
                        end: insertion + 9,
                    },
                },
            },
        },
    );
    let function = &mut file.functions[0];
    function.result_type += 1;
    function.parameters.push(RawParameterSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 9 },
        name: RawIdentifierSyntax {
            text: "flag".to_owned(),
            span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 4 },
        },
        type_syntax: 3,
    });
    let RawStatementKind::LocalDeclaration { type_syntax, .. } =
        &mut function.body.statements[0].kind
    else {
        panic!("outer local")
    };
    *type_syntax += 1;
    (source, raw)
}
