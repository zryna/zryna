//! Actual control preflight costs and same-seal source/terminator/leaf boundaries.
use super::{MAX_CODEGEN_UNITS, check_boundary, lower};
use crate::generic_execution_v1::{accounting as owner, shapes};
use zryna_layout::StorageTarget;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_native_accounting_control/source.rs"]
mod source;
const CASES: [&str; 3] =
    ["control-birth-call-loop", "control-live-count-call-loop", "control-expired-call-loop"];
#[test]
fn actual_control_preflight_and_full_mir_exact_extra_overflow_retry() {
    for case in CASES {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _| {
                let types = shapes::derive(program.layouts(StorageTarget::LinuxX8664V1), &mut 0)
                    .expect("actual shapes");
                let mut used = 0;
                let plan =
                    owner::derive(program, &types, &mut used).expect("actual control preflight");
                assert!(plan.guarded_control_sites() > 0 && used > 0);
                assert_eq!(plan.context_bytes(), 32);
                let mut exact = MAX_CODEGEN_UNITS - used;
                let accepted =
                    owner::derive(program, &types, &mut exact).expect("exact paid control work");
                assert_eq!(exact, MAX_CODEGEN_UNITS);
                assert_eq!(accepted.guarded_control_sites(), plan.guarded_control_sites());
                let mut extra = MAX_CODEGEN_UNITS - used + 1;
                assert_eq!(
                    owner::derive(program, &types, &mut extra).err().expect("first extra").code(),
                    "ZRYNA-N7001"
                );
                assert!(extra <= MAX_CODEGEN_UNITS);
                let mut overflow = u64::MAX;
                assert_eq!(
                    owner::derive(program, &types, &mut overflow).err().expect("overflow").code(),
                    "ZRYNA-N7001"
                );
                assert_eq!(overflow, u64::MAX);
                let mut retry = 0;
                let clean = owner::derive(program, &types, &mut retry).expect("pristine retry");
                assert_eq!(retry, used);
                assert_eq!(clean.guarded_control_sites(), plan.guarded_control_sites());
                check_boundary(program, case);
            });
        }
    }
}
#[test]
fn source_and_leaf_control_metadata_cannot_change_owner_or_role() {
    source::with_program(CASES[1], "native", |program, _| {
        let mir = lower(program).expect("actual native owner");
        let op = program
            .operations()
            .find(|op| op.shared_construct().is_some())
            .expect("actual Shared birth");
        let effect = mir.source_control(op).expect("same-seal birth");
        assert!(effect.belongs_to(program));
        let call = program
            .operations()
            .find(|op| op.call_target().is_some())
            .expect("actual private source call");
        assert_eq!(
            mir.source_control(call).err().expect("call not birth/count").code(),
            "ZRYNA-N7002"
        );
        assert_eq!(mir.control_clone(1).err().expect("i32 not control").code(), "ZRYNA-N7002");
        assert_eq!(
            mir.control_drop(1, false).err().expect("i32 not control").code(),
            "ZRYNA-N7002"
        );
        let weak = program
            .operations()
            .find_map(|op| {
                op.weak_downgrade()
                    .map(zryna_ir::generic_v1::execution_v1::VerifiedWeakDowngrade::ty)
            })
            .expect("actual Weak type");
        assert_eq!(
            mir.control_drop(weak, true).err().expect("Weak cannot mint Finish").code(),
            "ZRYNA-N7002"
        );
        source::with_program(CASES[1], "typescript", |foreign, _| {
            let wrong = foreign
                .operations()
                .find(|op| op.shared_construct().is_some())
                .expect("same site foreign seal");
            assert!(!effect.belongs_to(foreign));
            assert_eq!(
                mir.source_control(wrong).err().expect("foreign op rejected").code(),
                "ZRYNA-N7002"
            );
        });
    });
}
#[test]
fn upgrade_uses_exact_opaque_function_and_block_not_raw_index_selection() {
    source::with_program(CASES[2], "native", |program, _| {
        let mir = lower(program).expect("actual native owner");
        let (function, block) = program
            .functions()
            .find_map(|f| f.blocks().find(|b| b.weak_upgrade().is_some()).map(|b| (f, b)))
            .expect("actual source upgrade terminator");
        assert!(mir.source_upgrade(function, block).expect("exact pair").belongs_to(program));
        let other = program
            .functions()
            .find(|f| f.index() != function.index())
            .expect("different real function");
        assert_eq!(
            mir.source_upgrade(other, block)
                .err()
                .expect("block does not belong to other function")
                .code(),
            "ZRYNA-N7002"
        );
        source::with_program(CASES[2], "typescript", |foreign, _| {
            let (f, b) = foreign
                .functions()
                .find_map(|f| f.blocks().find(|b| b.weak_upgrade().is_some()).map(|b| (f, b)))
                .expect("foreign pair");
            assert_eq!(
                mir.source_upgrade(f, b).err().expect("foreign same-source pair rejected").code(),
                "ZRYNA-N7002"
            );
        });
    });
}
