use super::*;

fn joined_test_span(start: u32, end: u32) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan { file: 0, start, end }
}

#[test]
fn private_vec_producer_and_identity_calls_transfer_exact_owners() {
    for element in ["bool", "i32", "String"] {
        let (source, raw) = private_vec_call_fixture(element);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful private Vec calls");
        let program =
            lower(pair_input(&syntax, &sources)).expect("Vec producer and identity calls");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        let caller = &functions[0];
        let identity = &functions[1];
        let caller_block = caller.blocks().next().expect("caller block");
        let calls = caller_block
            .instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 2, "{element}");
        assert_eq!(calls[0].callee().expect("producer").declaration(), 2);
        assert_eq!(calls[0].call_arguments().count(), 0);
        assert_eq!(calls[1].callee().expect("identity").declaration(), 1);
        assert_eq!(calls[1].call_arguments().count(), 1);
        assert_ne!(
            calls[0].cleanup().expect("producer cleanup"),
            calls[1].cleanup().expect("identity cleanup")
        );
        for call in &calls {
            let result = call.result().expect("Vec call result");
            assert_eq!(
                caller
                    .places()
                    .filter(|place| {
                        matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == result)
                    })
                    .count(),
                1,
                "{element} call result has one exact Temporary owner"
            );
        }
        for call in calls {
            assert_eq!(
                call.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
                [1],
                "{element} CallTrap retains only survivor"
            );
            assert_eq!(
                caller
                    .cleanup_plans()
                    .find(|plan| plan.id() == call.cleanup().expect("CallTrap"))
                    .expect("plan")
                    .site()
                    .role(),
                VerifiedCleanupRole::CallTrap
            );
        }
        assert!(identity.places().any(|place| { place.kind() == VerifiedPlaceKind::Parameter(0) }));
        assert_eq!(identity.parameters().next().expect("dense Vec parameter").id().index(), 0);
        assert_eq!(
            identity
                .blocks()
                .next()
                .expect("identity block")
                .terminator()
                .derived_drop_actions()
                .count(),
            0
        );
        assert_eq!(caller_block.terminator().derived_drop_actions().count(), 1);
    }
}

#[test]
fn private_vec_identity_call_accepts_nested_string_retaining_construction() {
    let (source, raw) = private_vec_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec call");
    let program = lower(pair_input(&syntax, &sources)).expect("nested Vec identity call");
    let caller =
        program.verified_ir().modules().next().expect("module").functions().next().expect("caller");
    let kinds = caller
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&VerifiedInstructionKind::StringConcat));
    assert!(kinds.contains(&VerifiedInstructionKind::VecConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::DirectCall));
}

#[test]
fn private_vec_direct_call_reports_unsupported_nested_string_element_once_at_expression() {
    let (mut source, raw) = private_vec_nested_string_call_fixture();
    let old = "concat(\"a\", \"b\")";
    let start = u32::try_from(source.find(old).expect("nested concat")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        "1",
    );
    let mut raw =
        shift_snapshot_signed(raw, old_end, 1 - i32::try_from(old.len()).expect("length"));
    let body = &mut raw.files[0].functions[0].body;
    let survivor = body.expressions[0].clone();
    let mut construct = body.expressions[4].clone();
    let mut identity = body.expressions[5].clone();
    let returned = body.expressions[6].clone();
    let literal_span = zryna_source::UntrustedSpan { file: 0, start, end: start + 1 };
    let literal = RawExpressionSyntax {
        span: literal_span,
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() },
    };
    let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } = &mut construct.kind
    else {
        panic!("nested construction")
    };
    *elements = vec![1];
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut identity.kind else {
        panic!("identity call")
    };
    *arguments = vec![2];
    body.expressions = vec![survivor, literal, construct, identity, returned];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("result declaration")
    };
    *initializer = 3;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 4;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested call rejection");
    let first = lower(pair_input(&syntax, &sources)).expect_err("unsupported nested call element");
    let second =
        lower(pair_input(&syntax, &sources)).expect_err("deterministic nested call rejection");
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3013");
    assert_eq!(first[0].primary_span(), Some(span(&sources, literal_span)));
}

#[test]
fn private_vec_call_rejects_moved_argument_reuse_and_wrong_case() {
    let (source, raw) = private_vec_call_fixture("i32");
    let call_end = raw.files[0].functions[0].body.expressions[1].span.end;
    let mut raw = shift_snapshot_signed(raw, call_end, -2);
    let argument_span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: argument_span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: argument_span },
        },
    };
    let old_return = raw.files[0].functions[0].body.expressions[3].span;
    let mut raw = shift_snapshot_signed(raw, old_return.end, 2);
    let returned = joined_test_span(old_return.start, old_return.end + 2);
    raw.files[0].functions[0].body.expressions[3] = RawExpressionSyntax {
        span: returned,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: returned },
        },
    };
    let source = source.replacen("producer()", "survivor", 1).replacen(
        "return result",
        "return survivor",
        1,
    );
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved Vec reuse");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved Vec reuse");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3014"));

    for replacement in ["Producer", "missingx"] {
        let (source, mut raw) = private_vec_call_fixture("i32");
        let source = source.replacen("producer()", &format!("{replacement}()"), 1);
        let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } =
            &mut raw.files[0].functions[0].body.expressions[1].kind
        else {
            panic!("producer call")
        };
        callee.text = replacement.to_owned();
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved Vec call");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved Vec call");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    }
}

#[test]
fn private_vec_call_rejects_arity_export_and_cycles() {
    let (source, raw) = private_vec_call_fixture("i32");
    let removed = raw.files[0].functions[0].body.expressions[1].span;
    let mut raw = shift_snapshot_signed(raw, removed.end, -10);
    let caller = &mut raw.files[0].functions[0];
    caller.body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } =
        &mut caller.body.expressions[1].kind
    else {
        panic!("identity call")
    };
    arguments.clear();
    let RawStatementKind::LocalDeclaration { initializer, .. } =
        &mut caller.body.statements[1].kind
    else {
        panic!("result local")
    };
    *initializer = 1;
    let RawStatementKind::Return { value, .. } = &mut caller.body.statements[2].kind else {
        panic!("return")
    };
    *value = 2;
    let source = source.replacen("identity(producer())", "identity()", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec call arity");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Vec call arity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));

    let (source, raw) = private_vec_call_fixture("i32");
    let cutoff = raw.files[0].functions[1].span.start;
    let mut raw = shift_snapshot(raw, cutoff, 7);
    let identity = &mut raw.files[0].functions[1];
    identity.export_span = Some(joined_test_span(cutoff, cutoff + 6));
    identity.span.start = cutoff;
    let source = source.replacen(" function identity", " export function identity", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported Vec callee");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("exported Vec callee");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));

    let (mut source, mut raw) = private_vec_call_fixture("i32");
    for element_id in [8_usize, 10] {
        let old = raw.files[0].type_syntax[element_id].span;
        source.replace_range(
            usize::try_from(old.start).expect("start")..usize::try_from(old.end).expect("end"),
            "bool",
        );
        raw = shift_snapshot(raw, old.end, 1);
        let at = raw.files[0].type_syntax[element_id].span;
        raw.files[0].type_syntax[element_id].kind = RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: at },
        };
    }
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec family mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Vec family mismatch");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));

    let (source, raw) = private_vec_call_fixture("i32");
    let old = raw.files[0].functions[1].body.expressions[0].span;
    let mut raw = shift_snapshot_signed(raw, old.end, 10);
    let expression = &mut raw.files[0].functions[1].body.expressions[0];
    expression.span = joined_test_span(old.start, old.end + 10);
    expression.kind = zryna_syntax::v4::RawExpressionKind::Call {
        callee: RawIdentifierSyntax {
            text: "identity".to_owned(),
            span: joined_test_span(old.start, old.start + 8),
        },
        open_paren_span: joined_test_span(old.start + 8, old.start + 9),
        arguments: vec![0],
        close_paren_span: joined_test_span(old.end + 9, old.end + 10),
    };
    let reference = RawExpressionSyntax {
        span: joined_test_span(old.start + 9, old.end + 9),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "value".to_owned(),
                span: joined_test_span(old.start + 9, old.end + 9),
            },
        },
    };
    raw.files[0].functions[1].body.expressions.insert(0, reference);
    let RawStatementKind::Return { value, .. } =
        &mut raw.files[0].functions[1].body.statements[0].kind
    else {
        panic!("identity return")
    };
    *value = 1;
    let source = source.replacen("return value", "return identity(value)", 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive Vec identity");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("recursive Vec identity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}
