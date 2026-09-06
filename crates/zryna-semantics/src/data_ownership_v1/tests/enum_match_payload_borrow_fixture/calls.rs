use super::*;

pub(super) fn callee(builder: &mut Builder, payload: &Ty, exclusive: bool) -> RawFunctionSyntax {
    builder.text(" ");
    let start = builder.source.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("relay");
    builder.text("(");
    let parameter_start = builder.source.len();
    let parameter_name = builder.name("loan");
    builder.text(": ");
    let type_start = builder.source.len();
    let keyword_span = builder.text(if exclusive { "BorrowMut" } else { "Borrow" });
    let less_than_span = builder.text("<");
    let argument = builder.ty(payload);
    let greater_than_span = builder.text(">");
    let type_syntax = u32::try_from(builder.types.len()).expect("borrow type");
    builder.types.push(RawTypeSyntax {
        span: at(type_start, builder.source.len()),
        kind: if exclusive {
            RawTypeSyntaxKind::BorrowMut {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            }
        } else {
            RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
        },
    });
    let parameters = vec![RawParameterSyntax {
        span: at(parameter_start, builder.source.len()),
        name: parameter_name,
        type_syntax,
    }];
    builder.text("): ");
    let result_type = builder.ty(payload);
    builder.text(" ");
    let body_start = builder.source.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    let return_start = builder.source.len();
    let keyword_span = builder.text("return");
    builder.text(" ");
    let value = builder.clone_value(|builder| builder.reference("loan"));
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(return_start, builder.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = at(body_start, builder.source.len());
    RawFunctionSyntax {
        span: at(start, builder.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                statements: vec![0],
                close_brace_span,
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    }
}

#[test]
fn exhaustive_match_payload_calls_preserve_shared_and_exclusive_arm_authority() {
    for case in [MatchBorrowCase::CallShared, MatchBorrowCase::CallExclusive] {
        let (source, raw) = match_borrow_fixture(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated match payload call");
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
        let caller = program.modules().next().expect("module").functions().next().expect("caller");
        let blocks = caller.blocks().collect::<Vec<_>>();
        assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
        let instructions = blocks[2].instructions().collect::<Vec<_>>();
        let begin = instructions
            .iter()
            .position(|item| item.kind() == VerifiedInstructionKind::BeginBorrow)
            .expect("begin");
        let call = instructions
            .iter()
            .position(|item| item.kind() == VerifiedInstructionKind::DirectCall)
            .expect("call");
        let end = instructions
            .iter()
            .position(|item| item.kind() == VerifiedInstructionKind::EndBorrow)
            .expect("end");
        assert!(begin < call && call < end);
        let payload = instructions[begin].place_operands().next().expect("payload");
        let place = caller.places().find(|place| place.id() == payload).expect("payload place");
        assert!(matches!(place.kind(), VerifiedPlaceKind::EnumPayload { variant: 1, .. }));
        assert_eq!(
            instructions[begin].borrow_access(),
            Some(if matches!(case, MatchBorrowCase::CallExclusive) {
                VerifiedBorrowAccess::Exclusive
            } else {
                VerifiedBorrowAccess::Shared
            })
        );
        assert_eq!(
            instructions[call].failure_ended_borrows().collect::<Vec<_>>(),
            [instructions[begin].borrow().expect("borrow identity")]
        );
        assert_eq!(
            instructions[call]
                .cleanup()
                .and_then(|id| caller.cleanup_plans().find(|plan| plan.id() == id))
                .expect("call cleanup")
                .site()
                .role(),
            VerifiedCleanupRole::CallTrap
        );
        let failure_cleanup = instructions[call].derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(failure_cleanup.len(), 2);
        assert_eq!(failure_cleanup[0].active_variant(), None);
        assert_eq!(failure_cleanup[1].active_variant(), Some(1));
        assert_eq!(instructions[end].borrow(), instructions[begin].borrow());
    }
}

#[test]
fn exhaustive_match_payload_calls_reject_inactive_and_wrong_access_then_recover() {
    for (case, message) in [
        (
            MatchBorrowCase::CallInactive,
            "inline refined borrowing requires one active match payload binding",
        ),
        (
            MatchBorrowCase::CallWrongAccess,
            "refined payload call requires exact referent and borrow access",
        ),
    ] {
        let (source, raw) = match_borrow_fixture(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected payload call");
        let first = lower(pair_input(&syntax, &sources)).expect_err("payload call rejected");
        assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("deterministic replay"));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].code(), "ZRYNA-M3017");
        assert_eq!(first[0].message(), message);
    }

    let (source, raw) = match_borrow_fixture(MatchBorrowCase::CallShared);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("valid recovery source");
    lower(pair_input(&syntax, &sources)).expect("recovery after rejected match payload calls");
}
