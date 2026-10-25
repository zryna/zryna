//! Genuine source-bound invocation preflight and opaque effect-owner boundaries.
use super::{MAX_CODEGEN_UNITS, check_boundary, lower};
use crate::generic_execution_v1::{accounting as owner, shapes};
use zryna_ir::generic_v1::execution_v1::raw;
use zryna_layout::StorageTarget;
#[path = "../../../../zryna-driver/tests/generic_execution_v1_native_accounting_string/source.rs"]
mod source;
const CASES: [&str; 3] = ["nonempty-call-loop", "empty-call-loop", "retained-clone-call-loop"];
#[test]
fn actual_invocation_preflight_and_complete_mir_exact_extra_overflow_retry() {
    for case in CASES {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _| {
                let types = shapes::derive(program.layouts(StorageTarget::LinuxX8664V1), &mut 0)
                    .expect("same actual independently checked Linux shapes");
                let mut used = 0;
                let plan =
                    owner::derive(program, &types, &mut used).expect("actual invocation preflight");
                assert_eq!(plan.context_bytes(), 32, "fixed shared four-u64 transport");
                assert!(
                    plan.guarded_string_sites() > 0 && used > 0,
                    "empty still owns logical cleanup"
                );
                let mut exact = MAX_CODEGEN_UNITS - used;
                let accepted =
                    owner::derive(program, &types, &mut exact).expect("exact invocation work");
                assert_eq!(exact, MAX_CODEGEN_UNITS);
                assert_eq!(accepted.guarded_string_sites(), plan.guarded_string_sites());
                let mut extra = MAX_CODEGEN_UNITS - used + 1;
                assert_eq!(
                    owner::derive(program, &types, &mut extra)
                        .err()
                        .expect("first extra actual accounting work")
                        .code(),
                    "ZRYNA-N7001"
                );
                assert!(extra <= MAX_CODEGEN_UNITS);
                let mut overflow = u64::MAX;
                assert_eq!(
                    owner::derive(program, &types, &mut overflow)
                        .err()
                        .expect("overflow before traversal")
                        .code(),
                    "ZRYNA-N7001"
                );
                assert_eq!(overflow, u64::MAX);
                let mut retry = 0;
                let clean =
                    owner::derive(program, &types, &mut retry).expect("pristine accounting retry");
                assert_eq!(retry, used);
                assert_eq!(clean.guarded_string_sites(), plan.guarded_string_sites());
                check_boundary(program, case);
            });
        }
    }
}
#[test]
fn direct_string_effect_requires_exact_program_operation_and_role() {
    source::with_program("nonempty-call-loop", "native", |program, _| {
        let mir = lower(program).expect("genuine current native owner");
        let operation = program
            .operations()
            .find(|op| matches!(op.kind(), raw::Operation::StringFromUtf8 { .. }))
            .expect("actual source String effect");
        let effect = mir.invocation_plan().source_string(operation).expect("same-seal effect");
        assert!(effect.belongs_to(program));
        let call = program
            .operations()
            .find(|op| op.call_target().is_some())
            .expect("actual source call carrier");
        assert_eq!(
            mir.invocation_plan()
                .source_string(call)
                .err()
                .expect("source call is not a canonical event")
                .code(),
            "ZRYNA-N7002"
        );
        assert_eq!(
            mir.string_leaf(1, true).err().expect("i32 cannot mint String clone effect").code(),
            "ZRYNA-N7002"
        );
        source::with_program("nonempty-call-loop", "typescript", |foreign, _| {
            let wrong = foreign
                .operations()
                .find(|op| matches!(op.kind(), raw::Operation::StringFromUtf8 { .. }))
                .expect("different actual seal has same source/type/site");
            assert!(!effect.belongs_to(foreign));
            assert_eq!(
                mir.invocation_plan()
                    .source_string(wrong)
                    .err()
                    .expect("foreign owner view rejected despite identical source/site")
                    .code(),
                "ZRYNA-N7002"
            );
        });
    });
}
