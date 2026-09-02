use super::borrow_parameter_calls::{mixed_snapshot, mixed_source};
use super::*;

const SOURCE: &str = include_str!("../../../../../tests/m3-fixtures/borrow-forwarding-shared.zry");
const JSON: &[u8] =
    include_bytes!("../../../../../tests/m3-fixtures/borrow-forwarding-shared.json");

#[test]
fn lexical_authority_is_forwarded_unchanged_and_ended_only_by_its_caller() {
    let sources = sources_for(SOURCE);
    let raw = decode_snapshot(JSON).expect("borrow forwarding fixture");
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful borrow forwarding fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("borrow forwarding lowering");
    let module = program.modules().next().expect("module");
    let functions = module.functions().collect::<Vec<_>>();
    let [sink, relay, caller] = functions.as_slice() else { panic!("three functions") };

    let sink_parameter = sink.borrow_parameters().next().expect("sink borrow parameter");
    let relay_parameter = relay.borrow_parameters().next().expect("relay borrow parameter");
    assert_eq!(sink_parameter.id().index(), 0);
    assert_eq!(relay_parameter.id().index(), 0);
    assert_eq!(sink_parameter.access(), VerifiedBorrowAccess::Shared);
    assert_eq!(relay_parameter.access(), VerifiedBorrowAccess::Shared);

    let relay_instructions =
        relay.blocks().next().expect("relay block").instructions().collect::<Vec<_>>();
    let relay_call = relay_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("relay DirectCall");
    assert!(matches!(
        relay_call.call_arguments().nth(2),
        Some(VerifiedCallArgument::Borrow(borrow)) if borrow.index() == relay_parameter.id().index()
    ));
    assert!(!relay_instructions.iter().any(|instruction| {
        matches!(
            instruction.kind(),
            VerifiedInstructionKind::BeginBorrow | VerifiedInstructionKind::EndBorrow
        )
    }));

    let caller_instructions =
        caller.blocks().next().expect("caller block").instructions().collect::<Vec<_>>();
    let begin = caller_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("caller BeginBorrow");
    let call = caller_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("caller DirectCall");
    let end = caller_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("caller EndBorrow");
    let authority = begin.borrow().expect("begun authority");
    assert_eq!(authority.index(), 0);
    assert!(matches!(
        call.call_arguments().nth(2),
        Some(VerifiedCallArgument::Borrow(borrow)) if borrow == authority
    ));
    assert_eq!(end.borrow(), Some(authority));
    let begin_index = caller_instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("begin index");
    let call_index = caller_instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("call index");
    let end_index = caller_instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("end index");
    assert!(begin_index < call_index && call_index < end_index);
}

#[test]
fn forwarding_replay_is_deterministic_after_a_later_borrow_slot_rejection() {
    let arguments = ["left", "left", "right", "exclusive"];
    let rejected_source = mixed_source("exclusive", &arguments, false);
    let rejected_sources = sources_for(&rejected_source);
    let rejected_syntax = verify_snapshot(
        mixed_snapshot(&rejected_source, "exclusive", &arguments, false),
        &rejected_sources,
    )
    .expect("source-faithful rejected forwarding call");
    let diagnostics = lower(pair_input(&rejected_syntax, &rejected_sources))
        .expect_err("later borrow slot rejection");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].message(),
        "borrow arguments must forward an in-scope borrow parameter"
    );

    let accepted = ["left", "shared", "right", "exclusive"];
    let source = mixed_source("exclusive", &accepted, false);
    let sources = sources_for(&source);
    for _ in 0..2 {
        let syntax =
            verify_snapshot(mixed_snapshot(&source, "exclusive", &accepted, false), &sources)
                .expect("source-faithful accepted forwarding call");
        let program = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
        let caller = program.modules().next().expect("module").functions().nth(1).expect("caller");
        assert_eq!(
            caller
                .blocks()
                .next()
                .expect("block")
                .instructions()
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>(),
            [
                VerifiedInstructionKind::CopyFromPlace,
                VerifiedInstructionKind::CopyFromPlace,
                VerifiedInstructionKind::DirectCall,
            ]
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn post_preflight_argument_failure_restores_the_full_lowerer_snapshot_before_replay() {
    use super::copy_calls::{copy_aggregate_call_snapshot, copy_aggregate_call_source};

    let mut source = copy_aggregate_call_source().to_owned();
    let return_start = source.rfind("p.x + q.x").expect("aggregate return expression");
    let return_end = return_start + "p.x + q.x".len();
    source.replace_range(return_start..return_end, "true + loan + p.x");
    let mut snapshot = shift_snapshot(
        copy_aggregate_call_snapshot(),
        u32::try_from(return_end).expect("return end"),
        u32::try_from("true + loan + p.x".len() - "p.x + q.x".len()).expect("return growth"),
    );
    let expression_start = u32::try_from(return_start).expect("return start");
    let expression_end = expression_start + 17;
    let raw_span = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let caller_body = &mut snapshot.files[0].functions[1].body;
    let nested_argument = caller_body.expressions[0].clone();
    let nested_call = caller_body.expressions[1].clone();
    caller_body.expressions = vec![
        nested_argument,
        nested_call,
        RawExpressionSyntax {
            span: raw_span(expression_start, expression_start + 4),
            kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true },
        },
        RawExpressionSyntax {
            span: raw_span(expression_start + 7, expression_start + 11),
            kind: zryna_syntax::v4::RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "loan".to_owned(),
                    span: raw_span(expression_start + 7, expression_start + 11),
                },
            },
        },
        RawExpressionSyntax {
            span: raw_span(expression_start, expression_start + 11),
            kind: zryna_syntax::v4::RawExpressionKind::Addition {
                operator_span: raw_span(expression_start + 5, expression_start + 6),
                lhs: 2,
                rhs: 3,
            },
        },
        RawExpressionSyntax {
            span: raw_span(expression_start + 14, expression_start + 15),
            kind: zryna_syntax::v4::RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "p".to_owned(),
                    span: raw_span(expression_start + 14, expression_start + 15),
                },
            },
        },
        RawExpressionSyntax {
            span: raw_span(expression_start + 14, expression_end),
            kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                base: 5,
                dot_span: raw_span(expression_start + 15, expression_start + 16),
                field: RawIdentifierSyntax {
                    text: "x".to_owned(),
                    span: raw_span(expression_start + 16, expression_end),
                },
            },
        },
        RawExpressionSyntax {
            span: raw_span(expression_start, expression_end),
            kind: zryna_syntax::v4::RawExpressionKind::Addition {
                operator_span: raw_span(expression_start + 12, expression_start + 13),
                lhs: 4,
                rhs: 6,
            },
        },
    ];
    let RawStatementKind::Return { value, .. } = &mut caller_body.statements[1].kind else {
        panic!("caller must end with the return");
    };
    *value = 7;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("source-faithful rollback expressions");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified Copy layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    let mut catalog = super::super::build_function_catalog(
        input,
        &declarations,
        &graph,
        &node_types,
        &mut setup_errors,
    );
    assert!(setup_errors.finish().is_empty());
    let caller_signature = catalog.modules[0][1].as_ref().expect("caller signature");
    let aggregate_ty = caller_signature.parameters[0];
    let i32_ty = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::I32)
        .copied()
        .expect("i32 type");
    let function = &syntax.files()[0].functions()[1];
    let at = span(&sources, function.name.span);
    catalog.modules[0].push(Some(FunctionSignature {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 2 },
        name: "sink".to_owned(),
        parameters: vec![aggregate_ty, i32_ty, i32_ty, i32_ty],
        borrow_parameters: vec![FunctionBorrowParameter {
            referent: i32_ty,
            access: raw::BorrowAccess::Shared,
            span: at,
        }],
        parameter_order: vec![
            FunctionParameterOrder::Value(0),
            FunctionParameterOrder::Value(1),
            FunctionParameterOrder::Value(2),
            FunctionParameterOrder::Value(3),
            FunctionParameterOrder::Borrow(0),
        ],
        result: i32_ty,
        private: true,
    }));
    let parameter_span = span(&sources, function.parameters[0].span);
    let parameter_place = raw::PlaceId(0);
    let mut errors = Errors::new(&sources);
    let mut lowerer = super::super::FunctionLowerer {
        input,
        file: &syntax.files()[0],
        function,
        module: 0,
        declarations: &declarations,
        graph: &graph,
        node_types: &node_types,
        layouts: &layouts,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::from([(
            "p".to_owned(),
            super::super::Binding { ty: aggregate_ty, place: parameter_place, mutable: false },
        )]),
        borrow_bindings: std::collections::BTreeMap::from([(
            "loan".to_owned(),
            super::super::BorrowBinding {
                ty: i32_ty,
                borrow: raw::BorrowId(0),
                access: raw::BorrowAccess::Shared,
            },
        )]),
        places: vec![raw::Place {
            id: parameter_place,
            ty: aggregate_ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(0),
        }],
        projections: std::collections::BTreeMap::new(),
        instructions: Vec::new(),
        cleanup_plans: Vec::new(),
        values: 1,
    };
    let callee = RawIdentifierSyntax { text: "sink".to_owned(), span: function.name.span };
    let rejected_arguments = [1, 6, 6, 7, 3];
    let signature = lowerer.resolve_copy_call(&callee, &rejected_arguments, at).expect("sink");
    let borrows = lowerer
        .preflight_copy_borrow_call(&signature, &rejected_arguments, at)
        .expect("preflight-valid rejected call");
    let before = lowerer.mutation_snapshot();
    assert!(
        lowerer.lower_direct_call_arguments(&signature, &rejected_arguments, borrows).is_none()
    );
    let mutated = lowerer.mutation_snapshot();
    let before_shape = before.shape();
    let mutated_shape = mutated.shape();
    assert!(mutated_shape.0 > before_shape.0, "derived value IDs must advance");
    assert!(mutated_shape.1 > before_shape.1, "projected places must be allocated");
    assert!(mutated_shape.2 > before_shape.2, "instructions must be emitted");
    assert!(mutated_shape.3 > before_shape.3, "nested call cleanup must be allocated");
    assert!(mutated_shape.4 > before_shape.4, "projection cache must be populated");
    lowerer.restore_mutation_snapshot(before.clone());
    assert_eq!(lowerer.mutation_snapshot(), before);

    assert!(lowerer.direct_call(&callee, &rejected_arguments, at).is_none());
    assert_eq!(lowerer.mutation_snapshot(), before);
    let accepted_arguments = [1, 6, 6, 6, 3];
    assert!(lowerer.direct_call(&callee, &accepted_arguments, at).is_some());
    assert_ne!(lowerer.mutation_snapshot(), before);
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.code() == "ZRYNA-M3007"
            && diagnostic.message() == "left operand has a different exact aggregate type"
    }));
}
