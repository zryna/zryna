use super::*;

#[derive(Clone, Copy)]
pub(super) enum OwnedPairProjectedStringAssignmentRhs {
    Fresh,
    TargetMove,
}

pub(super) fn owned_pair_projected_string_assignment_snapshot(
    rhs: OwnedPairProjectedStringAssignmentRhs,
    mutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let assignment = match rhs {
        OwnedPairProjectedStringAssignmentRhs::Fresh => "p.first = \"b\"; ",
        OwnedPairProjectedStringAssignmentRhs::TargetMove => "p.first = p.first; ",
    };
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    if mutable {
        source.replace_range(100..105, "let  ");
    }
    let insertion = source.find("return p;").expect("projected assignment insertion");
    source.insert_str(insertion, assignment);
    let insertion = u32::try_from(insertion).expect("projected assignment offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        insertion,
        u32::try_from(assignment.len()).expect("projected assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable: is_mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned Pair local")
    };
    if mutable {
        keyword_span.end = keyword_span.start + 3;
        *is_mutable = true;
    }
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let target_base = u32::try_from(body.expressions.len()).expect("target base");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(insertion, insertion + 1) },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("target projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 7),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: target_base,
            dot_span: s(insertion + 1, insertion + 2),
            field: RawIdentifierSyntax {
                text: "first".to_owned(),
                span: s(insertion + 2, insertion + 7),
            },
        },
    });
    let value = match rhs {
        OwnedPairProjectedStringAssignmentRhs::Fresh => {
            let value = u32::try_from(body.expressions.len()).expect("fresh String value");
            body.expressions.push(RawExpressionSyntax {
                span: s(insertion + 10, insertion + 13),
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"b\"".to_owned(),
                },
            });
            value
        }
        OwnedPairProjectedStringAssignmentRhs::TargetMove => {
            let base = u32::try_from(body.expressions.len()).expect("RHS projection base");
            body.expressions.push(RawExpressionSyntax {
                span: s(insertion + 10, insertion + 11),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "p".to_owned(),
                        span: s(insertion + 10, insertion + 11),
                    },
                },
            });
            let value = u32::try_from(body.expressions.len()).expect("RHS projection");
            body.expressions.push(RawExpressionSyntax {
                span: s(insertion + 10, insertion + 17),
                kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                    base,
                    dot_span: s(insertion + 11, insertion + 12),
                    field: RawIdentifierSyntax {
                        text: "first".to_owned(),
                        span: s(insertion + 12, insertion + 17),
                    },
                },
            });
            value
        }
    };
    let statement_end = insertion + u32::try_from(assignment.trim_end().len()).expect("statement");
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, statement_end),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 8, insertion + 9),
                value,
                semicolon_span: s(statement_end - 1, statement_end),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

pub(super) fn owned_pair_projected_string_clone_assignment_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_projected_string_assignment_snapshot(
        OwnedPairProjectedStringAssignmentRhs::Fresh,
        true,
    );
    let start = source.find("\"b\"").expect("projected clone operand");
    let replacement = "clone(p.first)";
    source.replace_range(start..start + 3, replacement);
    let start = u32::try_from(start).expect("projected clone offset");
    raw = shift_snapshot(raw, start + 3, 11);
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let RawStatementKind::Assignment { value, .. } = body.statements[1].kind else {
        panic!("projected clone assignment")
    };
    body.expressions[value as usize] = RawExpressionSyntax {
        span: s(start + 6, start + 7),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start + 6, start + 7) },
        },
    };
    let projection = u32::try_from(body.expressions.len()).expect("clone projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 6, start + 13),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: value,
            dot_span: s(start + 7, start + 8),
            field: RawIdentifierSyntax { text: "first".to_owned(), span: s(start + 8, start + 13) },
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("projected clone");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, start + 14),
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: s(start, start + 5),
            open_paren_span: s(start + 5, start + 6),
            value: projection,
            close_paren_span: s(start + 13, start + 14),
        },
    });
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[1].kind else {
        panic!("projected clone assignment")
    };
    *value = cloned;
    (source, raw)
}

pub(super) fn owned_pair_copy_projection_clone_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_pair_projected_string_clone_assignment_snapshot();
    let clone_start = source.find("clone(p.first)").expect("copy clone expression");
    let field_start = clone_start + 8;
    let field_end = field_start + 5;
    source.replace_range(field_start..field_end, "flag");
    let mut raw = shift_snapshot_signed(raw, u32::try_from(field_end).expect("copy field end"), -1);
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { value: cloned, .. } = body.statements[1].kind else {
        panic!("copy clone assignment")
    };
    let zryna_syntax::v4::RawExpressionKind::Clone { value: projection, .. } =
        body.expressions[cloned as usize].kind
    else {
        panic!("copy clone expression")
    };
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { field, .. } =
        &mut body.expressions[projection as usize].kind
    else {
        panic!("copy clone projection")
    };
    field.text = "flag".to_owned();
    (source, raw)
}

pub(super) fn owned_pair_copy_projection_assignment_target_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_projected_string_assignment_snapshot(
        OwnedPairProjectedStringAssignmentRhs::Fresh,
        true,
    );
    let start = source.find("p.first =").expect("projected assignment target");
    let replacement = "p.flag";
    source.replace_range(start..start + 7, replacement);
    let start = u32::try_from(start).expect("invalid target offset");
    raw = shift_snapshot_signed(
        raw,
        start + 7,
        i32::try_from(replacement.len()).expect("small target") - 7,
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { target, .. } = body.statements[1].kind else {
        panic!("projected assignment")
    };
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { base, .. } =
        body.expressions[target as usize].kind
    else {
        panic!("projected target")
    };
    body.expressions[target as usize] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 6 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: start + 1, end: start + 2 },
            field: RawIdentifierSyntax {
                text: "flag".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: start + 2, end: start + 6 },
            },
        },
    };
    (source, raw)
}

pub(super) fn owned_array_projected_string_assignment_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "a[0] = \"c\"; ";
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    let mutable = source.find("const a").expect("owned array local");
    source.replace_range(mutable..mutable + 5, "let  ");
    let insertion = source.find("return a;").expect("array assignment insertion");
    source.insert_str(insertion, ASSIGNMENT);
    let insertion = u32::try_from(insertion).expect("array assignment offset");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        insertion,
        u32::try_from(ASSIGNMENT.len()).expect("array assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned array local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let base = u32::try_from(body.expressions.len()).expect("array target base");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(insertion, insertion + 1) },
        },
    });
    let index = u32::try_from(body.expressions.len()).expect("array target index");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 2, insertion + 3),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let target = u32::try_from(body.expressions.len()).expect("array target projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 4),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base,
            open_bracket_span: s(insertion + 1, insertion + 2),
            index,
            close_bracket_span: s(insertion + 3, insertion + 4),
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("array replacement String");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 7, insertion + 10),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"c\"".to_owned() },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: s(insertion, insertion + 11),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 5, insertion + 6),
                value,
                semicolon_span: s(insertion + 10, insertion + 11),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

pub(super) fn owned_array_projected_string_clone_assignment_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_array_projected_string_assignment_snapshot();
    let start = source.find("\"c\"").expect("array clone operand");
    let replacement = "clone(a[0])";
    source.replace_range(start..start + 3, replacement);
    let start = u32::try_from(start).expect("array clone offset");
    raw = shift_snapshot(raw, start + 3, 8);
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let RawStatementKind::Assignment { value, .. } = body.statements[1].kind else {
        panic!("array clone assignment")
    };
    body.expressions[value as usize] = RawExpressionSyntax {
        span: s(start + 6, start + 7),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "a".to_owned(), span: s(start + 6, start + 7) },
        },
    };
    let index = u32::try_from(body.expressions.len()).expect("clone array index");
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 8, start + 9),
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let projection = u32::try_from(body.expressions.len()).expect("clone array projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(start + 6, start + 10),
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: value,
            open_bracket_span: s(start + 7, start + 8),
            index,
            close_bracket_span: s(start + 9, start + 10),
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("projected array clone");
    body.expressions.push(RawExpressionSyntax {
        span: s(start, start + 11),
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: s(start, start + 5),
            open_paren_span: s(start + 5, start + 6),
            value: projection,
            close_paren_span: s(start + 10, start + 11),
        },
    });
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[1].kind else {
        panic!("array clone assignment")
    };
    *value = cloned;
    (source, raw)
}
