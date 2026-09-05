use super::generic_static_fixture::{Case, Shape, fixture};
use super::*;

#[test]
fn generic_copy_projection_reads_have_no_owned_root_or_copy_drop_obligation() {
    for shape in [Shape::CopyStruct, Shape::CopyArray, Shape::CopyNestedArray] {
        let (source, raw) = fixture(shape, Case::Move);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated Copy subtree read");
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{shape:?}: {errors:?}\n{source}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let returned = block.terminator().value_operands().next().expect("Copy result");
            let read = block
                .instructions()
                .find(|i| i.result() == Some(returned))
                .expect("projection read");
            assert_eq!(read.kind(), VerifiedInstructionKind::CopyFromPlace);
            let projection = read.place_operands().next().expect("static projection");
            let place = function.places().find(|p| p.id() == projection).expect("place");
            assert!(place.is_copy());
            assert!(matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { .. }
                    | VerifiedPlaceKind::FixedArrayConstant { .. }
            ));
            assert!(!function.places().any(|p| p.kind() == VerifiedPlaceKind::Temporary(returned)));
            assert!(block.instructions().all(|i| !matches!(
                i.kind(),
                VerifiedInstructionKind::GenericMoveFromPlace
                    | VerifiedInstructionKind::GenericReplacePlace
            )));
            let drops = block.terminator().derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(drops.len(), 1, "only the unrelated String parameter is owned");
            let owned =
                function.places().find(|p| p.id() == drops[0].root()).expect("owned parameter");
            assert_eq!(owned.kind(), VerifiedPlaceKind::Parameter(1));
            assert!(!owned.is_copy());
        }
    }
}

#[test]
fn generic_copy_projection_write_prepares_rhs_before_exclusive_copy_commit() {
    for shape in [Shape::CopyStruct, Shape::CopyArray, Shape::CopyNestedArray] {
        let (source, raw) = fixture(shape, Case::CopyReplace);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated Copy subtree write");
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{shape:?}: {errors:?}\n{source}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let begin = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                .expect("exclusive Copy projection access");
            let write = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::BorrowWrite)
                .expect("Copy write");
            let end = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("lexical end");
            assert_eq!(write, begin + 1);
            assert_eq!(instructions[begin].borrow_access(), Some(VerifiedBorrowAccess::Exclusive));
            assert_eq!(end, write + 1);
            let value = instructions[write].value_operands().next().expect("prepared Copy RHS");
            assert!(
                instructions[..begin].iter().any(|i| i.result() == Some(value)
                    && i.kind() == VerifiedInstructionKind::CopyFromPlace)
            );
            assert!(!function.places().any(|p| p.kind() == VerifiedPlaceKind::Temporary(value)));
            assert!(instructions.iter().all(|i| i.derived_drop_actions().len() == 0));
            assert!(instructions.iter().all(|i| !matches!(
                i.kind(),
                VerifiedInstructionKind::ReplacePlace
                    | VerifiedInstructionKind::GenericReplacePlace
                    | VerifiedInstructionKind::BorrowReplace
            )));
            let drops = block.terminator().derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(drops.len(), 1);
            assert_eq!(
                function
                    .places()
                    .find(|p| p.id() == drops[0].root())
                    .expect("owned parameter")
                    .kind(),
                VerifiedPlaceKind::Parameter(2)
            );
        }
    }
}
