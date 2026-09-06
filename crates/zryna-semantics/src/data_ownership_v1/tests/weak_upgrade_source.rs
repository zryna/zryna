use super::generic_vec_fixture::weak_upgrade_fixture::fixture;
use super::*;

#[test]
fn weak_upgrade_source_seals_success_only_owner_and_retains_addressable_operand() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated weak upgrade");
    let program = lower(pair_input(&syntax, &sources)).expect("weak upgrade lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    let upgrade = blocks[0].terminator();
    assert_eq!(upgrade.kind(), VerifiedTerminatorKind::WeakUpgradeBranch);
    assert_eq!(upgrade.place_operands().count(), 1);
    let (success, expired) = upgrade.weak_upgrade_edges().expect("upgrade outcomes");
    assert_ne!(success.target(), expired.target());
    let success_block =
        blocks.iter().find(|block| block.id() == success.target()).expect("success block");
    let expired_block =
        blocks.iter().find(|block| block.id() == expired.target()).expect("expired block");
    assert_eq!(success_block.parameters().count(), 1);
    assert_eq!(expired_block.parameters().count(), 0);
    assert!(upgrade.derived_drop_actions().count() >= 2);
}

#[test]
fn temporary_weak_upgrade_operand_is_prepared_once_and_released_on_both_outcomes() {
    let (source, raw) = fixture(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated temporary weak upgrade");
    let program = lower(pair_input(&syntax, &sources)).expect("temporary weak upgrade lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(
        blocks[0]
            .instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::WeakClone)
            .count(),
        1
    );
    let upgrade = blocks[0].terminator();
    let operand = upgrade.place_operands().next().expect("temporary operand");
    let (success, expired) = upgrade.weak_upgrade_edges().expect("upgrade outcomes");
    for target in [success.target(), expired.target()] {
        let block = blocks.iter().find(|block| block.id() == target).expect("outcome block");
        assert!(block.instructions().any(|instruction| {
            instruction.kind() == VerifiedInstructionKind::DropPlace
                && instruction.place_operands().next() == Some(operand)
        }));
    }
    assert!(upgrade.derived_drop_actions().any(|action| action.root() == operand));
}
