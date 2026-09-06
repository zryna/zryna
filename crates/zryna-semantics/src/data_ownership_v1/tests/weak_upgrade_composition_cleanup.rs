use super::*;
use zryna_ir::data_ownership_v1::{
    PlaceIdentity, VerifiedBlock, VerifiedFunction, VerifiedPlaceKind,
};

pub(super) fn assert_exact(function: VerifiedFunction<'_>, context: Context) {
    let blocks = function.blocks().collect::<Vec<_>>();
    let downgrade = blocks[0]
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::WeakDowngrade)
        .expect("one source downgrade");
    let shared = downgrade.place_operands().next().expect("Shared root");
    let downgrade_result = downgrade.result().expect("Weak result");
    let weak = blocks[0]
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::InitializePlace
                && instruction.value_operands().any(|value| value == downgrade_result)
        })
        .expect("Weak local publication")
        .place_operands()
        .next()
        .expect("Weak root");
    let parents = blocks
        .iter()
        .filter_map(|block| {
            let (success, _) = block.terminator().weak_upgrade_edges()?;
            let parameter = blocks
                .iter()
                .find(|block| block.id() == success.target())?
                .parameters()
                .next()?
                .id();
            let owner = function
                .places()
                .find(|place| place.kind() == VerifiedPlaceKind::Temporary(parameter))?
                .id();
            Some((success.target(), owner))
        })
        .collect::<Vec<_>>();
    for block in &blocks {
        let terminator = block.terminator();
        if terminator.kind() == VerifiedTerminatorKind::WeakUpgradeBranch {
            let operand = terminator.place_operands().next().expect("upgrade operand");
            let mut expected = Vec::new();
            if operand != weak {
                expected.push(operand);
            }
            if let Some((_, owner)) = parents.iter().find(|(target, _)| *target == block.id()) {
                expected.push(*owner);
            }
            expected.extend([weak, shared]);
            assert_eq!(
                terminator.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
                expected,
                "{context:?}: exact live owner completion order"
            );
        }
        if terminator.kind() == VerifiedTerminatorKind::Return {
            assert_eq!(
                terminator.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
                [weak],
                "only retained Weak remains after final Shared return"
            );
        }
    }
    assert_operands(&blocks, context, weak, shared);
}

fn assert_operands(
    blocks: &[VerifiedBlock<'_>],
    context: Context,
    weak: PlaceIdentity,
    shared: PlaceIdentity,
) {
    if matches!(context, Context::Call) {
        let instructions = blocks[0].instructions().collect::<Vec<_>>();
        let cloned = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::WeakClone)
            .expect("argument prepared once");
        let called = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .expect("call once");
        assert!(cloned < called);
        assert_eq!(
            instructions[called].value_operands().collect::<Vec<_>>(),
            [instructions[cloned].result().expect("prepared argument")]
        );
        for index in [cloned, called] {
            assert_eq!(
                instructions[index]
                    .derived_drop_actions()
                    .map(|action| action.root())
                    .collect::<Vec<_>>(),
                [weak, shared],
                "argument retains roots; call transfers the argument before trap cleanup"
            );
        }
        assert_eq!(
            blocks[0].terminator().kind(),
            VerifiedTerminatorKind::WeakUpgradeBranch,
            "call completes before the only upgrade"
        );
    }
    if matches!(context, Context::Match) {
        let matched = blocks
            .iter()
            .find(|block| block.terminator().kind() == VerifiedTerminatorKind::EnumMatch)
            .expect("Match before upgrade")
            .terminator()
            .place_operands()
            .next()
            .expect("materialized scrutinee");
        let clones = blocks
            .iter()
            .flat_map(|block| block.instructions())
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::WeakClone)
            .collect::<Vec<_>>();
        assert_eq!(clones.len(), 2, "each active payload arm prepares once");
        for clone in clones {
            assert_eq!(
                clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
                [weak, shared, matched],
                "arm failure retains exact active scrutinee before the joined result exists"
            );
        }
    }
}
