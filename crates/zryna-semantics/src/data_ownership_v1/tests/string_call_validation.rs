use super::*;

#[test]
fn private_string_call_rejects_exported_owned_callee_at_call_name() {
    let (source, raw) = private_string_call_fixture();
    let identity_start = raw.files[0].functions[1].span.start;
    let mut raw = shift_snapshot(raw, identity_start, 7);
    let identity = &mut raw.files[0].functions[1];
    identity.span.start = identity_start;
    identity.export_span = Some(zryna_source::UntrustedSpan {
        file: 0,
        start: identity_start,
        end: identity_start + 6,
    });
    let call_span = match &raw.files[0].functions[0].body.expressions[2].kind {
        zryna_syntax::v4::RawExpressionKind::Call { callee, .. } => callee.span,
        _ => panic!("identity call"),
    };
    let source = source.replacen("function identity", "export function identity", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported String callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("exported owned call");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3016")
        .expect("private-call diagnostic");
    let at = diagnostic.primary_span().expect("callee span");
    assert_eq!((at.start(), at.end()), (call_span.start, call_span.end));
}

#[test]
fn private_string_call_rejects_wrong_arity_before_argument_transfer() {
    let (source, raw) = private_string_call_fixture();
    let removed = raw.files[0].functions[0].body.expressions[1].span;
    let mut raw = shift_snapshot_signed(raw, removed.end, -10);
    let caller = &mut raw.files[0].functions[0];
    caller.body.expressions.remove(1);
    let RawStatementKind::LocalDeclaration { initializer, .. } =
        &mut caller.body.statements[1].kind
    else {
        panic!("value local")
    };
    *initializer = 1;
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[2].kind else {
        panic!("return")
    };
    *value = 3;
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } =
        &mut caller.body.expressions[1].kind
    else {
        panic!("identity call")
    };
    arguments.clear();
    let zryna_syntax::v4::RawExpressionKind::Clone { value, .. } =
        &mut caller.body.expressions[3].kind
    else {
        panic!("clone")
    };
    *value = 2;
    let source = source.replacen("identity(producer())", "identity()", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong call arity");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong String call arity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3012"));
}

#[test]
fn private_string_call_rejects_wrong_argument_type_and_missing_name() {
    let (source, mut raw) = private_string_call_fixture();
    let argument_span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: argument_span.start,
            end: argument_span.start + 1,
        },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() },
    };
    let source = source.replacen("producer()", "1         ", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong String argument");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong owned argument type");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3012"));

    let (source, mut raw) = private_string_call_fixture();
    let source = source.replacen("producer()", "missingx()", 1);
    let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("producer call")
    };
    callee.text = "missingx".to_owned();
    let call_span = callee.span;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful missing String callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("missing owned callee");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    let at = diagnostics[0].primary_span().expect("callee span");
    assert_eq!((at.start(), at.end()), (call_span.start, call_span.end));
}

#[test]
fn owned_call_cleanup_amplification_has_checked_exact_plus_one_and_overflow_boundaries() {
    let plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let actions = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert!(!owned_call_cleanup_budget_violation(plans - 1, actions - 1, 2, 1));
    assert!(owned_call_cleanup_budget_violation(plans, 0, 1, 1));
    assert!(owned_call_cleanup_budget_violation(0, actions, 2, 1));
    assert!(owned_call_cleanup_budget_violation(0, usize::MAX, 2, 1));
    assert!(owned_call_cleanup_budget_violation(0, 0, 0, 1));
}

#[test]
fn private_string_call_rejects_use_after_argument_transfer_at_the_reference() {
    let (source, raw) = private_string_call_fixture();
    let producer_call_end = raw.files[0].functions[0].body.expressions[1].span.end;
    let mut raw = shift_snapshot_signed(raw, producer_call_end, -2);
    let argument_span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: argument_span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: argument_span },
        },
    };
    let old_reference = raw.files[0].functions[0].body.expressions[3].span;
    let mut raw = shift_snapshot_signed(raw, old_reference.end, 3);
    let returned_name = zryna_source::UntrustedSpan {
        file: 0,
        start: old_reference.start,
        end: old_reference.start + 8,
    };
    raw.files[0].functions[0].body.expressions[3] = RawExpressionSyntax {
        span: returned_name,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: returned_name },
        },
    };
    let source =
        source.replacen("producer()", "survivor", 1).replacen("clone(value)", "clone(survivor)", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful transferred String reuse");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("use after call transfer");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("moved String diagnostic");
    let at = diagnostic.primary_span().expect("reference span");
    assert_eq!(at.end() - at.start(), 8);
}

#[test]
fn private_string_call_rejects_the_same_owned_binding_in_a_second_call() {
    let (source, raw) = private_string_call_fixture();
    let producer_call_end = raw.files[0].functions[0].body.expressions[1].span.end;
    let mut raw = shift_snapshot_signed(raw, producer_call_end, -2);
    let first_use = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: first_use,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: first_use },
        },
    };
    let old_second_use = raw.files[0].functions[0].body.expressions[3].span;
    let mut raw = shift_snapshot_signed(raw, old_second_use.end, 3);
    let second_use = zryna_source::UntrustedSpan {
        file: 0,
        start: old_second_use.start,
        end: old_second_use.start + 8,
    };
    raw.files[0].functions[0].body.expressions[3] = RawExpressionSyntax {
        span: second_use,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: second_use },
        },
    };
    let clone_keyword_end = match &raw.files[0].functions[0].body.expressions[4].kind {
        zryna_syntax::v4::RawExpressionKind::Clone { keyword_span, .. } => keyword_span.end,
        _ => panic!("clone"),
    };
    let mut raw = shift_snapshot(raw, clone_keyword_end, 3);
    let second_use = raw.files[0].functions[0].body.expressions[3].span;
    let replacement = raw.files[0].functions[0].body.expressions[4].clone();
    let zryna_syntax::v4::RawExpressionKind::Clone {
        keyword_span,
        open_paren_span,
        value,
        close_paren_span,
    } = replacement.kind
    else {
        panic!("shifted clone")
    };
    raw.files[0].functions[0].body.expressions[4] = RawExpressionSyntax {
        span: replacement.span,
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax { text: "identity".to_owned(), span: keyword_span },
            open_paren_span,
            arguments: vec![value],
            close_paren_span,
        },
    };
    let source = source.replacen("producer()", "survivor", 1).replacen(
        "clone(value)",
        "identity(survivor)",
        1,
    );
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful duplicate owned call use");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("duplicate owned call use");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("second transfer diagnostic");
    let at = diagnostic.primary_span().expect("second use span");
    assert_eq!((at.start(), at.end()), (second_use.start, second_use.end));
}

#[test]
fn private_string_call_name_is_exact_and_cycles_fail_closed_in_ir() {
    let (source, mut raw) = private_string_call_fixture();
    let source = source.replacen("producer()", "Producer()", 1);
    let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("producer call")
    };
    callee.text = "Producer".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong-case String call");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong-case String call");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");

    let (source, raw) = private_string_call_fixture();
    let literal = raw.files[0].functions[2].body.expressions[0].span;
    let mut raw = shift_snapshot_signed(raw, literal.end, 4);
    let call_span =
        zryna_source::UntrustedSpan { file: 0, start: literal.start, end: literal.end + 4 };
    raw.files[0].functions[2].body.expressions[0] = RawExpressionSyntax {
        span: call_span,
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "producer".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: literal.start,
                    end: literal.start + 8,
                },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: literal.start + 8,
                end: literal.start + 9,
            },
            arguments: Vec::new(),
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: literal.start + 9,
                end: literal.start + 10,
            },
        },
    };
    let source = source.replacen("\"made\"", "producer()", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive producer");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("recursive owned call");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}
