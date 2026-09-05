use super::generic_call_fixture::{Case, fixture};
use super::generic_vec_fixture::Element;
use super::*;
use zryna_ir::data_ownership_v1::VerifiedCallArgument;

#[test]
fn generic_calls_transfer_multiple_owned_and_copy_arguments_in_source_order() {
    for element in [Element::Struct, Element::Enum, Element::String, Element::Vec] {
        for case in [Case::Direct, Case::Nested] {
            let (source, raw) = fixture(&element, case);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated generic call");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{element:?} {case:?}: {errors:?}"));
            let functions =
                program.modules().next().expect("module").functions().collect::<Vec<_>>();
            assert_eq!(functions.len(), 2);
            let caller = functions[0];
            let block = caller.blocks().next().expect("caller block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let call_at = instructions.iter().position(|i| i.callee().is_some()).expect("call");
            let call = instructions[call_at];
            assert_eq!(call.callee().expect("callee").declaration(), 1);
            let arguments = call.call_arguments().collect::<Vec<_>>();
            assert_eq!(arguments.len(), 3);
            for (ordinal, argument) in arguments.iter().enumerate() {
                let VerifiedCallArgument::Value(value) = argument else {
                    panic!("ordinary value parameter")
                };
                let position =
                    instructions.iter().position(|i| i.result() == Some(*value)).expect("argument");
                assert!(position < call_at);
                let read = instructions[position];
                assert_eq!(
                    read.kind(),
                    if ordinal == 1 {
                        VerifiedInstructionKind::CopyFromPlace
                    } else {
                        VerifiedInstructionKind::MoveFromPlace
                    }
                );
                let source = read.place_operands().next().expect("argument source");
                assert_eq!(
                    caller.places().find(|p| p.id() == source).expect("parameter").kind(),
                    VerifiedPlaceKind::Parameter(u32::try_from(ordinal).expect("ordinal"))
                );
            }
            assert!(arguments.windows(2).all(|pair| {
                let [VerifiedCallArgument::Value(left), VerifiedCallArgument::Value(right)] = pair
                else {
                    panic!("value arguments")
                };
                left.index() < right.index()
            }));
            let keep = caller
                .places()
                .find(|p| p.kind() == VerifiedPlaceKind::Parameter(3))
                .expect("retained owner");
            assert_eq!(
                call.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
                [keep.id()]
            );
            let cleanup = caller
                .cleanup_plans()
                .find(|p| Some(p.id()) == call.cleanup())
                .expect("call failure");
            assert_eq!(cleanup.site().role(), VerifiedCleanupRole::CallTrap);
            let destination_block = functions[1].blocks().next().expect("callee block");
            let discarded = functions[1]
                .places()
                .find(|p| p.kind() == VerifiedPlaceKind::Parameter(2))
                .expect("second owner");
            assert_eq!(
                destination_block
                    .terminator()
                    .derived_drop_actions()
                    .map(|drop| drop.root())
                    .collect::<Vec<_>>(),
                [discarded.id()]
            );
            if matches!(case, Case::Nested) {
                let constructor = instructions
                    .iter()
                    .find(|i| i.kind() == VerifiedInstructionKind::VecConstruct)
                    .expect("outer mixed constructor");
                assert_eq!(
                    constructor.value_operands().collect::<Vec<_>>(),
                    [call.result().expect("exact owned call result")]
                );
            }
            assert_eq!(
                block
                    .terminator()
                    .derived_drop_actions()
                    .map(|drop| drop.root())
                    .collect::<Vec<_>>(),
                [keep.id()]
            );
        }
    }
}

#[test]
fn generic_calls_reject_wrong_argument_type_and_repeated_owner_deterministically() {
    for case in [Case::WrongType, Case::RepeatedOwner] {
        let (source, raw) = fixture(&Element::Struct, case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated invalid generic call");
        let expected = lower(pair_input(&syntax, &sources)).expect_err("invalid call rejected");
        assert_eq!(expected.len(), 1);
        assert_eq!(
            expected[0].code,
            if matches!(case, Case::WrongType) { "ZRYNA-M3016" } else { "ZRYNA-M3014" }
        );
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("repeat rejection"),
                expected
            );
        }
    }
}
