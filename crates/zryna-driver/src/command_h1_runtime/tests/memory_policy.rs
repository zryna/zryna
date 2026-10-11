//! Direct private limiter-policy admission, distinct from real page-granular growth.

use wasmtime::ResourceLimiter;

#[test]
fn private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217() {
    let mut limits = super::super::envelope::limits();
    assert!(limits.memory_growing(0, 16_777_216, Some(16_777_217)).expect("exact byte admission"));
    assert!(limits.memory_growing(16_777_216, 16_777_217, Some(16_777_217)).is_err());
    let mut fresh = super::super::envelope::limits();
    assert!(fresh.memory_growing(0, 16_777_216, Some(16_777_217)).expect("fresh exact admission"));
}
