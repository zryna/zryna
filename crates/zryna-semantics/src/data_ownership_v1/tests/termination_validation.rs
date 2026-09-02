use super::*;

#[test]
fn duplicate_return_is_rejected_at_the_second_return() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.insert_str(189, "return 0; ");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 189, 10);
    let body = &mut raw.files[0].functions[0].body;
    let value = u32::try_from(body.expressions.len()).expect("expression id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 196, end: 197 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 198 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 195 },
            value,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 197, end: 198 },
        },
    });
    body.blocks[0].statements.push(statement);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful duplicate return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("duplicate return");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(diagnostics[0].primary_span().expect("second return").start(), 189);
}

#[test]
fn mutation_after_return_is_rejected_before_lowering_the_mutation() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.insert_str(189, "pair.left = 0; ");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 189, 15);
    let body = &mut raw.files[0].functions[0].body;
    let reference = u32::try_from(body.expressions.len()).expect("reference id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 193 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "pair".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 193 },
            },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("target id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 198 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: reference,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: 193, end: 194 },
            field: RawIdentifierSyntax {
                text: "left".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 194, end: 198 },
            },
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("value id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 201, end: 202 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 203 },
        kind: RawStatementKind::Assignment {
            target,
            equals_span: zryna_source::UntrustedSpan { file: 0, start: 199, end: 200 },
            value,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 202, end: 203 },
        },
    });
    body.blocks[0].statements.push(statement);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful post-return mutation");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mutation after return");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(diagnostics[0].primary_span().expect("mutation span").start(), 189);
}
