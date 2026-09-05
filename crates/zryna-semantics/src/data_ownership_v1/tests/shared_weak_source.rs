use super::generic_vec_fixture::shared_weak_fixture::{Case, fixture, fixture_case};
use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::RawExpressionKind;

#[test]
fn shared_and_weak_source_operations_preserve_exact_handle_ownership() {
    let (source, raw) = fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated shared/weak fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("shared/weak source lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let operations = instructions
        .iter()
        .filter_map(|instruction| match instruction.kind() {
            VerifiedInstructionKind::SharedConstruct
            | VerifiedInstructionKind::SharedClone
            | VerifiedInstructionKind::WeakDowngrade
            | VerifiedInstructionKind::WeakClone => Some(instruction.kind()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
        ]
    );
    let cleanup_roots = instructions
        .iter()
        .filter(|instruction| {
            matches!(
                instruction.kind(),
                VerifiedInstructionKind::SharedConstruct
                    | VerifiedInstructionKind::SharedClone
                    | VerifiedInstructionKind::WeakDowngrade
                    | VerifiedInstructionKind::WeakClone
            )
        })
        .map(|instruction| {
            instruction
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cleanup_roots,
        [vec![1], vec![3], vec![5, 3], vec![7, 5, 3]],
        "allocation/count failures retain each source and release earlier owners in reverse order"
    );
    assert_eq!(
        function.blocks().next().expect("block").terminator().derived_drop_actions().count(),
        3,
        "returned Shared is excluded while cloned Shared and both Weak handles release"
    );
}

#[test]
fn shared_and_weak_source_lowering_replays_identically() {
    let (source, raw) = fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated shared/weak fixture");
    let first = format!("{:?}", lower(pair_input(&syntax, &sources)).expect("first lowering"));
    let second = format!("{:?}", lower(pair_input(&syntax, &sources)).expect("replay lowering"));
    assert_eq!(first, second);
}

#[test]
fn shared_and_weak_structural_payload_categories_lower_through_verified_ir() {
    for case in [
        Case::ScalarBool,
        Case::ScalarI32,
        Case::NominalEnum,
        Case::NominalStruct,
        Case::StringArrayZero,
        Case::StringArrayOne,
        Case::StringVec,
    ] {
        let (source, raw) = fixture_case(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated scalar handle fixture");
        let program = lower(pair_input(&syntax, &sources)).expect("scalar handle lowering");
        let kinds = program
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .next()
            .expect("block")
            .instructions()
            .filter_map(|instruction| match instruction.kind() {
                VerifiedInstructionKind::SharedConstruct
                | VerifiedInstructionKind::SharedClone
                | VerifiedInstructionKind::WeakDowngrade
                | VerifiedInstructionKind::WeakClone => Some(instruction.kind()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                VerifiedInstructionKind::SharedConstruct,
                VerifiedInstructionKind::SharedClone,
                VerifiedInstructionKind::WeakDowngrade,
                VerifiedInstructionKind::WeakClone,
            ]
        );
    }
}

#[test]
fn nested_shared_payload_moves_into_outer_control_without_implicit_clone() {
    let (source, raw) = fixture_case(Case::NestedShared);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated nested handle fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("nested handle lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let kinds = block
        .instructions()
        .filter_map(|instruction| match instruction.kind() {
            VerifiedInstructionKind::SharedConstruct
            | VerifiedInstructionKind::SharedClone
            | VerifiedInstructionKind::WeakDowngrade
            | VerifiedInstructionKind::WeakClone => Some(instruction.kind()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
        ]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 3);
}

#[test]
fn temporary_handle_operands_drop_at_the_exact_expression_boundary() {
    let (source, raw) = fixture_case(Case::TemporaryOperands);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated temporary handle fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("temporary handle lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let kinds = instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>();
    let handle_or_drop = kinds
        .iter()
        .copied()
        .filter(|kind| {
            matches!(
                kind,
                VerifiedInstructionKind::SharedConstruct
                    | VerifiedInstructionKind::SharedClone
                    | VerifiedInstructionKind::WeakDowngrade
                    | VerifiedInstructionKind::WeakClone
                    | VerifiedInstructionKind::DropPlace
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handle_or_drop,
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
            VerifiedInstructionKind::DropPlace,
        ],
        "each non-addressable operand is counted before its temporary is released"
    );
    let drops = instructions
        .iter()
        .enumerate()
        .filter(|(_, instruction)| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .collect::<Vec<_>>();
    assert_eq!(drops.len(), 3);
    for (drop_index, drop) in drops {
        let outer = instructions[drop_index - 1];
        let inner = instructions[drop_index - 2];
        let inner_result = inner.result().expect("inner handle result");
        let temporary = function
            .places()
            .find(|place| place.kind() == VerifiedPlaceKind::Temporary(inner_result))
            .expect("inner result has one temporary owner")
            .id();
        assert_eq!(outer.place_operands().collect::<Vec<_>>(), [temporary]);
        assert_eq!(drop.place_operands().collect::<Vec<_>>(), [temporary]);
        assert_eq!(
            outer.derived_drop_actions().next().map(|action| action.root()),
            Some(temporary),
            "outer count failure retains and first releases its temporary source"
        );
    }
}

#[test]
fn handle_leaves_compose_through_struct_array_vec_projection_and_replacement() {
    let (source, raw) = generic_vec_fixture::shared_weak_fixture::composition_fixture::fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated handle composition fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("handle composition lowering");
    let function =
        program.modules().next().expect("module").functions().nth(1).expect("composition caller");
    let kinds = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(|instruction| instruction.kind())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&VerifiedInstructionKind::StructConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::EnumConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::FixedArrayConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::VecConstruct));
    assert!(kinds.contains(&VerifiedInstructionKind::DirectCall));
    assert!(kinds.contains(&VerifiedInstructionKind::ReplacePlace));
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::SharedClone).count(),
        2,
        "root and projected Shared clones are explicit count operations"
    );
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::WeakClone).count(),
        1,
        "fixed-array Weak projection clone is an explicit count operation"
    );
}

#[test]
fn structural_handle_clone_seals_struct_enum_array_and_vec_count_recipes() {
    let (source, raw) =
        generic_vec_fixture::shared_weak_fixture::composition_fixture::clone_rejection_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated structural clone fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("handle-aware clone lowering");
    let function =
        program.modules().next().expect("module").functions().nth(1).expect("composition caller");
    let clones = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .filter_map(|instruction| {
            instruction.handle_aware_clone().map(|clone| (instruction, clone))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        clones.len(),
        6,
        "place/indexed Struct, indexed Shared leaf, Enum, fixed array, and Vec clones"
    );
    assert_eq!(
        clones
            .iter()
            .filter(|(_, clone)| matches!(
                clone.source(),
                zryna_ir::data_ownership_v1::VerifiedHandleAwareCloneSource::Borrow(_)
            ))
            .count(),
        2,
        "indexed aggregate and direct Shared leaf retain borrow authority"
    );
    assert!(clones.iter().all(|(instruction, clone)| {
        let prefix =
            instruction.handle_aware_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
        assert_eq!(prefix[0].root(), clone.destination());
        assert_eq!(&prefix[1..], instruction.derived_drop_actions().collect::<Vec<_>>());
        if let zryna_ir::data_ownership_v1::VerifiedHandleAwareCloneSource::Place(source) =
            clone.source()
        {
            assert_ne!(source, clone.destination());
        }
        clone.frontier().nodes().any(|node| {
            matches!(
                node.kind(),
                VerifiedHandleCloneRecipeKind::SharedCountClone
                    | VerifiedHandleCloneRecipeKind::WeakCountClone
            )
        })
    }));
}

#[test]
fn structural_handle_clone_reports_missing_source_before_capability_boundary() {
    let (source, raw) =
        generic_vec_fixture::shared_weak_fixture::composition_fixture::missing_clone_source_fixture(
        );
    let missing_at = raw.files[0]
        .functions
        .iter()
        .flat_map(|function| &function.body.expressions)
        .find_map(|expression| match &expression.kind {
            RawExpressionKind::Reference { name } if name.text == "ghostx" => Some(name.span),
            _ => None,
        })
        .expect("missing structural clone source");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated missing source fixture");
    let expected = vec![Diagnostic::error_at(
        "ZRYNA-M3002",
        span(&sources, missing_at),
        "aggregate binding 'ghostx' is not declared in this function",
        "clone one preceding available aggregate local",
    )];
    for _ in 0..2 {
        assert_eq!(lower(pair_input(&syntax, &sources)).expect_err("missing source"), expected);
    }
}

fn rejected(case: Case, code: &str, message: &str, guidance: &str) {
    let (source, raw) = fixture_case(case);
    let body = &raw.files[0].functions[0].body;
    let expression = body
        .expressions
        .iter()
        .rev()
        .find(|expression| matches!(expression.kind, RawExpressionKind::Clone { .. }))
        .expect("rejected clone expression");
    let RawExpressionKind::Clone { value, .. } = expression.kind else {
        unreachable!("selected clone")
    };
    let operation_at = expression.span;
    let at = body.expressions[value as usize].span;
    assert!(source[operation_at.start as usize..operation_at.end as usize].starts_with("clone("));
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected handle source");
    let expected = vec![Diagnostic::error_at(code, span(&sources, at), message, guidance)];
    for _ in 0..2 {
        assert_eq!(lower(pair_input(&syntax, &sources)).expect_err("handle rejection"), expected);
    }
}

#[test]
fn moved_handle_clone_reports_exact_diagnostic_and_replays() {
    rejected(
        Case::MovedReuse,
        "ZRYNA-M3014",
        "shared or weak handle is moved or unavailable",
        "use one complete initialized handle before moving it",
    );
}

#[test]
fn wrong_handle_clone_type_reports_exact_diagnostic_and_replays() {
    rejected(
        Case::WrongCloneType,
        "ZRYNA-M3013",
        "shared or weak operation has the wrong exact handle type",
        "clone one exact handle or downgrade Shared<T> to Weak<T>",
    );
}

#[test]
fn missing_handle_source_reports_exact_diagnostic_and_replays() {
    rejected(
        Case::MissingHandle,
        "ZRYNA-M3002",
        "aggregate value 'ghost' is not declared",
        "reference one exact preceding local using its declared spelling",
    );
}
