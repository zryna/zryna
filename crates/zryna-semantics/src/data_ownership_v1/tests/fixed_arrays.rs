use super::*;

const ARRAY_CONSTRUCT_SOURCE: &str =
    "function make(): FixedArray<i32, 2> { return FixedArray<i32, 2>([1, 2]); }";
const ARRAY_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":56,"end":59},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":56,"end":59}}}},{"span":{"file":0,"start":45,"end":63},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":45,"end":55},"less_than_span":{"file":0,"start":55,"end":56},"element":2,"comma_span":{"file":0,"start":59,"end":60},"length_span":{"file":0,"start":61,"end":62},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":62,"end":63}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":74},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":36,"end":74},"root_block":0,"blocks":[{"span":{"file":0,"start":36,"end":74},"open_brace_span":{"file":0,"start":36,"end":37},"statements":[0],"close_brace_span":{"file":0,"start":73,"end":74}}],"statements":[{"span":{"file":0,"start":38,"end":72},"kind":{"kind":"return","keyword_span":{"file":0,"start":38,"end":44},"value":2,"semicolon_span":{"file":0,"start":71,"end":72}}}],"expressions":[{"span":{"file":0,"start":65,"end":66},"kind":{"kind":"i32-literal","spelling":"1"}},{"span":{"file":0,"start":68,"end":69},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":45,"end":71},"kind":{"kind":"fixed-array-construction","type_syntax":3,"open_paren_span":{"file":0,"start":63,"end":64},"open_bracket_span":{"file":0,"start":64,"end":65},"elements":[0,1],"close_bracket_span":{"file":0,"start":69,"end":70},"close_paren_span":{"file":0,"start":70,"end":71}}}]}}]}],"diagnostics":[]}}"#;

#[test]
fn fixed_array_constant_projection_accepts_last_index() {
    let sources = sources_for(ARRAY_VALID_SOURCE);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index literal")
    };
    *spelling = "1".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful fixed array v4");
    let program = lower(pair_input(&syntax, &sources)).expect("last fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 1, .. }
    )));
}

#[test]
fn fixed_array_constant_projection_accepts_zero_index() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "0".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful zero index");
    let program = lower(pair_input(&syntax, &sources)).expect("zero fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(
        function.parameters().len()
            + block.parameters().len()
            + block.instructions().filter(|instruction| instruction.result().is_some()).count(),
        2,
        "fixed-array constant index spelling is not emitted as a runtime value",
    );
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 0, .. }
    )));
}

#[test]
fn fixed_array_constructor_requires_exact_count_and_element_type() {
    let sources = sources_for(ARRAY_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), &sources)
        .expect("array constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact fixed-array constructor");

    let mut missing_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    missing_source.replace_range(66..69, "   ");
    let missing_sources = sources_for(&missing_source);
    let mut missing = response_snapshot(ARRAY_CONSTRUCT_RESPONSE);
    let body = &mut missing.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut body.expressions[1].kind
    else {
        panic!("array constructor")
    };
    elements.pop();
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 1;
    let syntax = verify_snapshot(missing, &missing_sources).expect("source-faithful short array");
    assert_eq!(
        lower(pair_input(&syntax, &missing_sources)).expect_err("short array")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(65..66, "true");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), 66, 3);
    typed.files[0].functions[0].body.expressions[0].kind =
        zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true };
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped array");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped array");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped element child");
    assert_eq!((primary.start(), primary.end()), (65, 69));
}

#[test]
fn fixed_array_index_equal_to_length_is_rejected() {
    let sources = sources_for(ARRAY_OOB_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_RESPONSE), &sources)
        .expect("source-faithful fixed array v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("index N is out of bounds");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn dynamic_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "x");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1].kind =
        zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "x".to_owned(), span },
        };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful dynamic index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("dynamic index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("dynamic index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn negative_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "-1");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 55, 1);
    let body = &mut raw.files[0].functions[0].body;
    let mut index = body.expressions.pop().expect("index expression");
    let literal = &mut body.expressions[1];
    literal.span.start = 55;
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } = &mut literal.kind else {
        panic!("literal")
    };
    *spelling = "1".to_owned();
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 56 },
        kind: zryna_syntax::v4::RawExpressionKind::Negation {
            operator_span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 55 },
            operand: 1,
        },
    });
    let zryna_syntax::v4::RawExpressionKind::Index { index: index_id, .. } = &mut index.kind else {
        panic!("index")
    };
    *index_id = 2;
    body.expressions.push(index);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 3;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful negative index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("negative index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("negative index child");
    assert_eq!((primary.start(), primary.end()), (54, 56));
}

#[test]
fn owned_fixed_array_accepts_disjoint_string_projection_moves() {
    let (source, raw) = owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Disjoint);
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("source-faithful disjoint array projections");
    let program = lower(pair_input(&syntax, &sources)).expect("disjoint array projection moves");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let projected = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                Some((index, place.id()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(projected.iter().map(|(index, _)| *index).collect::<Vec<_>>(), vec![0, 1]);
    let moved = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::MoveFromPlace)
        .filter_map(|instruction| instruction.place_operands().next())
        .filter(|place| projected.iter().any(|(_, projected)| projected == place))
        .count();
    assert_eq!(moved, 2);
}
#[test]
fn projected_string_clone_preserves_a_disjoint_partial_root_mask() {
    let (source, raw) =
        owned_array_projected_clone_return_snapshot(OwnedArrayProjectionCase::Disjoint, 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful disjoint projected clone");
    let program = lower(pair_input(&syntax, &sources)).expect("disjoint projected String clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let projected = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                Some((index, place.id()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let moved = projected.iter().find(|(index, _)| *index == 0).expect("moved element").1;
    let cloned = projected.iter().find(|(index, _)| *index == 1).expect("cloned element").1;
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let move_index = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(moved)
        })
        .expect("first element move");
    let clone_index = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::StringClone
                && instruction.place_operands().next() == Some(cloned)
        })
        .expect("second element clone");
    let construct_index = instructions
        .iter()
        .rposition(|instruction| instruction.kind() == VerifiedInstructionKind::FixedArrayConstruct)
        .expect("result array construction");
    assert!(
        move_index < clone_index && clone_index < construct_index,
        "move={move_index}, clone={clone_index}, construct={construct_index}, kinds={:?}",
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
    );
    let cleanup = instructions[clone_index]
        .derived_drop_actions()
        .find(|action| action.root() == root)
        .expect("partially moved root clone cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [moved]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [cloned]);
    let exit = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == root)
        .expect("partially moved source root exit cleanup");
    assert_eq!(exit.moved_projections().collect::<Vec<_>>(), [moved]);
    assert_eq!(exit.initialized_projections().collect::<Vec<_>>(), [cloned]);
}
#[test]
#[allow(clippy::too_many_lines)]
fn partial_fixed_array_owner_transfers_with_exact_topology_and_mask() {
    let (source, raw) = owned_array_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("partial FixedArray transfer");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let target_root = roots[&2];
    let elements = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                    Some((index, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let source_elements = elements(source_root);
    let target_elements = elements(target_root);
    assert_eq!(source_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(target_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let projected_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_elements[&0])
        })
        .expect("first element move");
    let whole_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial array whole move");
    let initialize = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::InitializePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial array target initialization");
    let clone = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::StringClone
                && instruction.place_operands().next() == Some(target_elements[&1])
        })
        .expect("target second element clone");
    let construct = instructions
        .iter()
        .rposition(|instruction| instruction.kind() == VerifiedInstructionKind::FixedArrayConstruct)
        .expect("result array construction");
    assert!(projected_move < whole_move && whole_move < initialize);
    assert!(initialize < clone && clone < construct);
    let transfer_value = instructions[whole_move].result().expect("array transfer value");
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == transfer_value)
        })
        .expect("array transfer temporary")
        .id();
    assert_eq!(elements(temporary).keys().copied().collect::<Vec<_>>(), [0, 1]);
    for actions in [
        instructions[clone].derived_drop_actions().collect::<Vec<_>>(),
        block.terminator().derived_drop_actions().collect::<Vec<_>>(),
    ] {
        let cleanup = actions
            .iter()
            .find(|action| action.root() == target_root)
            .expect("transferred array cleanup");
        assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [target_elements[&0]]);
        assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [target_elements[&1]]);
        assert!(
            actions.iter().all(|action| action.root() != source_root && action.root() != temporary)
        );
    }
    let clone_instruction = instructions[clone];
    for status in [RuntimeStatus::Allocation, RuntimeStatus::Capacity, RuntimeStatus::AbiViolation]
    {
        let injection =
            OwnedFaultInjection::Runtime { operation: LogicalOperation::StringClone, status };
        let first = owned_fault_trace(abi, function, clone_instruction, injection, 0, 1)
            .expect("transferred array clone fault");
        let replay = owned_fault_trace(abi, function, clone_instruction, injection, 0, 1)
            .expect("deterministic transferred array clone fault");
        assert_eq!(first, replay);
        assert!(!first.result_committed);
        assert_eq!(first.uncommitted_result, clone_instruction.result());
        assert!(first.retained_roots.contains(&target_root));
        assert!(first.reverse_cleanup.contains(&target_root));
        assert!(!first.retained_roots.contains(&source_root));
        assert!(!first.retained_roots.contains(&temporary));
    }
}
#[test]
fn partial_fixed_array_owner_returns_with_exact_topology_and_survivor_cleanup() {
    let (source, raw) = owned_array_partial_then_root_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial array return");
    let program = lower(pair_input(&syntax, &sources)).expect("partial FixedArray return");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf_owner = roots[&1];
    let block = function.blocks().next().expect("block");
    let whole_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial array return move");
    let returned = block.terminator().value_operands().next().expect("returned value");
    assert_eq!(whole_move.result(), Some(returned));
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned)
        })
        .expect("partial array return temporary")
        .id();
    let elements = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                    Some((index, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let source_elements = elements(source_root);
    let returned_elements = elements(temporary);
    assert_eq!(source_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(returned_elements.keys().copied().collect::<Vec<_>>(), [0, 1]);
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].root(), moved_leaf_owner);
    assert!(cleanup.iter().all(|action| {
        action.root() != source_root
            && action.root() != temporary
            && action.root() != returned_elements[&0]
    }));
}
