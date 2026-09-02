use super::*;

fn nested_owned_partial_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "unused = q; ";
    let (mut source, mut raw) = nested_owned_partial_return_snapshot();
    let local_start = source.rfind("const unused").expect("nested assignment target local");
    source.replace_range(local_start..local_start + 5, "let  ");
    let insertion = source.rfind("return q;").expect("nested assignment insertion");
    source.insert_str(insertion, ASSIGNMENT);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("nested assignment offset"),
        u32::try_from(ASSIGNMENT.len()).expect("nested assignment length"),
    );
    let return_value = insertion + ASSIGNMENT.len() + "return ".len();
    source.replace_range(return_value..=return_value, "unused");
    raw = shift_snapshot(
        raw,
        u32::try_from(return_value + 1).expect("nested assignment return growth start"),
        u32::try_from("unused".len() - 1).expect("nested assignment return growth"),
    );
    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("nested assignment span start"),
        end: u32::try_from(end).expect("nested assignment span end"),
    };
    let body = &mut raw.files[0].functions[0].body;
    let target_statement = body
        .statements
        .iter_mut()
        .find(|statement| {
            matches!(
                &statement.kind,
                RawStatementKind::LocalDeclaration { name, .. } if name.text == "unused"
            )
        })
        .expect("nested assignment target declaration");
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut target_statement.kind
    else {
        unreachable!("filtered target declaration")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let target = u32::try_from(body.expressions.len()).expect("nested assignment target");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 6),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "unused".to_owned(),
                span: s(insertion, insertion + 6),
            },
        },
    });
    let partial_source = u32::try_from(body.expressions.len()).expect("nested assignment source");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 9, insertion + 10),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(insertion + 9, insertion + 10),
            },
        },
    });
    let return_index = body.statements.len() - 1;
    body.statements.insert(
        return_index,
        RawStatementSyntax {
            span: s(insertion, insertion + 11),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 7, insertion + 8),
                value: partial_source,
                semicolon_span: s(insertion + 10, insertion + 11),
            },
        },
    );
    let return_start = insertion + ASSIGNMENT.len();
    let return_statement = &mut body.statements[return_index + 1];
    return_statement.span = s(return_start, return_value + 7);
    let RawStatementKind::Return { value, keyword_span, semicolon_span } =
        &mut return_statement.kind
    else {
        panic!("nested assignment return")
    };
    let value = *value;
    *keyword_span = s(return_start, return_start + 6);
    *semicolon_span = s(return_value + 6, return_value + 7);
    body.expressions[value as usize] = RawExpressionSyntax {
        span: s(return_value, return_value + 6),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "unused".to_owned(),
                span: s(return_value, return_value + 6),
            },
        },
    };
    body.blocks[0].statements =
        (0..u32::try_from(body.statements.len()).expect("nested assignment statements")).collect();
    (source, raw)
}

#[test]
fn partial_fixed_array_assignment_preserves_exact_elements_and_old_value_drop() {
    let (source, raw) = owned_array_partial_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial array assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("partial FixedArray assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf = roots[&1];
    let target_root = roots[&2];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let assignment_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial array assignment move");
    let replace = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::ReplacePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial array assignment replacement");
    let return_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial assigned array return");
    assert!(assignment_move < replace && replace < return_move);
    assert_eq!(
        instructions[replace]
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [target_root],
    );
    let assignment_value = instructions[assignment_move].result().expect("assignment value");
    let returned_value = instructions[return_move].result().expect("return value");
    let assignment_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == assignment_value)
        })
        .expect("partial array assignment temporary")
        .id();
    let return_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned_value)
        })
        .expect("partial array return temporary")
        .id();
    for root in [source_root, assignment_temporary, target_root, return_temporary] {
        assert_eq!(
            function
                .places()
                .filter_map(|place| match place.kind() {
                    VerifiedPlaceKind::FixedArrayConstant { base, index } if base == root => {
                        Some(index)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [0, 1],
        );
    }
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [moved_leaf],
    );
}

#[test]
fn nested_partial_struct_assignment_preserves_recursive_topology_and_cleanup() {
    let (source, raw) = nested_owned_partial_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("nested partial Struct assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let moved_leaf = roots[&1];
    let source_root = roots[&2];
    let target_root = roots[&3];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let assignment_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("nested assignment preparation");
    let replace = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::ReplacePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("nested assignment replacement");
    let return_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("nested assignment return");
    assert!(assignment_move < replace && replace < return_move);
    assert_eq!(
        instructions[replace]
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [target_root],
    );
    let assignment_value = instructions[assignment_move].result().expect("assignment value");
    let return_value = instructions[return_move].result().expect("return value");
    let assignment_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == assignment_value)
        })
        .expect("nested assignment temporary")
        .id();
    let return_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == return_value)
        })
        .expect("nested return temporary")
        .id();
    for root in [source_root, assignment_temporary, target_root, return_temporary] {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
        assert_eq!(
            function
                .places()
                .filter_map(|place| match place.kind() {
                    VerifiedPlaceKind::StructField { base, ordinal } if base == fields[&0] => {
                        Some(ordinal)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [0],
        );
    }
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [moved_leaf],
    );
}
