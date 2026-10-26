//! Real emitted private ledger, genuine operation roles, seeded counters: not million-event evidence.
use super::super::accounting;
use crate::generic_execution_v1::{Context, accounting_tests, container_tests, push_upgrade_tests};
use sha2::{Digest, Sha256};
#[test]
fn string_owner_admission_release_failure_and_overflow_match_the_frozen_held_oracle() {
    accounting_tests::with_program("retained-clone-call-loop", "native", |program, _| {
        assert!(program.operations().any(|op| matches!(
            op.kind(),
            zryna_ir::generic_v1::execution_v1::raw::Operation::StringFromUtf8 { .. }
        )));
        let context = Context::new(program).expect("genuine source/type/owner indices");
        assert!(
            accounting::Plan::derive(&context)
                .expect("actual effect scan")
                .expect("String-only owner")
                .bound(&context)
        );
        run("string");
    });
}
#[test]
fn known_count_numeric_controls_retain_genuine_role_and_control_activation() {
    container_tests::source::with_program("shared-weak-count", "native", |program| {
        assert!(program.operations().any(|op| {
            op.clone_frontier().is_some_and(|c| {
                c.frontier().kind()
                    == zryna_ir::generic_v1::execution_v1::CloneNodeKind::SharedCount
            })
        }));
        let context = Context::new(program).expect("actual retained count role");
        assert!(
            accounting::Plan::derive(&context).expect("bounded role scan").is_some(),
            "genuine count source must activate its complete control effect transport"
        );
        run("count");
    });
}
#[test]
fn known_expired_numeric_control_retains_genuine_upgrade_and_control_activation() {
    push_upgrade_tests::with_program("weak-expired", "native", |program, _| {
        assert!(program.functions().any(|f| f.blocks().any(|b| b.weak_upgrade().is_some())));
        let context = Context::new(program).expect("actual weak/Shared source world");
        assert!(accounting::Plan::derive(&context).expect("bounded upgrade gate").is_some());
        run("weak");
    });
}
fn run(group: &str) {
    let oracle = include_str!("accounting_tests/HELD_COUNTER_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle.as_bytes())),
        "578b3f1d8373237b3c92a1267865fefe83243b17fed2d04eaa5098ff51f6faf2"
    );
    let source = format!(
        "const $heldSource={};\n{}\n{}",
        serde_json::to_string(oracle).expect("exact literal held oracle"),
        accounting::LEDGER,
        include_str!("accounting_tests/cases.mjs")
    );
    let receipt = accounting_tests::private_boundary(group, &source);
    assert_eq!(receipt["group"], group);
    assert_eq!(
        receipt["cases"],
        if group == "string" {
            12
        } else if group == "count" {
            2
        } else {
            1
        }
    );
    assert_eq!(receipt["seededPrivateCounterEvidence"], true);
}

#[test]
fn string_effect_owner_is_bound_to_one_genuine_issuer_and_prepays_its_scan() {
    accounting_tests::with_program("retained-clone-call-loop", "native", |program, _| {
        let context = Context::new(program).expect("genuine own-phase input");
        context.work.set(0);
        let plan =
            accounting::Plan::derive(&context).expect("actual private scan").expect("String owner");
        let cost = context.work.get();
        assert!(cost > context.types.len());
        context.work.set(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - cost);
        accounting::Plan::derive(&context).expect("exact complete scan credit").expect("owner");
        assert_eq!(context.work.get(), crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
        for start in [
            crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - context.types.len() + 1,
            usize::MAX,
        ] {
            context.work.set(start);
            let Err(error) = accounting::Plan::derive(&context) else {
                panic!("first debit rejects");
            };
            assert_eq!(error.code(), "ZRYNA-J2003");
            assert_eq!(context.work.get(), start);
        }
        context.work.set(0);
        accounting::Plan::derive(&context).expect("pristine retry").expect("same owner");
        assert_eq!(context.work.get(), cost);
        accounting_tests::with_program("retained-clone-call-loop", "native", |other, _| {
            let foreign = Context::new(other).expect("second actual issuer, same source bytes");
            assert!(!plan.bound(&foreign), "byte-equivalent new seals are distinct authority");
        });
    });
}
#[test]
fn accounted_ledger_manifest_pins_the_actual_private_guard_bytes() {
    let hash = format!("{:x}", Sha256::digest(accounting::LEDGER.as_bytes()));
    assert_eq!(accounting::MANIFEST, format!(",accounting:'{hash}'"));
}

mod representation;
