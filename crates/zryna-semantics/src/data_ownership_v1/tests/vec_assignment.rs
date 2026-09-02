use super::*;

#[test]
fn private_vec_string_root_assignment_prepares_then_replaces_exact_owner() {
    let sources = sources_for(VEC_ASSIGN_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_ASSIGN_STRING_RESPONSE), &sources)
        .expect("source-faithful Vec<String> assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String> assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let constructs = block
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::VecConstruct)
        .collect::<Vec<_>>();
    assert_eq!(constructs.len(), 2);
    assert_eq!(
        constructs[1]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [3, 2]
    );
    let replace = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_i32_move_assignment_replaces_old_owner_and_preserves_stack_slot() {
    let sources = sources_for(VEC_ASSIGN_I32_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_ASSIGN_I32_RESPONSE), &sources)
        .expect("source-faithful Vec<i32> move assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<i32> assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let replace = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_assignment_rejects_call_based_target_consumption_before_rhs() {
    let source = VEC_ASSIGN_I32_SOURCE.replacen("x = y", "x = identity(x)", 1);
    let mut raw = shift_snapshot(response_snapshot(VEC_ASSIGN_I32_RESPONSE), 98, 10);
    let body = &mut raw.files[0].functions[0].body;
    body.statements[2].span.end += 10;
    let RawStatementKind::Assignment { value, semicolon_span, .. } = &mut body.statements[2].kind
    else {
        unreachable!("Vec assignment")
    };
    semicolon_span.start += 10;
    semicolon_span.end += 10;
    body.expressions[3] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 104, end: 105 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 104, end: 105 },
            },
        },
    };
    *value = u32::try_from(body.expressions.len()).expect("call id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 95, end: 106 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "identity".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 95, end: 103 },
            },
            open_paren_span: zryna_source::UntrustedSpan { file: 0, start: 103, end: 104 },
            arguments: vec![3],
            close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 105, end: 106 },
        },
    });
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful consuming Vec assignment");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("target move must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "x", 2)))
    );
}
