//! Exact ten representable String-owned rows from the unchanged fifteen-row freeze.
use super::*;
use sha2::{Digest, Sha256};
fn frozen_held_rows() -> serde_json::Value {
    let oracle = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../zryna-ownership-runtime-abi/tests/generic_execution_accounting/HELD_COUNTER_EXPECTED_FREEZE.json"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "578b3f1d8373237b3c92a1267865fefe83243b17fed2d04eaa5098ff51f6faf2"
    );
    serde_json::from_str(oracle).expect("fixed15 held rows")
}

#[test]
fn actual_string_adapters_match_the_ten_representable_frozen_string_held_rows() {
    let oracle = frozen_held_rows();
    with_empty(|m, wire| {
        let harness = Harness::new(m, wire, "held-string10", None);
        let _deadline = super::super::deadline::Deadline::new(harness.engine.clone());
        let out = i32::try_from(m.memory.claims().scratch_start).expect("checked scratch");
        let input = out + 31;
        let heap = m.memory.claims().heap_start;
        let mut count = 0;
        for row in oracle["cases"].as_array().expect("frozen rows") {
            let id = row["id"].as_str().expect("frozen identifier");
            if !representable(id) {
                continue;
            }
            count += 1;
            let mut session = harness.session();
            session
                .memory
                .write(&mut session.store, usize::try_from(input).expect("input"), b"x")
                .expect("private safe one-byte input");
            let mut old = None;
            if row["pre"]["L"] == 1 {
                assert_eq!(session.call3("from", (input, 1, out)), 0);
                old = Some(session.record(usize::try_from(out).expect("record")));
            }
            for name in ["A", "L", "S", "H"] {
                session.set(
                    name,
                    u32::try_from(row["pre"][name].as_u64().expect("representable numeric seed"))
                        .expect("no narrowing"),
                );
            }
            session.poison(usize::try_from(out + 16).expect("separate output"));
            if matches!(
                id,
                "valid-allocation-failure-keeps-attempt"
                    | "potential-allocation-fault-debt-denied-first"
            ) {
                session.set("cursor", 16_777_208);
            }
            let status = if matches!(id, "cleanup-at-total-ceiling" | "invalid-held-debt-release") {
                session.call3("release", old.unwrap_or((0, 0, 0)))
            } else {
                let empty = id.starts_with("empty-");
                session.call3("from", (if empty { 0 } else { input }, i32::from(!empty), out + 16))
            };
            let after = session.state();
            let output = session.record(usize::try_from(out + 16).expect("output"));
            harness.retain(id,&serde_json::json!({"id":id,"returned_status":status,"post":after,"out_record":output,"cursor":session.get("cursor"),"raw_ordinal_not_A":session.get("attempts"),"seeded_private_evidence":true}));
            let expected = expected_status(row);
            assert_eq!(status, expected, "{id}");
            assert_eq!(after, row["expected"]["post"], "{id}");
            if !matches!(id, "cleanup-at-total-ceiling" | "invalid-held-debt-release")
                && status != 0
            {
                assert_eq!(output, (0, 0, 0), "{id}: all failed output lanes zero");
            }
            if status == 0
                && !matches!(id, "cleanup-at-total-ceiling" | "invalid-held-debt-release")
            {
                old = Some(output);
            }
            if !row["expected"]["after_cleanup"].is_null() {
                assert_eq!(session.call3("release", old.expect("actual live or empty owner")), 0);
                let cleanup = session.state();
                harness.retain(&format!("{id}-cleanup"), &cleanup);
                assert_eq!(cleanup, row["expected"]["after_cleanup"], "{id}");
            }
            if matches!(id, "potential-allocation-fault-debt-denied-first") {
                assert_eq!(
                    session.get("cursor"),
                    16_777_208,
                    "no raw allocation before shared denial"
                );
                assert_eq!(session.get("attempts"), 0, "no raw attempt ordinal before admission");
            }
            if matches!(
                id,
                "positive-attempt-first-extra"
                    | "empty-prospective-credit-first-extra"
                    | "empty-forward-first-extra"
            ) {
                assert_eq!(
                    session.get("cursor"),
                    if row["pre"]["L"] == 1 { heap + 24 } else { heap },
                    "denial does not allocate"
                );
            }
        }
        assert_eq!(
            count, 10,
            "UTF8/count/u64-only rows are explicitly outside this actual adapter subset"
        );
    });
}

fn representable(id: &str) -> bool {
    !matches!(
        id,
        "known-utf8-reject-no-owner"
            | "known-expired-only-forward-credit"
            | "known-refcount-only-forward-credit"
            | "count-success-needs-new-credit"
            | "forged-overflow-context"
    )
}
fn expected_status(row: &serde_json::Value) -> i32 {
    let admission = row["expected"]["admission"].as_str().expect("fixed expected admission");
    if admission.starts_with("DENIED") {
        2
    } else if admission.contains("ABI255") {
        255
    } else {
        i32::from(admission.contains("ALLOCATION1"))
    }
}
