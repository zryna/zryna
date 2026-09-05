use super::generic_vec_fixture::{Element, Operation, fixture_array};
use super::*;
use zryna_ir::data_ownership_v1::{VerifiedGenericCloneSource, VerifiedTrapIdentity};

#[test]
fn ordinary_array_source_checked_copy_reads_include_negative_upper_and_zero_length() {
    for element in [Element::I32, Element::Bool] {
        for (length, index) in [(2, None), (2, Some(-1)), (2, Some(2)), (0, Some(0))] {
            let (source, raw) = fixture_array(&element, Operation::Read, index, length);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated checked array");
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let instructions = block.instructions().collect::<Vec<_>>();
                let begin = instructions.iter().find_map(|i| i.indexed_borrow()).expect("bounds");
                assert_eq!(begin.array_length(), Some(u64::from(length)));
                assert_eq!(begin.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                assert_eq!(begin.access(), VerifiedBorrowAccess::Shared);
                assert_eq!(
                    instructions
                        .iter()
                        .filter(|i| i.kind() == VerifiedInstructionKind::BorrowRead)
                        .count(),
                    1
                );
                assert_eq!(
                    instructions
                        .iter()
                        .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .count(),
                    1
                );
                assert_eq!(block.terminator().derived_drop_actions().count(), 0);
            }
        }
    }
}

#[test]
fn ordinary_array_source_owned_clone_retains_container_and_common_recursive_frontier() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        for (length, index) in [(2, None), (2, Some(-1)), (2, Some(2)), (0, Some(0))] {
            let (source, raw) = fixture_array(&element, Operation::Clone, index, length);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated owned array");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let begin_at =
                instructions.iter().position(|i| i.indexed_borrow().is_some()).expect("bounds");
            let clone_at =
                instructions.iter().position(|i| i.generic_clone().is_some()).expect("clone");
            assert!(begin_at < clone_at);
            let begin = instructions[begin_at].indexed_borrow().expect("indexed authority");
            let clone = instructions[clone_at].generic_clone().expect("canonical clone");
            assert_eq!(clone.source(), VerifiedGenericCloneSource::Borrow(begin.borrow()));
            assert_eq!(clone.ty(), begin.referent());
            assert_eq!(begin.array_length(), Some(u64::from(length)));
            assert!(
                instructions[clone_at]
                    .generic_clone_prefix_failure_drop_actions()
                    .any(|drop| drop.root() == clone.destination())
            );
            // A zero-length array has no owned elements and may itself be Copy.
            if length != 0 {
                assert!(
                    instructions[clone_at]
                        .derived_drop_actions()
                        .any(|drop| drop.root() == begin.container())
                );
                assert!(
                    block
                        .terminator()
                        .derived_drop_actions()
                        .any(|drop| drop.root() == begin.container())
                );
            }
            assert_eq!(
                instructions[clone_at]
                    .failure_ended_borrows()
                    .map(|b| b.index())
                    .collect::<Vec<_>>(),
                [begin.borrow().index()]
            );
        }
    }
}

#[test]
fn ordinary_array_source_replacement_checks_bounds_before_preparing_rhs() {
    for element in [
        Element::I32,
        Element::String,
        Element::Struct,
        Element::Enum,
        Element::Array,
        Element::Vec,
    ] {
        let operation = if matches!(element, Element::I32) {
            Operation::Replace
        } else {
            Operation::ReplaceClone
        };
        let (source, raw) = fixture_array(&element, operation, None, 2);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated array replacement");
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        let begin_at =
            instructions.iter().position(|i| i.indexed_borrow().is_some()).expect("bounds");
        let begin = instructions[begin_at].indexed_borrow().expect("exclusive element");
        assert_eq!(begin.access(), VerifiedBorrowAccess::Exclusive);
        let commit_at = instructions
            .iter()
            .position(|i| {
                matches!(
                    i.kind(),
                    VerifiedInstructionKind::BorrowReplace | VerifiedInstructionKind::BorrowWrite
                )
            })
            .expect("commit");
        assert!(begin_at < commit_at);
        if !matches!(element, Element::I32) {
            let failures = instructions[begin_at + 1..commit_at]
                .iter()
                .filter(|instruction| instruction.cleanup().is_some())
                .collect::<Vec<_>>();
            assert!(!failures.is_empty());
            assert!(failures.iter().all(|instruction| {
                instruction.derived_drop_actions().any(|drop| drop.root() == begin.container())
            }));
        }
        assert_eq!(instructions[commit_at + 1].kind(), VerifiedInstructionKind::EndBorrow);
    }
}

#[test]
fn ordinary_array_source_owned_dynamic_read_cannot_move_an_element() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let (source, raw) = fixture_array(&element, Operation::Read, None, 2);
        let at = raw.files[0].functions[0].body.expressions.last().expect("index").span;
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected array read");
        let check = || lower(pair_input(&syntax, &sources)).expect_err("no dynamic element move");
        let errors = check();
        assert_eq!(
            errors,
            vec![zryna_diagnostics::Diagnostic::error_at(
                "ZRYNA-M3013",
                span(&sources, at),
                "array observation requires its exact Copy element or an explicit owned clone",
                "read the exact Copy element type or explicitly clone the owned indexed element",
            )]
        );
        assert_eq!(errors, check());
    }
}
