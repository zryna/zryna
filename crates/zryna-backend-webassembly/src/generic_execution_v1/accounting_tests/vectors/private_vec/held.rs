//! Fifteen direct numeric rows; aggregate rows use actual Clone/Push roles in resources.rs.
use super::*;
#[test]
fn actual_vec_adapters_obey_fifteen_representable_frozen_numeric_rows() {
    let oracle = oracle();
    with_vec(|m, wire| {
        let h = Harness::new(m, wire, "vec-held15", None);
        let _deadline = super::super::super::deadline::Deadline::new(h.engine.clone());
        let (_, element) = vector(m);
        let out = scratch(m) + 64;
        let mut count = 0;
        for row in oracle["cases"].as_array().expect("unchanged18 rows") {
            let id = row["id"].as_str().expect("fixed id");
            if matches!(
                id,
                "forged-overflow-context"
                    | "clone-prefix-second-string-failure"
                    | "prepared-RHS-survives-reserve-failure"
            ) {
                continue;
            }
            count += 1;
            let mut s = h.session();
            let mut owner = None;
            if row["pre"]["L"] == 1 {
                assert_eq!(s.call3("vecAllocate", (element, 4, out)), 0);
                owner = Some(s.record(usize::try_from(out).expect("out")));
            } else if row["pre"]["H"] == 1 {
                assert_eq!(s.call3("vecAllocate", (element, 0, out)), 0);
                owner = Some((0, 0, 0));
            }
            for name in ["A", "L", "S", "H"] {
                s.set(
                    name,
                    u32::try_from(row["pre"][name].as_u64().expect("integer seed"))
                        .expect("checked representable seed"),
                );
            }
            let cursor = s.get("cursor");
            let attempts = s.get("attempts");
            s.poison(usize::try_from(out + 16).expect("separate output"));
            if id == "reserve-admitted-target-allocation-failure" {
                s.set("cursor", 16_777_208);
            }
            let release = matches!(
                id,
                "positive-storage-release-at-total-ceiling"
                    | "empty-storage-release-spends-credit"
                    | "release-without-storage-debt"
            );
            let allocate = id.starts_with("empty-allocate") || id.starts_with("positive-allocate");
            let status = if release {
                let old = owner.expect("actual old storage");
                s.call4_named("vecRelease", (element, old.0, old.1, old.2))
            } else if allocate {
                s.call3("vecAllocate", (element, i32::from(!id.starts_with("empty-")), out + 16))
            } else {
                let required = if id == "canonical-size-capacity-only-forward" {
                    1_048_577
                } else if id == "zero-reserve-noop" {
                    0
                } else if id.starts_with("within-capacity") {
                    1
                } else {
                    5
                };
                s.call4_named("vecReserve", (element, out, required, out + 16))
            };
            let post = s.state();
            let output = s.record(usize::try_from(out + 16).expect("output"));
            let expected = expected_status(row);
            h.retain(id,&serde_json::json!({"status":status,"post":post,"out":output,"old":owner,"seeded_private_evidence":true,"actual_source_threshold":false}));
            assert_eq!(status, expected, "{id}");
            assert_eq!(post, row["expected"]["post"], "{id}");
            if !release && status != 0 {
                assert_eq!(output, (0, 0, 0), "{id}: all output lanes zero");
            }
            if !allocate && !release {
                assert_eq!(
                    s.record(usize::try_from(out).expect("old record")),
                    owner.expect("old owner"),
                    "{id}: old descriptor retained"
                );
            }
            if row["expected"]["no_helper_or_side_effect"] == true {
                assert_eq!(s.get("cursor"), cursor, "{id}: no raw allocation");
                assert_eq!(s.get("attempts"), attempts, "{id}: no raw ordinal");
            }
            if status == 0 && !release {
                owner = Some(output);
            }
            if !row["expected"]["after_cleanup"].is_null() {
                let old = owner.expect("actual owner to clean");
                assert_eq!(s.call4_named("vecRelease", (element, old.0, old.1, old.2)), 0);
                let cleanup = s.state();
                h.retain(&format!("{id}-cleanup"), &cleanup);
                assert_eq!(cleanup, row["expected"]["after_cleanup"], "{id}");
            }
        }
        assert_eq!(count, 15);
    });
}

fn expected_status(row: &serde_json::Value) -> i32 {
    let admission = row["expected"]["admission"].as_str().expect("frozen admission");
    if admission.contains("ABI255") {
        255
    } else if admission.contains("ALLOCATION1") {
        1
    } else if admission.contains("CAPACITY2") {
        2
    } else {
        0
    }
}
