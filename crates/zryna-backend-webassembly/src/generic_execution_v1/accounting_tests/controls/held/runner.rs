//! Real canonical headers plus explicitly synthetic A/L/S/H and count seeds.
use super::{Harness, Value};
use crate::generic_execution_v1::{accounting_tests::controls::harness::Session, memory};
fn array(value: &Value) -> [u32; 4] {
    std::array::from_fn(|i| {
        u32::try_from(value[i].as_u64().expect("counter")).expect("representable counter")
    })
}
fn allocate(h: &Harness, s: &mut Session, strong: u32, weak: u32) -> u32 {
    let packed = s.wide("rawControlAllocate", (h.control_size, h.control_alignment));
    assert_eq!(packed >> 32, 0, "test setup uses the actual canonical allocator");
    let pointer = u32::try_from(packed).expect("actual pointer");
    s.put(pointer, strong);
    s.put(pointer + 4, weak);
    s.put(pointer + 8, 7);
    pointer
}
pub(super) fn run(h: &Harness, row: &Value) {
    let id = row["id"].as_str().expect("row id");
    let mut s = h.session();
    let (mut pointer, payload) = initialized_controls(h, &mut s, id);
    for (name, value) in ["A", "L", "S", "H"].into_iter().zip(array(&row["pre"])) {
        s.set(name, value);
    }
    if id.contains("allocation-fault") {
        s.set("cursor", memory::MEMORY_BYTES - 8);
    }
    let mut actual = Vec::new();
    for (index, event) in row["events"].as_array().expect("events").iter().enumerate() {
        let role = event["role"].as_str().expect("role");
        let out = h.scratch + 24;
        s.put(out, 0);
        // This countMAX seed is not a naturally closed four-billion-owner program.
        // Restore the sole actual owner before Begin ONLY; never overwrite pending strong0 before Finish.
        if id.contains("known-refcount") && role.starts_with("StrongReleaseBegin") {
            s.put(pointer, 1);
        }
        let target = if role.contains("payload-distinct") { payload } else { pointer };
        let before_counts = if target != 0 {
            [s.word(target), s.word(target + 4), s.word(target - 8)]
        } else {
            [0; 3]
        };
        let before_cursor = s.get("cursor");
        let before_attempts = s.get("attempts");
        let status = if role == "RawAllocate-control" {
            let packed = s.wide("controlAllocate", (h.control_size, h.control_alignment));
            if packed >> 32 == 0 {
                pointer = u32::try_from(packed).expect("allocated pointer");
                s.put(pointer, 1);
                s.put(pointer + 4, 1);
                s.put(pointer + 8, 7);
            }
            u32::try_from(packed >> 32).expect("packed canonical status")
        } else if role.starts_with("StrongReleaseBegin") {
            s.call2("begin", (target, out))
        } else if role.starts_with("StrongReleaseFinish") {
            s.call1("finish", target)
        } else if role.starts_with("WeakRelease") {
            s.call2("weakRelease", (target, out))
        } else if role.starts_with("WeakDowngrade") {
            s.call1("downgrade", target)
        } else if role.starts_with("WeakClone") {
            s.call1("weakClone", target)
        } else if role.starts_with("WeakUpgrade") {
            s.call1("upgrade", target)
        } else {
            s.call1("strongClone", target)
        };
        let post = s.state();
        let flag = s.word(out);
        actual.push(serde_json::json!({"role":role,"status":status,"post":post,"flag":flag,"before_counts":before_counts,"synthetic_counts_and_context":true}));
        h.retain(&format!("{id}-{index}"), &serde_json::json!({"expected":row,"actual":actual}));
        let expected_status = match event["fault"].as_str() {
            Some("Denied") => 2,
            Some("Invariant") => 255,
            _ => u32::try_from(event["canonical_status"].as_u64().expect("canonical result"))
                .expect("status"),
        };
        assert_eq!(status, expected_status, "{id}/{role}");
        assert_eq!(post, array(&event["after"]), "{id}/{role}");
        if event["fault"] == "Denied" || event["fault"] == "Invariant" {
            assert_eq!(s.get("cursor"), before_cursor);
            assert_eq!(s.get("attempts"), before_attempts);
        }
        if status != 0 && target != 0 {
            assert_eq!(
                [s.word(target), s.word(target + 4), s.word(target - 8)],
                before_counts,
                "failed call preserves real control"
            );
        }
        if status == 0 && event["kind"] == "release" {
            assert_eq!(
                s.word(target - 8),
                u32::from(!event["positive"].as_bool().expect("release predicate")),
                "exact canonical physical header"
            );
        }
        if status == 0 && role.starts_with("StrongReleaseBegin") {
            assert_eq!(flag, u32::from(!role.contains("nonlast")), "last-strong flag");
        }
        if status == 0 && role.starts_with("WeakRelease") {
            assert_eq!(
                flag,
                u32::from(event["positive"].as_bool().expect("release predicate")),
                "actual physical-free flag"
            );
        }
    }
    // Rows ending with H>0 are synthetic arithmetic boundaries; instance disposal is not source cleanup.
}

fn initialized_controls(h: &Harness, s: &mut Session, id: &str) -> (u32, u32) {
    let mut pointer = 0;
    let mut payload = 0;
    if !id.starts_with("control-birth") {
        let expired = id.contains("known-expired")
            || id.starts_with("expired-two")
            || id.starts_with("invalid-free");
        let weak = if id.starts_with("weak-clone")
            || id.starts_with("weak-upgrade-live")
            || id.starts_with("expired-two")
            || id.starts_with("exhausted-forward")
        {
            2
        } else {
            1
        };
        let strong = if expired {
            0
        } else if id.contains("known-refcount") {
            u32::MAX
        } else {
            1
        };
        pointer = allocate(h, s, strong, weak);
        if id.starts_with("last-strong-distinct") {
            payload = allocate(h, s, 0, 1);
            s.put(pointer + 8, payload);
        }
    }
    (pointer, payload)
}
