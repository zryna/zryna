//! Genuine formal/lexical loans reach native MIR, actual ELF, and independent call-edge audit.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_loans/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::{VerifiedDropTarget, VerifiedLoanOrigin, raw};
use zryna_native_mir::generic_execution_v1::LoanAddress;
fn native(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let mir = zryna_native_mir::generic_execution_v1::lower(program).expect(
                "genuine loan operations must enter native MIR; unfinished N7002 is behavior RED",
            );
            assert!(std::ptr::eq(mir.program(), program));
            for function in program.functions() {
                let plan = &mir.functions()[function.index()];
                for loan in function.loans() {
                    if let Some(VerifiedLoanOrigin::Formal(ordinal)) = loan.origin() {
                        assert!(
                            loan.local_root().is_none(),
                            "formal referents have no fake local root"
                        );
                        assert!(
                            matches!(plan.loan(loan.id()), Some(LoanAddress::Formal { parameter, offset: 0 }) if parameter == ordinal)
                        );
                    }
                    if let Some(VerifiedLoanOrigin::Place(place)) = loan.origin() {
                        let expected =
                            plan.place(place.id()).expect("same-function physical place");
                        assert!(
                            matches!(plan.loan(loan.id()), Some(LoanAddress::Frame(record)) if record.offset() == expected.offset() && record.ty() == expected.ty())
                        );
                    }
                }
                for operation in function
                    .blocks()
                    .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedBlock::operations)
                {
                    if let raw::Operation::ReplaceLoan { loan, .. } = operation.kind() {
                        let proof =
                            operation.drop_targets().expect("same-operation old referent proof");
                        assert!(proof.is_whole());
                        let mut targets = proof.targets();
                        assert_eq!(targets.len(), 1);
                        assert!(
                            matches!(targets.next(), Some(VerifiedDropTarget::Loan(target)) if target.id() == *loan)
                        );
                    }
                }
            }
            let target = zryna_backend_native::select_object_target(
                zryna_backend_native::NATIVE_OBJECT_TARGET,
            )
            .expect("existing qualified Linux target");
            let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
                .expect("actual native loan and replacement emitter");
            zryna_backend_native::generic_execution_v1::validate_object_inventory(
                object.bytes(),
                &mir,
            )
            .expect("independent IR-derived runtime/helper/private-call relocation inventory");
            assert!(!object.bytes().is_empty());
        });
    }
}
#[test]
fn formal_copy_mixed_parameters_enter_native_object() {
    native("formal-copy");
}
#[test]
fn opaque_generic_formal_forwarding_enters_native_object() {
    native("opaque-forward");
}
#[test]
fn lexical_shared_reborrow_clone_enters_native_object() {
    native("shared-reborrow-clone");
}
#[test]
fn lexical_exclusive_reborrow_push_enters_native_object() {
    native("exclusive-reborrow-push");
}
#[test]
fn formal_and_lexical_generic_replacement_enter_native_object() {
    native("generic-replace");
}
#[test]
fn static_disjoint_projection_reads_and_replacements_enter_native_object() {
    native("static-disjoint");
}
#[test]
fn conditional_lexical_cleanup_enters_native_object() {
    native("branch-cleanup");
}
#[test]
fn referent_clone_and_caller_failure_cleanup_enter_native_object() {
    native("fallible-cleanup");
}
#[test]
fn all_admitted_referent_shapes_enter_native_object() {
    native("referent-catalogue");
}
