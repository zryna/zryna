//! Remaining settled originals reach native admission through the actual whole issuer.
use super::{check_boundary, fixture, lower};
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, VerifiedFunction, VerifiedOperation};
use zryna_ownership_runtime_abi::LogicalOperation;

#[test]
fn genuine_push_and_both_upgrade_branches_reach_native_resource_boundaries() {
    for case in fixture::NEXT {
        fixture::with_program(case, |program| {
            let mir = lower(program).expect("genuine settled next-op native owner");
            let plan = mir.containers().expect("original stored Vec/Weak world");
            let push = program.operations().filter_map(VerifiedOperation::vec_push).count();
            let upgrades = program
                .functions()
                .flat_map(VerifiedFunction::blocks)
                .filter_map(VerifiedBlock::weak_upgrade)
                .count();
            assert_eq!((push, upgrades), if case == "vec-push" { (1, 0) } else { (0, 1) });
            assert_eq!(
                plan.operations().contains(&LogicalOperation::VecReserve),
                case == "vec-push"
            );
            assert_eq!(
                plan.operations().contains(&LogicalOperation::WeakUpgrade),
                case != "vec-push"
            );
            check_boundary(program, case);
        });
    }
}
