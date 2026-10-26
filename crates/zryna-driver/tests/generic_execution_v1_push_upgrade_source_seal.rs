//! Genuine frozen source packets must reach the NEW whole seal before backend work.
#[path = "generic_execution_v1_native_containers/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::{VerifiedCleanupAction, raw};
fn check(case: &str, upgrade: bool) {
    assert_eq!(source::SETTLED.len(), 4, "existing checkpoint remains unchanged");
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert_eq!(program.scalar_abi().exports().len(), 1);
            let mut found = 0;
            for function in program.functions() {
                if upgrade {
                    for block in function.blocks() {
                        let raw::Terminator::WeakUpgradeBranch {
                            success_binding,
                            success,
                            expired,
                            ..
                        } = block.terminator()
                        else {
                            continue;
                        };
                        found += 1;
                        let yes = function
                            .blocks()
                            .find(|b| b.id() == success.target)
                            .expect("exact success block");
                        assert_eq!(
                            yes.value_parameters().next().expect("generated Shared").id(),
                            success_binding.id
                        );
                        assert_eq!(
                            yes.value_parameters().next().expect("same generated type").ty(),
                            success_binding.ty
                        );
                        let no = function
                            .blocks()
                            .find(|b| b.id() == expired.target)
                            .expect("exact expired block");
                        assert_eq!(
                            no.value_parameters().len(),
                            0,
                            "expired path has no generated Shared"
                        );
                        let destination = function
                            .places()
                            .find(|p| p.kind() == raw::PlaceKind::Temporary(success_binding.id))
                            .expect("exact generated owner")
                            .id();
                        assert!(
                            matches!(yes.exit_cleanup().expect("success exit").actions().next(), Some(VerifiedCleanupAction::DropPlace(p)) if p.id() == destination),
                            "new success owner releases before outer owners"
                        );
                        assert!(!no.exit_cleanup().expect("expired exit").actions().any(|a| matches!(a, VerifiedCleanupAction::DropPlace(p) if p.id() == destination)));
                    }
                } else {
                    for operation in function
                        .blocks()
                        .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedBlock::operations)
                    {
                        if matches!(operation.kind(), raw::Operation::VecPush { .. }) {
                            found += 1;
                            assert!(
                                operation.result().is_none(),
                                "append publishes no value owner"
                            );
                            assert!(
                                operation.failure().is_some(),
                                "exact precommit growth cleanup"
                            );
                        }
                    }
                }
            }
            assert_eq!(found, 1, "genuine exact original operation reached whole issuer");
        });
    }
}
#[test]
fn frozen_vec_push_reaches_genuine_whole_seal_for_both_named_providers() {
    check("vec-push", false);
}
#[test]
fn frozen_weak_live_reaches_genuine_whole_seal_for_both_named_providers() {
    check("weak-live", true);
}
#[test]
fn frozen_weak_expired_reaches_genuine_whole_seal_for_both_named_providers() {
    check("weak-expired", true);
}
