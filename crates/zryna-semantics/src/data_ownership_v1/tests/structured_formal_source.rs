use super::*;

#[test]
fn lexical_authority_is_rejected_before_crossing_a_match_edge() {
    let (text, raw) = structured_owned_fixture::lexical_match_fixture();
    let at = raw.files[0].functions[0]
        .body
        .expressions
        .iter()
        .find(|expression| {
            matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
        })
        .expect("match expression")
        .span;
    let sources = sources_for(&text);
    let syntax =
        verify_snapshot(raw, &sources).expect("authenticated lexical authority around match call");
    let first = lower(pair_input(&syntax, &sources))
        .expect_err("lexical carry requires separate authority");
    assert_eq!(
        first,
        lower(pair_input(&syntax, &sources)).expect_err("deterministic lexical exclusion")
    );
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3017");
    assert_eq!(first[0].primary_span(), Some(span(&sources, at)));
    assert_eq!(first[0].message(), "borrow authority cannot cross a structured ownership edge");
    assert_eq!(
        first[0].guidance(),
        "end lexical borrows before a branch, loop edge, or continuation"
    );
}

#[test]
fn structured_formal_wrong_access_and_missing_alias_reject_exactly_and_replay() {
    for wrong_access in [false, true] {
        let (mut text, mut raw) = structured_owned_fixture::formal_match_fixture(true);
        if wrong_access {
            let syntax = raw.files[0].functions[1].parameters[0].type_syntax;
            let node = &mut raw.files[0].type_syntax[syntax as usize];
            let RawTypeSyntaxKind::BorrowMut {
                mut keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            } = node.kind
            else {
                panic!("exclusive callee formal")
            };
            text.replace_range(keyword_span.start as usize..keyword_span.end as usize, "Borrow   ");
            keyword_span.end = keyword_span.start + 6;
            node.kind = RawTypeSyntaxKind::Borrow {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            };
        }
        let alias = raw.files[0].functions[0].body.expressions.iter_mut().find(|expression| matches!(&expression.kind, zryna_syntax::v4::RawExpressionKind::Reference { name } if name.text == "loan")).expect("forwarded alias operand");
        let at = alias.span;
        if !wrong_access {
            text.replace_range(at.start as usize..at.end as usize, "lost");
            let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut alias.kind else {
                unreachable!("alias reference")
            };
            name.text = "lost".into();
        }
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated wrong formal argument");
        let first = lower(pair_input(&syntax, &sources)).expect_err("invalid formal rejected");
        assert_eq!(
            first,
            lower(pair_input(&syntax, &sources)).expect_err("deterministic formal rejection")
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].code(), "ZRYNA-M3017");
        assert_eq!(first[0].primary_span(), Some(span(&sources, at)));
        assert_eq!(
            first[0].message(),
            "structured call requires an exact live call-frame borrow parameter"
        );
        assert_eq!(
            first[0].guidance(),
            "forward one matching formal authority without ending or reborrowing it across a match"
        );
    }
}

#[test]
fn structured_formal_authority_is_forwarded_across_match_without_end_or_reborrow() {
    for exclusive in [false, true] {
        let (text, raw) = structured_owned_fixture::formal_match_fixture(exclusive);
        let sources = sources_for(&text);
        let syntax =
            verify_snapshot(raw, &sources).expect("authenticated formal-first call with match");
        let program =
            lower(pair_input(&syntax, &sources)).expect("formal borrow survives exact match edges");
        let function = program
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("caller");
        let formal = function.borrow_parameters().next().expect("formal authority");
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 4);
        for instruction in blocks.iter().flat_map(|block| block.instructions()) {
            assert!(!matches!(
                instruction.kind(),
                VerifiedInstructionKind::BeginBorrow | VerifiedInstructionKind::EndBorrow
            ));
        }
        let call = blocks[3].instructions().next().expect("call after match");
        let arguments = call.call_arguments().collect::<Vec<_>>();
        assert_eq!(arguments.len(), 3);
        assert!(matches!(arguments[0], VerifiedCallArgument::Value(_)));
        assert!(matches!(arguments[1], VerifiedCallArgument::Value(_)));
        assert_eq!(arguments[2], VerifiedCallArgument::Borrow(formal.id()));
        assert_eq!(call.failure_ended_borrows().collect::<Vec<_>>(), vec![formal.id()]);
        for arm in &blocks[1..3] {
            assert_eq!(
                arm.instructions()
                    .next()
                    .expect("payload clone")
                    .failure_ended_borrows()
                    .collect::<Vec<_>>(),
                vec![formal.id()]
            );
        }
        let replay = lower(pair_input(&syntax, &sources)).expect("formal replay");
        assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
    }
}
