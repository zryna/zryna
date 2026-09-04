use super::*;
use crate::data_ownership_v1::tests::mixed_string_calls::mixed_string_call_fixture;

const ROOM: Limits = Limits { work: 10_000, stack: 100, events: 100 };

fn with_program(
    source: &str,
    raw: RawProjectSyntaxSnapshot,
    check: impl FnOnce(&crate::data_ownership_v1::VerifiedProgram),
) {
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated witness source");
    let program = lower(pair_input(&syntax, &sources)).expect("independent witness full IR");
    check(&program);
}

fn check_events(
    source: &str,
    raw: RawProjectSyntaxSnapshot,
    expected: &[(usize, &[(char, usize)])],
) {
    with_program(source, raw, |program| {
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        let sites = instructions
            .iter()
            .enumerate()
            .filter_map(|(index, i)| {
                matches!(
                    i.kind(),
                    VerifiedInstructionKind::StringFromUtf8 | VerifiedInstructionKind::VecConstruct
                )
                .then_some(index)
            })
            .collect::<Vec<_>>();
        assert_eq!(sites, expected.iter().map(|(index, _)| *index).collect::<Vec<_>>());
        assert!(!sites.is_empty());
        for &(index, event_rows) in expected {
            let instruction = instructions[index];
            let mut statuses = vec![
                RuntimeStatus::Allocation,
                RuntimeStatus::Capacity,
                RuntimeStatus::AbiViolation,
            ];
            if instruction.kind() == VerifiedInstructionKind::StringFromUtf8 {
                statuses.push(RuntimeStatus::Utf8);
            }
            let events = event_rows
                .iter()
                .map(|&(kind, value)| {
                    let value = instructions[value].result().expect("completed result identity");
                    match kind {
                        's' => Event::StringRelease(value),
                        'v' => Event::VecStorageRelease(value),
                        _ => panic!("fixed event kind"),
                    }
                })
                .collect::<Vec<_>>();
            for status in statuses {
                let run = || {
                    witness(
                        program.runtime_abi(),
                        program.verified_ir().linear32_layouts(),
                        function,
                        instruction,
                        status,
                        ROOM,
                    )
                    .expect("constructor-only witness")
                };
                let first = run();
                assert_eq!(first.events, events);
                assert_eq!(run(), first);
            }
        }
    });
}

#[test]
fn mixed_recursive_cleanup_witness_orders_every_completed_constructor_site() {
    for outer in [false, true] {
        let (source, raw) = mixed_construction::mixed_fixture(outer);
        let vector = if outer { 2 } else { 1 };
        check_events(&source, raw, &[(0, &[]), (vector, &[('s', 0)])]);
    }
    let (source, raw) = super::super::super::nested_vec_fixture();
    // Copy Value0 has no owner: inner Vec Value1/Value2 are Place0/Place1.
    check_events(source.0, raw, &[(1, &[]), (2, &[('v', 1)]), (3, &[('v', 2), ('v', 1)])]);
    let (source, raw) = super::super::super::nested_enum_fixture();
    check_events(source.0, raw, &[(0, &[]), (1, &[('s', 0)]), (3, &[('s', 0), ('v', 1)])]);
    let (source, raw) = super::super::two_elements::two_element_fixture();
    check_events(
        &source,
        raw,
        &[
            (0, &[]),
            (1, &[('s', 0)]),
            (2, &[('s', 1), ('s', 0)]),
            (4, &[('s', 1), ('s', 0), ('v', 2)]),
        ],
    );
}

#[test]
fn mixed_recursive_cleanup_witness_bounds_work_stack_and_events_separately() {
    let (source, raw) = super::super::two_elements::two_element_fixture();
    with_program(&source, raw, |program| {
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instruction =
            function.blocks().next().expect("block").instructions().nth(4).expect("outer Vec");
        let run = |limits| {
            witness(
                program.runtime_abi(),
                program.verified_ir().linear32_layouts(),
                function,
                instruction,
                RuntimeStatus::Allocation,
                limits,
            )
        };
        let observed = run(ROOM).expect("unbounded small control");
        assert_eq!(observed.events.len(), 3);
        assert!(observed.work > 0 && observed.peak_stack > 0);
        let exact = Limits { work: observed.work, stack: observed.peak_stack, events: 3 };
        assert_eq!(run(exact), Ok(observed));
        assert_eq!(run(Limits { work: exact.work - 1, ..exact }), Err(Failure::WorkLimit));
        assert_eq!(run(Limits { stack: exact.stack - 1, ..exact }), Err(Failure::StackLimit));
        assert_eq!(run(Limits { events: 2, ..exact }), Err(Failure::EventLimit));
        assert_eq!(run(Limits { work: 0, ..ROOM }), Err(Failure::WorkLimit));
        // Failed attempts return no successful partial event list; fresh replay is unchanged.
        assert!(run(exact).is_ok());
    });
}

#[test]
fn mixed_recursive_cleanup_witness_rejects_foreign_missing_and_duplicate_provenance() {
    let (source, raw) = super::super::super::nested_vec_fixture();
    with_program(source.0, raw.clone(), |program| {
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        let instruction = instructions[3];
        let fresh = || Budget { limits: ROOM, work: 0, peak_stack: 0 };
        let context = provenance(function, instruction, &mut fresh()).expect("valid prefix");
        let root = instructions[1].result().expect("completed inner Vec");
        let layouts = program.verified_ir().linear32_layouts();
        assert_eq!(
            walk(&context, layouts, &[root, root], &mut fresh()),
            Err(Failure::DuplicateOwner)
        );
        let failed = instruction.result().expect("uncommitted outer Vec");
        assert_eq!(walk(&context, layouts, &[failed], &mut fresh()), Err(Failure::Provenance));
        with_program(source.0, raw, |foreign| {
            let foreign_function =
                foreign.modules().next().expect("module").functions().next().expect("function");
            let foreign_instructions =
                foreign_function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            assert_eq!(
                root.index(),
                foreign_instructions[1].result().expect("same numeric ID").index()
            );
            assert_eq!(
                witness(
                    program.runtime_abi(),
                    layouts,
                    function,
                    foreign_instructions[3],
                    RuntimeStatus::Allocation,
                    ROOM
                ),
                Err(Failure::Site)
            );
            assert_eq!(
                walk(
                    &context,
                    layouts,
                    &[foreign_instructions[1].result().expect("foreign branded result")],
                    &mut fresh()
                ),
                Err(Failure::Provenance)
            );
        });
        let mut missing = provenance(function, instruction, &mut fresh()).expect("second prefix");
        missing.owners[root.index() as usize] = None;
        assert_eq!(walk(&missing, layouts, &[root], &mut fresh()), Err(Failure::Provenance));
    });
}

#[test]
fn mixed_recursive_cleanup_witness_rejects_nonconstructor_prefix_and_real_partial_actions() {
    let (source, raw) = mixed_string_call_fixture();
    with_program(&source, raw, |program| {
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instruction =
            function.blocks().next().expect("block").instructions().last().expect("outer Vec");
        assert_eq!(
            witness(
                program.runtime_abi(),
                program.verified_ir().linear32_layouts(),
                function,
                instruction,
                RuntimeStatus::Allocation,
                ROOM
            ),
            Err(Failure::Prefix)
        );
    });
    // Explicitly exclude mutation/read vocabulary even when its results would be unreachable
    // from the constructor provenance tree. Actual source callers above fail the whole-prefix gate.
    for kind in [
        VerifiedInstructionKind::VecPush,
        VerifiedInstructionKind::ReplacePlace,
        VerifiedInstructionKind::InitializePlace,
        VerifiedInstructionKind::MoveFromPlace,
        VerifiedInstructionKind::DirectCall,
        VerifiedInstructionKind::StringClone,
    ] {
        assert!(!admitted(kind));
    }
    let (source, raw) = owned_pair_projected_return_snapshot("first");
    with_program(&source, raw, |program| {
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let action = function
            .blocks()
            .next()
            .expect("block")
            .terminator()
            .derived_drop_actions()
            .find(|action| action.moved_projections().len() != 0)
            .expect("real partial cleanup mask");
        // Admission must reject the actual partial action before consulting provenance.
        let unused = Provenance {
            fault: function
                .blocks()
                .next()
                .expect("block")
                .instructions()
                .next()
                .expect("instruction"),
            producers: Vec::new(),
            owners: Vec::new(),
            roots: Vec::new(),
        };
        assert_eq!(complete_root(&action, &unused), Err(Failure::Partial));
    });
}
