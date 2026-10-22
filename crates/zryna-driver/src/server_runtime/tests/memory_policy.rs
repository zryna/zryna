//! Separate exact byte admission/policy from actual page-granular guest memory growth.

use wasmtime::ResourceLimiter;

#[test]
fn public_server_memory_envelope_accepts65536_and_rejects65537_before_host() {
    let limits = crate::server_transport::Config::parse(
        br#"{"listen":"127.0.0.1:0","attempts":1,"header_bytes":1024,"body_bytes":32,"request_ms":5000,"service_ms":30000}"#,
    )
    .expect("public configuration")
    .envelope();
    assert_eq!(limits.memory_bytes, 65_536);
    limits.validate().expect("exact memory envelope admission");
    let extra = super::Envelope { memory_bytes: 65_537, ..limits };
    assert!(matches!(extra.validate(), Err(super::Error::Limit)));
    let mut policy = limits.store_limits();
    assert!(policy.memory_growing(0, 65_536, Some(65_537)).expect("exact byte limiter policy"));
    assert!(policy.memory_growing(65_536, 65_537, Some(65_537)).is_err());
    let mut fresh = limits.store_limits();
    assert!(fresh.memory_growing(0, 65_536, Some(65_537)).expect("fresh exact byte policy"));
}
