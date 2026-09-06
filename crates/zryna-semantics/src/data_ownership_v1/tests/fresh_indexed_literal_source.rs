use super::*;
use zryna_ir::data_ownership_v1::{BorrowIdentity, VerifiedGenericCloneSource};

#[test]
fn fresh_indexed_literal_source_rejects_non_lvalue_replacement_and_recovers() {
    let (text, raw) = structured_owned_fixture::fresh_indexed_assignment_fixture();
    let sources = sources_for(&text);
    let reject =
        || verify_snapshot(raw.clone(), &sources).expect_err("fresh result is not an lvalue");
    let errors = reject();
    assert_eq!(errors, reject());
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-Y4002");
    assert_eq!(errors[0].message(), "assignment target is not syntactically a place");
    assert_eq!(errors[0].guidance(), "return source-faithful canonical protocol-v4 syntax");
    assert!(errors[0].primary_span().is_none(), "syntax rejection identifies the workspace path");
    let (text, raw) = structured_owned_fixture::fresh_indexed_literal_fixture(true, 0);
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("recovery syntax");
    lower(pair_input(&syntax, &sources)).expect("valid recovery");
}

#[test]
fn fresh_indexed_literal_source_preserves_static_prefix_order_and_storage() {
    for owned in [false, true] {
        for shape in 0..3 {
            let (text, raw) = structured_owned_fixture::fresh_indexed_literal_fixture(owned, shape);
            let sources = sources_for(&text);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated literal prefix");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{text}\n{errors:?}"));
            let replay = lower(pair_input(&syntax, &sources)).expect("literal prefix replay");
            assert_eq!(
                format!("{:?}", program.verified_ir()),
                format!("{:?}", replay.verified_ir())
            );
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let blocks = function.blocks().collect::<Vec<_>>();
            assert_eq!(blocks.len(), 4);
            assert_eq!(
                blocks
                    .iter()
                    .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::EnumMatch)
                    .count(),
                1
            );
            let continuation = blocks.last().expect("joined base");
            let instructions = continuation.instructions().collect::<Vec<_>>();
            let begins = instructions
                .iter()
                .filter_map(|instruction| instruction.indexed_borrow())
                .collect::<Vec<_>>();
            assert_eq!(begins.len(), 1);
            let begin = begins[0];
            let projects = instructions
                .iter()
                .filter_map(|instruction| instruction.indexed_projection())
                .collect::<Vec<_>>();
            assert_eq!(projects.len(), usize::from(shape != 0));
            let final_borrow = projects.first().map_or(begin.borrow(), |project| project.borrow());
            if let Some(project) = projects.first() {
                assert_eq!(project.parent(), begin.borrow());
                assert_eq!(project.container(), begin.container());
            }
            assert_storage(&instructions, continuation, begin.container(), owned);
            let finish = instructions
                .iter()
                .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("final end");
            assert_eq!(
                instructions
                    .iter()
                    .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
                    .count(),
                1
            );
            assert_eq!(instructions[finish].borrow(), Some(final_borrow));
            assert_eq!(instructions[finish - 1].borrow(), Some(final_borrow));
            if owned {
                let clone =
                    instructions[finish - 1].generic_clone().expect("owned observation clone");
                assert_eq!(clone.source(), VerifiedGenericCloneSource::Borrow(final_borrow));
                assert_ne!(clone.destination(), begin.container());
                assert_eq!(
                    instructions[finish - 1].failure_ended_borrows().collect::<Vec<_>>(),
                    [final_borrow]
                );
                assert_eq!(instructions[finish + 1].kind(), VerifiedInstructionKind::DropPlace);
                assert_eq!(
                    instructions[finish + 1].place_operands().collect::<Vec<_>>(),
                    [begin.container()]
                );
                assert_eq!(
                    continuation.terminator().value_operands().collect::<Vec<_>>(),
                    [clone.result()]
                );
            } else {
                assert_eq!(instructions[finish - 1].kind(), VerifiedInstructionKind::BorrowRead);
            }
            assert_index_order(&instructions, shape, begin.borrow());
        }
    }
}

fn assert_storage(
    instructions: &[zryna_ir::data_ownership_v1::VerifiedInstruction<'_>],
    continuation: &zryna_ir::data_ownership_v1::VerifiedBlock<'_>,
    container: zryna_ir::data_ownership_v1::PlaceIdentity,
    owned: bool,
) {
    let initialized = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::InitializePlace)
        .collect::<Vec<_>>();
    assert_eq!(initialized.len(), usize::from(!owned));
    if !owned {
        let storage = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::InitializePlace)
            .expect("Copy storage");
        let begin = instructions
            .iter()
            .position(|instruction| instruction.indexed_borrow().is_some())
            .expect("first access");
        assert!(storage < begin);
        assert_eq!(
            initialized[0].value_operands().collect::<Vec<_>>(),
            [continuation.parameters().next().expect("joined Copy base").id()]
        );
        assert_eq!(initialized[0].place_operands().collect::<Vec<_>>(), [container]);
    }
    for instruction in instructions {
        if instruction.indexed_borrow().is_some()
            || instruction.indexed_projection().is_some()
            || instruction.kind() == VerifiedInstructionKind::DirectCall
            || instruction.generic_clone().is_some()
        {
            assert_eq!(
                instruction
                    .derived_drop_actions()
                    .filter(|action| action.root() == container)
                    .count(),
                usize::from(owned)
            );
        }
    }
    assert_eq!(
        instructions
            .iter()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace
                && instruction.place_operands().any(|place| place == container))
            .count(),
        usize::from(owned)
    );
    assert!(
        !continuation.terminator().derived_drop_actions().any(|action| action.root() == container)
    );
}

fn assert_index_order(
    instructions: &[zryna_ir::data_ownership_v1::VerifiedInstruction<'_>],
    shape: usize,
    first: BorrowIdentity,
) {
    let begin = instructions
        .iter()
        .position(|instruction| instruction.indexed_borrow().is_some())
        .expect("first checked index");
    let access = instructions[begin].indexed_borrow().expect("begin");
    let literal = instructions
        .iter()
        .position(|instruction| instruction.result() == Some(access.index()))
        .expect("once-evaluated first literal");
    assert!(literal < begin);
    assert_eq!(instructions[literal].kind(), VerifiedInstructionKind::I32Literal);
    assert_eq!(
        instructions
            .iter()
            .filter(|instruction| instruction.result() == Some(access.index()))
            .count(),
        1
    );
    let calls = instructions
        .iter()
        .enumerate()
        .filter(|(_, instruction)| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), usize::from(shape == 2));
    if shape != 0 {
        let project = instructions
            .iter()
            .position(|instruction| instruction.indexed_projection().is_some())
            .expect("second index");
        assert!(begin < project);
        assert_eq!(instructions[project].failure_ended_borrows().collect::<Vec<_>>(), [first]);
        if shape == 2 {
            assert!(begin < calls[0].0 && calls[0].0 < project);
            assert_eq!(calls[0].1.failure_ended_borrows().collect::<Vec<_>>(), [first]);
            assert_eq!(
                calls[0].1.result(),
                Some(instructions[project].indexed_projection().expect("projection").index())
            );
        }
    }
}
