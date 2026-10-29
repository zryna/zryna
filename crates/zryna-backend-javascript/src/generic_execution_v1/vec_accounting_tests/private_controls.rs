//! Genuine retained Vec recipe and actual emitted ledger/glue with seeded private counters.
use crate::generic_execution_v1::{
    Context, Writer, accounting_tests, container_tests, runtime::accounting, shapes,
};
use sha2::{Digest, Sha256};
#[test]
fn vec_storage_growth_and_real_clone_prefix_match_the_eighteen_frozen_held_rows() {
    let addendum = include_str!("private_controls/ADDENDUM.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(addendum)),
        "5493da462d23d7b229d40d02f74850663dcc119ea4de8c7f522ab6afeaf480b9"
    );
    let oracle = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../zryna-ownership-runtime-abi/tests/generic_execution_accounting_vec/HELD_COUNTER_EXPECTED_FREEZE.json"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "5c6cbfc840dd06b29a409b5994002c294bf7436dd7d6be7a0abddede76f632f0"
    );
    for provider in ["native", "typescript"] {
        container_tests::source::with_program("vec-owned-prepare", provider, |program| {
            let clone = program
                .operations()
                .filter_map(zryna_ir::generic_v1::execution_v1::VerifiedOperation::clone_frontier)
                .find(|clone| {
                    clone.frontier().kind()
                        == zryna_ir::generic_v1::execution_v1::CloneNodeKind::VecEach
                })
                .expect("actual source Clone<Vec<String>> site");
            let vector = clone.ty();
            let string = clone.frontier().element().expect("exact retained String child").ty();
            let mut context = Context::new(program).expect("genuine source/type/owner");
            context.accounting = accounting::Plan::derive(&context).expect("own prepaid scan");
            assert!(
                context.accounting.as_ref().is_some_and(|plan| plan.bound(&context)),
                "Vec owner must actually activate"
            );
            let layout = context.element(string).expect("same runtime element metadata");
            let mut counter = Writer::counter(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
            shapes::declarations(&context, &mut counter).expect("prepaid actual glue counter");
            let mut writer =
                Writer::output(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES, counter.bytes())
                    .expect("bounded actual glue");
            shapes::declarations(&context, &mut writer)
                .expect("actual retained Clone/drop recipe emission");
            let script = format!(
                "const $heldSource={};\n{}\nconst $el{string}=Object.freeze({{stride:{},alignment:{},id:{string}}});\n{}\nconst $vectorClone=$clone{vector},$vectorDrop=$drop{vector},$stringDrop=$drop{string},$stringElement=$el{string};\n{}",
                serde_json::to_string(oracle).expect("frozen oracle literal"),
                accounting::LEDGER,
                layout.stride(),
                layout.alignment(),
                writer.finish(),
                include_str!("private_controls/cases.mjs")
            );
            let receipt =
                accounting_tests::private_boundary(&format!("vec-held-{provider}"), &script);
            assert_eq!(receipt["cases"], 18);
            assert_eq!(receipt["seededPrivateCounterEvidence"], true);
            assert_eq!(receipt["actualRetainedCloneAndDropGlue"], true);
        });
    }
}
#[test]
fn vec_effect_gate_prepays_its_genuine_scan_and_pristine_retry() {
    super::source::with_program("reserve-vec-call-loop", "native", |program, _| {
        let context = Context::new(program).expect("actual source issuer");
        context.work.set(0);
        assert!(accounting::Plan::derive(&context).expect("own scan").is_some());
        let cost = context.work.get();
        context.work.set(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - cost);
        assert!(accounting::Plan::derive(&context).expect("exact full scan").is_some());
        assert_eq!(context.work.get(), crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
        for before in [
            crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - context.types.len() + 1,
            usize::MAX,
        ] {
            context.work.set(before);
            let Err(error) = accounting::Plan::derive(&context) else {
                panic!("first scan debit must fail");
            };
            assert_eq!(error.code(), "ZRYNA-J2003");
            assert_eq!(context.work.get(), before);
        }
        context.work.set(0);
        assert!(accounting::Plan::derive(&context).expect("pristine retry").is_some());
        assert_eq!(context.work.get(), cost);
    });
}

#[test]
fn reserve_rejects_unfunded_existing_storage_before_admission_or_canonical_entry() {
    container_tests::source::with_program("vec-owned-prepare", "native", |program| {
        let context = Context::new(program).expect("genuine Vec owner");
        assert!(
            accounting::Plan::derive(&context).expect("actual effect classification").is_some()
        );
        let script =
            format!("{}\n{}", accounting::LEDGER, include_str!("private_controls/funding.mjs"));
        let receipt = accounting_tests::private_boundary("vec-existing-storage-funding", &script);
        assert_eq!(receipt["unfundedCases"], 3);
        assert_eq!(receipt["fundedCases"], 2);
        assert_eq!(receipt["seededPrivateCounterEvidence"], true);
    });
}
