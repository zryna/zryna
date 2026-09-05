use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::generic_vec_fixture::Element;
use super::*;

#[path = "explicit_indexed_effect_fixture.rs"]
mod effects;

#[test]
fn explicit_indexed_empty_vec_preserves_bounds_and_complete_cleanup() {
    let (source, raw) = effects::fixture(effects::Case::Empty);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated empty Vec borrow");
    let program = lower(pair_input(&syntax, &sources)).expect("empty Vec uses runtime bounds");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let construction = instructions
        .iter()
        .find(|i| i.kind() == VerifiedInstructionKind::VecConstruct)
        .expect("actual empty constructor");
    assert_eq!(construction.value_operands().count(), 0);
    let begin = instructions.iter().find(|i| i.indexed_borrow().is_some()).expect("begin");
    let authority = begin.indexed_borrow().expect("bounds");
    assert_eq!(authority.trap_identity(), VerifiedTrapIdentity::BoundsV1);
    assert!(
        instructions
            .iter()
            .any(|i| i.result() == Some(authority.index()) && i.i32_literal() == Some(0))
    );
    assert!(begin.derived_drop_actions().any(|drop| drop.root() == authority.container()));
    assert!(begin.derived_drop_actions().all(|drop| drop.moved_projections().len() == 0));
    assert_eq!(begin.failure_ended_borrows().len(), 0);
}

#[test]
fn explicit_indexed_effectful_index_and_rhs_calls_lower_once_on_opposite_sides_of_bounds() {
    let (source, raw) = effects::fixture(effects::Case::Effects);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated effectful index/RHS");
    let program = lower(pair_input(&syntax, &sources)).expect("ordered index/RHS calls");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let calls =
        instructions.iter().enumerate().filter(|(_, i)| i.callee().is_some()).collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1.callee().expect("indexer").declaration(), 1);
    assert_eq!(calls[1].1.callee().expect("fresh").declaration(), 2);
    let begin_at = instructions.iter().position(|i| i.indexed_borrow().is_some()).expect("begin");
    let begin = instructions[begin_at].indexed_borrow().expect("bounds");
    let write_at = instructions
        .iter()
        .position(|i| i.kind() == VerifiedInstructionKind::BorrowWrite)
        .expect("write");
    assert!(calls[0].0 < begin_at && begin_at < calls[1].0 && calls[1].0 < write_at);
    assert_eq!(calls[0].1.result(), Some(begin.index()));
    assert_eq!(
        instructions[write_at].value_operands().collect::<Vec<_>>(),
        [calls[1].1.result().expect("prepared RHS")]
    );
    assert_eq!(calls[0].1.failure_ended_borrows().len(), 0);
    assert_eq!(calls[1].1.failure_ended_borrows().collect::<Vec<_>>(), [begin.borrow()]);
    assert!(calls[1].1.derived_drop_actions().any(|drop| drop.root() == begin.container()));
}

#[test]
fn explicit_indexed_vec_growth_requires_lexical_end_and_then_retains_growth_cleanup() {
    for case in [effects::Case::GrowthInside, effects::Case::GrowthAfter] {
        let (source, raw) = effects::fixture(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated growth position");
        if matches!(case, effects::Case::GrowthInside) {
            let first = lower(pair_input(&syntax, &sources)).expect_err("borrow excludes growth");
            assert_eq!(first.len(), 1);
            assert_eq!(first[0].code(), "ZRYNA-M3014");
            assert_eq!(
                first,
                lower(pair_input(&syntax, &sources)).expect_err("deterministic rejection")
            );
        } else {
            let program =
                lower(pair_input(&syntax, &sources)).expect("lexical end restores growth");
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let end = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("end");
            let push = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::VecPush)
                .expect("growth");
            assert!(end < push);
            let begin = instructions.iter().find_map(|i| i.indexed_borrow()).expect("container");
            assert_eq!(instructions[push].failure_ended_borrows().len(), 0);
            assert!(
                instructions[push]
                    .derived_drop_actions()
                    .any(|drop| drop.root() == begin.container())
            );
            assert!(
                instructions[push]
                    .derived_drop_actions()
                    .all(|drop| drop.moved_projections().len() == 0)
            );
        }
    }
}

#[test]
fn explicit_indexed_bool_reads_writes_and_bounds_restore_container_access() {
    for container in [Container::Array(0), Container::Array(2), Container::Vec] {
        for index in [None, Some(-1), Some(i32::MAX)] {
            for (exclusive, action) in
                [(false, Action::Read), (true, Action::Read), (true, Action::Replace)]
            {
                let (source, raw) = fixture(container, &Element::Bool, exclusive, action, index);
                let sources = sources_for(&source);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated Bool indexed source");
                for _ in 0..2 {
                    let program =
                        lower(pair_input(&syntax, &sources)).expect("exact Bool element access");
                    let function = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("function");
                    let block = function.blocks().next().expect("block");
                    let instructions = block.instructions().collect::<Vec<_>>();
                    let begin_at = instructions
                        .iter()
                        .position(|i| i.indexed_borrow().is_some())
                        .expect("begin");
                    let begin = instructions[begin_at].indexed_borrow().expect("bounds");
                    assert_eq!(begin.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                    let access = if matches!(action, Action::Replace) {
                        VerifiedInstructionKind::BorrowWrite
                    } else {
                        VerifiedInstructionKind::BorrowRead
                    };
                    let access_at =
                        instructions.iter().position(|i| i.kind() == access).expect("Copy access");
                    let end_at = instructions
                        .iter()
                        .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .expect("lexical end");
                    assert!(begin_at < access_at && access_at < end_at);
                    assert_eq!(instructions[access_at].borrow(), Some(begin.borrow()));
                    assert!(
                        instructions[end_at + 1..]
                            .iter()
                            .any(|i| i.place_operands().any(|place| place == begin.container()))
                    );
                    assert!(
                        instructions.iter().all(
                            |i| i.borrow_replacement().is_none() && i.generic_clone().is_none()
                        )
                    );
                    assert!(
                        block
                            .terminator()
                            .derived_drop_actions()
                            .all(|drop| drop.root() != begin.container())
                    );
                }
            }
        }
    }
}
