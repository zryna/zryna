//! Public production envelope execution and separately scoped private quota evidence.

use super::*;

fn public_envelope() -> Envelope {
    crate::server_transport::Config::parse(
        br#"{"listen":"127.0.0.1:0","attempts":1,"header_bytes":1024,"body_bytes":32,"request_ms":5000,"service_ms":30000}"#,
    )
    .expect("public configuration")
    .envelope()
}

#[test]
fn public_production_resource_callback_and_fuel_envelope_executes_both_arrangements() {
    let limits = public_envelope();
    assert_eq!((limits.resources, limits.callbacks, limits.fuel), (5, 8, 100_000));
    for (operation, document, approval, reads) in [
        (ServerOperation::Reply, EMPTY, Approval::deny_all(), 0),
        (ServerOperation::ClockRead, CLOCK, Approval::one_monotonic_clock_read(), 1),
    ] {
        let observation = Arc::new(Observation::default());
        let server = server("202", operation, document, approval, limits, &observation);
        assert_eq!(server.admit(&crate::input(b"ok")).expect("admit").wait(), Ok((202, vec![])));
        assert_eq!(observation.clock_reads.load(Ordering::SeqCst), reads);
        assert_eq!(observation.created.load(Ordering::SeqCst), 5);
        assert_eq!(server.usage(), Ok((0, 0)));
        server.shutdown().expect("joined");
        clean(&observation);
    }
}

#[test]
fn private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers() {
    use super::super::resources::{Kind, Resources};

    // The fixed public handler peaks at four live resources. These six independent
    // insertions exercise its production quota object; they are not public guest code.
    let observation = Arc::new(Observation::default());
    let mut resources = Resources::new(public_envelope().resources, Arc::clone(&observation));
    let ids = (0..5)
        .map(|_| resources.insert(Kind::Fields, 7).expect("within production live quota"))
        .collect::<Vec<_>>();
    assert_eq!(observation.live(), (0, 5));
    assert!(matches!(resources.insert(Kind::Fields, 7), Err(Error::Limit)));
    assert_eq!(observation.live(), (0, 5));
    assert_eq!(observation.created.load(Ordering::SeqCst), 5);
    drop(resources.take(ids[0], 7).expect("release one real owner"));
    let replacement = resources.insert(Kind::Fields, 7).expect("quota recovers");
    assert_ne!(replacement, ids[0]);
    assert_eq!(observation.live(), (0, 5));
    drop(resources);
    assert_eq!(observation.created.load(Ordering::SeqCst), 6);
    clean(&observation);
}
