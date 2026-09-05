use super::generic_call_fixture::route_fixture;
use super::*;

#[test]
fn generic_route_single_string_parameter_returns_copy_with_callee_cleanup() {
    let (source, raw) = generic_call_fixture::single_string_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated single owned input");
    let program = lower(pair_input(&syntax, &sources)).expect("heterogeneous single input route");
    let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
    let caller = functions[0].blocks().next().expect("caller");
    let instructions = caller.instructions().collect::<Vec<_>>();
    assert_eq!(instructions.len(), 2);
    assert_eq!(instructions[0].kind(), VerifiedInstructionKind::StringFromUtf8);
    assert_eq!(instructions[1].kind(), VerifiedInstructionKind::DirectCall);
    assert_eq!(
        instructions[1].value_operands().collect::<Vec<_>>(),
        [instructions[0].result().expect("literal")]
    );
    assert_eq!(instructions[1].derived_drop_actions().count(), 0);
    assert_eq!(caller.terminator().derived_drop_actions().count(), 0);
    let destination = functions[1].blocks().next().expect("destination");
    let input = functions[1]
        .places()
        .find(|p| p.kind() == VerifiedPlaceKind::Parameter(0))
        .expect("String input");
    assert_eq!(
        destination.terminator().derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
        [input.id()]
    );
    assert_eq!(
        destination.instructions().next().expect("Copy result").kind(),
        VerifiedInstructionKind::I32Literal
    );
}

#[test]
fn generic_route_selects_mixed_fixed_array_local_without_owned_signature() {
    for string in [false, true] {
        let (source, raw) = route_fixture(true, string);
        assert!(raw.files[0].functions[0].parameters.is_empty());
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed array local");
        let program =
            lower(pair_input(&syntax, &sources)).expect("mixed array selects generic route");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let vector = instructions
            .iter()
            .find(|i| i.kind() == VerifiedInstructionKind::VecConstruct)
            .expect("nested Vec");
        let array = instructions
            .iter()
            .find(|i| i.kind() == VerifiedInstructionKind::FixedArrayConstruct)
            .expect("mixed FixedArray");
        assert_eq!(
            array.value_operands().collect::<Vec<_>>(),
            [vector.result().expect("Vec result")]
        );
        let owners =
            block.terminator().derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>();
        assert_eq!(owners.len(), 1, "complete local remains pending at scalar/String return");
        assert_eq!(
            function.places().find(|p| p.id() == owners[0]).expect("local owner").kind(),
            VerifiedPlaceKind::Local(0)
        );
        let result = block.terminator().value_operands().next().expect("returned value");
        let expected = if string {
            VerifiedInstructionKind::StringFromUtf8
        } else {
            VerifiedInstructionKind::I32Literal
        };
        assert!(instructions.iter().any(|i| i.result() == Some(result) && i.kind() == expected));
    }
}

#[test]
fn generic_route_selects_inline_mixed_call_without_owned_parameters_or_locals() {
    for string in [false, true] {
        let (source, raw) = route_fixture(false, string);
        assert!(raw.files[0].functions[0].parameters.is_empty());
        assert_eq!(raw.files[0].functions[0].body.statements.len(), 1);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated inline generic call");
        let program =
            lower(pair_input(&syntax, &sources)).expect("callee shape selects generic route");
        let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
        let block = functions[0].blocks().next().expect("caller block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions
                .iter()
                .filter(|i| i.kind() == VerifiedInstructionKind::VecConstruct)
                .count(),
            2
        );
        let call = instructions
            .iter()
            .find(|i| i.kind() == VerifiedInstructionKind::DirectCall)
            .expect("generic call");
        assert_eq!(call.call_arguments().count(), 2);
        assert_eq!(
            call.derived_drop_actions().count(),
            0,
            "both completed owned inputs transferred"
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            [call.result().expect("exact result")]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        let destination = functions[1].blocks().next().expect("destination block");
        assert_eq!(
            destination.terminator().derived_drop_actions().count(),
            2,
            "callee retains both unused owned inputs until return"
        );
    }
}
