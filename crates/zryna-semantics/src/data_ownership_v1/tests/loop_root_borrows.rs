use super::*;

#[test]
#[allow(clippy::too_many_lines)]
fn loop_root_borrows_discharge_before_the_canonical_backedge() {
    let sources = sources_for(LOOP_ROOT_BORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(LOOP_ROOT_BORROW_JSON).expect("loop borrow snapshot"),
        &sources,
    )
    .expect("source-faithful loop borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("loop borrow lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert!(blocks.iter().all(|block| block.parameters().next().is_none()));
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    assert_eq!(blocks[0].terminator().edges().next().expect("preheader").target().index(), 1);
    let (body, exit) = blocks[1].terminator().branch_edges().expect("loop branch");
    assert_eq!((body.target().index(), exit.target().index()), (2, 3));
    assert_eq!(blocks[2].terminator().edges().next().expect("backedge").target().index(), 1);
    assert!(
        blocks
            .iter()
            .all(|block| { block.terminator().edges().all(|edge| edge.arguments().count() == 0) })
    );
    let body = blocks[2].instructions().collect::<Vec<_>>();
    assert_eq!(body.first().expect("begin").kind(), VerifiedInstructionKind::BeginBorrow);
    assert_eq!(body.last().expect("lexical end").kind(), VerifiedInstructionKind::EndBorrow);
    assert_eq!(
        body.first()
            .and_then(|instruction| instruction.borrow())
            .map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
        body.last()
            .and_then(|instruction| instruction.borrow())
            .map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
        "every structural visit reuses the same static authority identity",
    );
    assert_eq!(
        blocks[1].instructions().next().expect("header condition").kind(),
        VerifiedInstructionKind::CopyFromPlace
    );
    assert_eq!(
        blocks[3].instructions().next().expect("exit read").kind(),
        VerifiedInstructionKind::CopyFromPlace
    );
    let lowering_trace = |function: VerifiedFunction<'_>| {
        function
            .blocks()
            .map(|block| {
                (
                    block.id().index(),
                    block
                        .instructions()
                        .map(|instruction| {
                            (
                                instruction.kind(),
                                instruction
                                    .borrow()
                                    .map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
                            )
                        })
                        .collect::<Vec<_>>(),
                    block.terminator().kind(),
                    block
                        .terminator()
                        .edges()
                        .map(|edge| (edge.target().index(), edge.arguments().count()))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    let first_lowering = lowering_trace(function);
    let replayed = lower(pair_input(&syntax, &sources)).expect("deterministic loop replay");
    let replayed_function = replayed
        .modules()
        .next()
        .expect("replayed module")
        .functions()
        .next()
        .expect("replayed function");
    assert_eq!(first_lowering, lowering_trace(replayed_function));
    let walk = |iterations: usize| {
        let mut remaining = iterations;
        let mut current = 0_usize;
        let mut trace = Vec::new();
        let mut body_authorities = Vec::new();
        loop {
            trace.push(current);
            let block = blocks[current];
            if current == 2 {
                body_authorities.push(
                    block
                        .instructions()
                        .filter(|instruction| {
                            matches!(
                                instruction.kind(),
                                VerifiedInstructionKind::BeginBorrow
                                    | VerifiedInstructionKind::EndBorrow
                            )
                        })
                        .map(|instruction| instruction.borrow().expect("body authority").index())
                        .collect::<Vec<_>>(),
                );
            }
            match block.terminator().kind() {
                VerifiedTerminatorKind::Jump => {
                    current = usize::try_from(
                        block.terminator().edges().next().expect("jump edge").target().index(),
                    )
                    .expect("dense block");
                }
                VerifiedTerminatorKind::Branch => {
                    let (body, exit) = block.terminator().branch_edges().expect("branch edges");
                    if remaining == 0 {
                        current = usize::try_from(exit.target().index()).expect("dense exit");
                    } else {
                        remaining -= 1;
                        current = usize::try_from(body.target().index()).expect("dense body");
                    }
                }
                VerifiedTerminatorKind::Return => break,
                kind => panic!("unexpected loop terminator {kind:?}"),
            }
        }
        assert!(body_authorities.iter().all(|ids| ids == &vec![0, 0]));
        (trace, body_authorities.len())
    };
    assert_eq!(walk(0), (vec![0, 1, 3], 0));
    assert_eq!(walk(1), (vec![0, 1, 2, 1, 3], 1));
    assert_eq!(walk(2), (vec![0, 1, 2, 1, 2, 1, 3], 2));
}

#[test]
fn loop_shared_root_borrow_keeps_owner_copy_reads_inside_the_body() {
    let sources = sources_for(LOOP_SHARED_ROOT_BORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(LOOP_SHARED_ROOT_BORROW_JSON).expect("shared loop snapshot"),
        &sources,
    )
    .expect("source-faithful shared loop v4");
    let program = lower(pair_input(&syntax, &sources)).expect("shared loop lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let body = function.blocks().nth(2).expect("loop body").instructions().collect::<Vec<_>>();
    assert!(body.iter().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::BeginBorrow
            && instruction.borrow_access() == Some(VerifiedBorrowAccess::Shared)
    }));
    assert!(
        body.iter().any(|instruction| instruction.kind() == VerifiedInstructionKind::BorrowRead)
    );
    assert!(
        body.iter().any(|instruction| instruction.kind() == VerifiedInstructionKind::CopyFromPlace)
    );
    assert_eq!(body.last().expect("reverse end").kind(), VerifiedInstructionKind::EndBorrow);
}

#[test]
fn loop_root_borrow_exclusions_are_source_faithful_ordered_and_stable() {
    let sources = sources_for(LOOP_ROOT_BORROW_EXCLUSIONS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(LOOP_ROOT_BORROW_EXCLUSIONS_JSON).expect("loop exclusions snapshot"),
        &sources,
    )
    .expect("source-faithful loop exclusions v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("excluded loop shapes");
    let trace = diagnostics
        .iter()
        .map(|item| {
            let primary = item.primary_span().expect("source diagnostic");
            (item.code().to_owned(), item.message().to_owned(), (primary.start(), primary.end()))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        trace,
        vec![
            ("ZRYNA-M3017".to_owned(), "shared-root borrowing requires one private parameter-free function".to_owned(), (0, 189)),
            ("ZRYNA-M3017".to_owned(), "shared-root borrowing requires one private parameter-free function".to_owned(), (191, 387)),
            ("ZRYNA-M3017".to_owned(), "root-borrow blocks admit only const aliases, const Copy reads, and BorrowMut writes".to_owned(), (467, 566)),
            ("ZRYNA-M3017".to_owned(), "loop borrowing requires the exact bool root as its condition".to_owned(), (663, 668)),
            ("ZRYNA-M3017".to_owned(), "loop borrowing requires the exact bool root as its condition".to_owned(), (854, 858)),
            ("ZRYNA-M3017".to_owned(), "loop borrowing requires one literal-initialized bool root".to_owned(), (1004, 1024)),
            ("ZRYNA-M3017".to_owned(), "root-borrow blocks admit only const aliases, const Copy reads, and BorrowMut writes".to_owned(), (1229, 1341)),
            ("ZRYNA-M3017".to_owned(), "shared-root borrowing requires one root local, one lexical block, and one final return".to_owned(), (1395, 1649)),
            ("ZRYNA-M3017".to_owned(), "root-borrow blocks admit only const aliases, const Copy reads, and BorrowMut writes".to_owned(), (1817, 1833)),
            ("ZRYNA-M3017".to_owned(), "shared-borrow Copy locals must read one active alias".to_owned(), (2063, 2082)),
            ("ZRYNA-M3017".to_owned(), "shared-root borrowing requires one root local, one lexical block, and one final return".to_owned(), (2138, 2320)),
        ]
    );
    let repeated = lower(pair_input(&syntax, &sources)).expect_err("stable exclusions");
    assert_eq!(
        trace,
        repeated
            .iter()
            .map(|item| {
                let primary = item.primary_span().expect("source diagnostic");
                (
                    item.code().to_owned(),
                    item.message().to_owned(),
                    (primary.start(), primary.end()),
                )
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn loop_root_borrow_resources_are_static_and_saturating() {
    let resources = loop_root_borrow_resources(2, 3, 5);
    assert_eq!((resources.values, resources.places), (11, 1));
    assert_eq!(resources.transitions, 21);
    assert_eq!((resources.blocks, resources.edges), (4, 4));
    assert_eq!((resources.active_peak, resources.cleanup_plans), (2, 1));
    let saturated = loop_root_borrow_resources(usize::MAX, usize::MAX, usize::MAX);
    assert_eq!(saturated.values, usize::MAX);
    assert_eq!(saturated.transitions, usize::MAX);
    let exact_values =
        loop_root_borrow_resources(0, zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - 3, 0);
    let extra_values =
        loop_root_borrow_resources(0, zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - 2, 0);
    assert_eq!(exact_values.values, zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION);
    assert_eq!(root_borrow_resource_violation(exact_values), None);
    assert_eq!(root_borrow_resource_violation(extra_values), Some(RootBorrowBudgetLimit::Values));
    let exact_active = loop_root_borrow_resources(
        zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION,
        0,
        0,
    );
    let extra_active = loop_root_borrow_resources(
        zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION + 1,
        0,
        0,
    );
    assert_eq!(root_borrow_resource_violation(exact_active), None);
    assert_eq!(
        root_borrow_resource_violation(extra_active),
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    );
    for (limit, exact, extra) in [
        (
            RootBorrowBudgetLimit::Values,
            RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
                ..resources
            },
            RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION + 1,
                ..resources
            },
        ),
        (
            RootBorrowBudgetLimit::Transitions,
            RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                ..resources
            },
            RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    + 1,
                ..resources
            },
        ),
        (
            RootBorrowBudgetLimit::ActiveBorrows,
            RootBorrowResources {
                active_peak: zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION,
                ..resources
            },
            RootBorrowResources {
                active_peak: zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION + 1,
                ..resources
            },
        ),
    ] {
        assert_eq!(root_borrow_resource_violation(exact), None);
        assert_eq!(root_borrow_resource_violation(extra), Some(limit));
    }
}
