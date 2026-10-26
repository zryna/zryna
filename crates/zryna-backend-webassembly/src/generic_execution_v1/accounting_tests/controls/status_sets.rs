//! Unsupported canonical statuses are ABI failures; shims are explicitly untrusted tests.
use super::{harness::Harness, with_model};
use crate::generic_execution_v1::model::containers as C;
#[test]
fn unsupported_birth_status_cancels_two_debts_and_cannot_publish_a_pointer() {
    with_model("control-live-count-call-loop", "native", |m, wire| {
        let h = Harness::with_shim(
            m,
            wire,
            "birth-invalid-status",
            usize::try_from(m.container_base + C::ALLOCATE_RAW).expect("raw role"),
            4,
            true,
        );
        let mut s = h.session();
        let result = s.wide("controlAllocate", (h.control_size, h.control_alignment));
        let post = s.state();
        h.retain(
            "unsupported-birth",
            &serde_json::json!({"packed":result,"post":post,"synthetic_raw_status":4}),
        );
        assert_eq!(result, 255_u64 << 32);
        assert_eq!(post, [1, 0, 1, 0]);
    });
}
#[test]
fn unsupported_count_status_refunds_only_new_owner_and_preserves_funded_cleanup() {
    with_model("control-live-count-call-loop", "native", |m, wire| {
        let h = Harness::with_shim(
            m,
            wire,
            "count-invalid-status",
            usize::try_from(m.container_base + C::STRONG_CLONE).expect("raw role"),
            1,
            false,
        );
        let mut s = h.session();
        let packed = s.wide("controlAllocate", (h.control_size, h.control_alignment));
        assert_eq!(packed >> 32, 0);
        let pointer = u32::try_from(packed).expect("actual pointer");
        s.put(pointer, 1);
        s.put(pointer + 4, 1);
        s.put(pointer + 8, 7);
        let status = s.call1("strongClone", pointer);
        let post = s.state();
        h.retain("unsupported-count",&serde_json::json!({"status":status,"post":post,"strong":s.word(pointer),"synthetic_raw_status":1}));
        assert_eq!(status, 255);
        assert_eq!(post, [1, 1, 2, 2]);
        assert_eq!(s.word(pointer), 1);
        let out = h.scratch + 24;
        s.put(out, 0);
        assert_eq!(s.call2("begin", (pointer, out)), 0);
        assert_eq!(s.word(out), 1);
        assert_eq!(s.call1("finish", pointer), 0);
        assert_eq!(s.state(), [1, 0, 4, 0]);
    });
}
#[test]
fn representable_corrupt_context_rejects_before_wrap_or_a_canonical_call() {
    with_model("control-live-count-call-loop", "native", |m, wire| {
        let h = Harness::new(m, wire, "corrupt-u32");
        for (index, seed) in
            [[0, 0, u32::MAX, 0], [0, 0, 0, u32::MAX], [0, 1, 0, 0]].into_iter().enumerate()
        {
            let mut s = h.session();
            for (name, value) in ["A", "L", "S", "H"].into_iter().zip(seed) {
                s.set(name, value);
            }
            let cursor = s.get("cursor");
            let attempts = s.get("attempts");
            let result = s.wide("controlAllocate", (h.control_size, h.control_alignment));
            let post = s.state();
            h.retain(&format!("corrupt-{index}"),&serde_json::json!({"seed":seed,"packed":result,"post":post,"attempts":s.get("attempts"),"cursor":s.get("cursor")}));
            assert_eq!(result, 255_u64 << 32);
            assert_eq!(post, seed);
            assert_eq!(s.get("cursor"), cursor);
            assert_eq!(s.get("attempts"), attempts);
        }
    });
}
