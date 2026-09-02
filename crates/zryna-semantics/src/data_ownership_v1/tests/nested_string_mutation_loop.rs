use super::*;

fn private_nested_string_mutation_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) =
        private_string_mutation_loop_fixture_with_options(true, StringLoopReplacement::ConcatRead);
    let old = "concat(outer, \"x\")";
    let replacement = "identity(concat(outer, \"x\"))";
    let start = u32::try_from(source.find(old).expect("loop concat")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        replacement,
    );
    let mut raw = shift_snapshot(raw, old_end, extra);
    let token = |needle, ordinal| nth_untrusted_span(&source, needle, ordinal);
    let body = &mut raw.files[0].functions[0].body;
    let outer = token("outer", 2);
    body.expressions[3] = RawExpressionSyntax {
        span: outer,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "outer".to_owned(), span: outer },
        },
    };
    let literal = token("\"x\"", 0);
    body.expressions[4] = RawExpressionSyntax {
        span: literal,
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"x\"".to_owned() },
    };
    let concat_start = token("concat", 0);
    let concat_close = token(")", 2);
    body.expressions[5] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: concat_start.start,
            end: concat_close.end,
        },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "concat".to_owned(), span: concat_start },
            open_paren_span: token("(", 3),
            arguments: vec![3, 4],
            close_paren_span: concat_close,
        },
    };
    let return_expression = body.expressions.pop().expect("return expression");
    let identity_id = u32::try_from(body.expressions.len()).expect("identity id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 28 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "identity".to_owned(), span: token("identity", 0) },
            open_paren_span: token("(", 2),
            arguments: vec![5],
            close_paren_span: token(")", 3),
        },
    });
    let return_id = u32::try_from(body.expressions.len()).expect("return id");
    body.expressions.push(return_expression);
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[2].kind else {
        panic!("loop assignment")
    };
    *value = identity_id;
    let RawStatementKind::Return { value, .. } = &mut body.statements[3].kind else {
        panic!("loop return")
    };
    *value = return_id;

    let (call_source, call_raw) = private_string_call_fixture();
    let identity = call_raw.files[0].functions[1].clone();
    let identity_text = &call_source[usize::try_from(identity.span.start).expect("start")
        ..usize::try_from(identity.span.end).expect("end")];
    source.push(' ');
    let appended_start = u32::try_from(source.len()).expect("offset");
    source.push_str(identity_text);
    let shift = i32::try_from(appended_start).expect("offset")
        - i32::try_from(identity.span.start).expect("offset");
    let mut identity_snapshot = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![
                call_raw.files[0].type_syntax
                    [usize::try_from(identity.parameters[0].type_syntax).expect("type")]
                .clone(),
                call_raw.files[0].type_syntax[usize::try_from(identity.result_type).expect("type")]
                    .clone(),
            ],
            data_declarations: Vec::new(),
            functions: vec![identity],
        }],
        diagnostics: Vec::new(),
    };
    identity_snapshot.files[0].functions[0].parameters[0].type_syntax = 0;
    identity_snapshot.files[0].functions[0].result_type = 1;
    identity_snapshot = shift_snapshot_signed(identity_snapshot, 0, shift);
    let type_base = u32::try_from(raw.files[0].type_syntax.len()).expect("type base");
    let appended = &mut identity_snapshot.files[0];
    appended.functions[0].parameters[0].type_syntax += type_base;
    appended.functions[0].result_type += type_base;
    raw.files[0].type_syntax.append(&mut appended.type_syntax);
    raw.files[0].functions.append(&mut appended.functions);
    (source, raw)
}

#[test]
fn private_string_mutation_loop_accepts_nested_private_concat_call() {
    let (source, raw) = private_nested_string_mutation_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested loop call");
    let program = lower(pair_input(&syntax, &sources)).expect("nested loop call");
    let body = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .nth(2)
        .expect("loop body");
    assert!(
        body.instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::StringConcat })
    );
    assert!(
        body.instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::DirectCall })
    );
    assert!(
        body.instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::ReplacePlace })
    );
}
