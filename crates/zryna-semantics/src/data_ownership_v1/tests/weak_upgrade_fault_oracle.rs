use super::generic_vec_fixture::weak_upgrade_fixture::fixture;
use super::*;

fn transition(before: u32, status: RuntimeStatus, after: u32) {
    validate_transition(TransitionClaim::WeakUpgrade { before, status, after })
        .expect("exact weak-upgrade transition");
}

#[test]
fn source_upgrade_binds_success_expiration_and_overflow_to_distinct_outcomes() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated weak upgrade");
    let program = lower(pair_input(&syntax, &sources)).expect("verified weak upgrade");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    let upgrade = blocks[0].terminator();
    let (success, expired) = upgrade.weak_upgrade_edges().expect("two outcomes");
    let success_block =
        blocks.iter().find(|block| block.id() == success.target()).expect("success block");
    let expired_block =
        blocks.iter().find(|block| block.id() == expired.target()).expect("expired block");

    transition(1, RuntimeStatus::Ok, 2);
    assert_eq!(success_block.parameters().count(), 1, "OK alone synthesizes Shared<T>");
    transition(0, RuntimeStatus::Expired, 0);
    assert_eq!(expired_block.parameters().count(), 0, "EXPIRED manufactures no value");
    transition(u32::MAX, RuntimeStatus::Refcount, u32::MAX);
    assert_ne!(success.target(), expired.target(), "overflow chooses neither source successor");
    assert!(!upgrade.derived_drop_actions().collect::<Vec<_>>().is_empty());

    let abi = program.runtime_abi();
    let expired_status = abi
        .status_declarations()
        .find(|declaration| declaration.status() == RuntimeStatus::Expired)
        .expect("expired status");
    assert_eq!(expired_status.disposition(), VerifiedStatusDisposition::Branch);
    assert_eq!(expired_status.trap_identity(), None);
    let overflow = abi
        .status_declarations()
        .find(|declaration| declaration.status() == RuntimeStatus::Refcount)
        .expect("refcount status");
    assert_eq!(overflow.disposition(), VerifiedStatusDisposition::ControlledTrap);
    assert_eq!(overflow.trap_identity(), Some(VerifiedStatusTrapIdentity::RefcountV1));

    for invalid in [
        TransitionClaim::WeakUpgrade { before: 0, status: RuntimeStatus::Ok, after: 1 },
        TransitionClaim::WeakUpgrade {
            before: u32::MAX,
            status: RuntimeStatus::Refcount,
            after: 0,
        },
        TransitionClaim::WeakUpgrade { before: 1, status: RuntimeStatus::Expired, after: 1 },
    ] {
        assert!(validate_transition(invalid).is_err());
    }
    let replay = lower(pair_input(&syntax, &sources)).expect("pristine recovery replay");
    assert_eq!(replay.verified_ir().identity(), program.verified_ir().identity());
}
