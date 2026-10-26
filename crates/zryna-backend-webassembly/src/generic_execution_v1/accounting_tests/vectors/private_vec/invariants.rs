//! Explicit seed representability and pre-call missing-funding controls.
use super::*;
#[test]
fn unrepresentable_u64_seed_is_rejected_before_narrowing_and_corrupt_i32_or_unfunded_storage_is_atomic()
 {
    let oracle = oracle();
    let row = oracle["cases"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["id"] == "forged-overflow-context")
        .expect("unchanged u64 row");
    assert!(
        u32::try_from(row["pre"]["S"].as_u64().expect("u64 max")).is_err(),
        "no forged narrowed execution claim"
    );
    with_vec(|m, wire| {
        let h = Harness::new(m, wire, "vec-invariant", None);
        let _deadline = super::super::super::deadline::Deadline::new(h.engine.clone());
        let (_, element) = vector(m);
        let out = scratch(m) + 64;
        for (label, live, debt, statuses) in
            [("corrupt-i32", 1, 1, u32::MAX), ("unfunded-H", 1, 0, 1), ("unfunded-L", 0, 1, 1)]
        {
            let mut s = h.session();
            assert_eq!(s.call3("vecAllocate", (element, 4, out)), 0);
            s.set("L", live);
            s.set("H", debt);
            s.set("S", statuses);
            let before = s.state();
            let old = s.record(usize::try_from(out).expect("old"));
            let attempts = s.get("attempts");
            let cursor = s.get("cursor");
            s.poison(usize::try_from(out + 16).expect("output"));
            let status = s.call4_named("vecReserve", (element, out, 5, out + 16));
            let after = s.state();
            h.retain(label,&serde_json::json!({"status":status,"before":before,"after":after,"old":old,"untrusted_seed":true}));
            assert_eq!(status, 255);
            assert_eq!(after, before);
            assert_eq!(s.record(usize::try_from(out).expect("old")), old);
            assert_eq!(s.record(usize::try_from(out + 16).expect("out")), (0, 0, 0));
            assert_eq!(s.get("attempts"), attempts);
            assert_eq!(s.get("cursor"), cursor);
        }
    });
}
