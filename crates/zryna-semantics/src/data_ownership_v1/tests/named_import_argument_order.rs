use super::named_import_calls::imported_fallible_arguments_fixture;
use super::*;
use zryna_ir::data_ownership_v1::VerifiedCallArgument;

#[test]
fn named_import_mixed_argument_producers_are_ordered_and_later_failure_retains_earlier_owners() {
    let (sources, raw) = imported_fallible_arguments_fixture();
    let syntax = verify_snapshot(raw, &sources).expect("authenticated fallible arguments");
    let entry = sources.file_id(&NormalizedSourcePath::new("src/main.zry").unwrap()).unwrap();
    let program = lower(SemanticInput::try_new(&syntax, &sources, entry).unwrap())
        .unwrap_or_else(|errors| panic!("{errors:?}"));
    let caller = program.modules().nth(1).unwrap().functions().next().unwrap();
    let block = caller.blocks().next().unwrap();
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::DirectCall,
        ],
        "distinguishable argument producers retain source order"
    );
    assert_eq!(
        instructions[3].call_arguments().collect::<Vec<_>>(),
        [
            VerifiedCallArgument::Value(instructions[0].result().unwrap()),
            VerifiedCallArgument::Value(instructions[1].result().unwrap()),
            VerifiedCallArgument::Value(instructions[2].result().unwrap()),
        ]
    );
    assert_eq!(
        instructions[2]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [4, 3, 2, 0],
        "later producer failure drops the earlier prepared String before caller survivors"
    );
    assert_eq!(
        instructions[3]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [3, 2, 0],
        "call commit transfers both prepared String arguments"
    );
}
