//! Real private Store/request admission bound to the public serial production envelope.

use super::*;

#[test]
fn production_serial_one_request_accepts_first_rejects_second_and_recovers() {
    let limits = crate::server_transport::Config::parse(
        br#"{"listen":"127.0.0.1:0","attempts":1,"header_bytes":1024,"body_bytes":32,"request_ms":5000,"service_ms":30000}"#,
    )
    .expect("public configuration")
    .envelope();
    assert_eq!(limits.requests.requests, 1);
    let observation = Arc::new(Observation::default());
    let (server, entered, release) = paused(limits, &observation);
    let first = server.admit(&crate::input(b"first")).expect("one real request admitted");
    entered.recv_timeout(Duration::from_secs(2)).expect("real Store pauses in clock callback");
    assert_eq!(observation.live(), (1, 2));
    assert_eq!(server.usage().expect("retained admission").0, 1);
    assert!(matches!(
        server.admit(&crate::input(b"second")),
        Err(Error::Lifecycle(crate::server_lifecycle::Error::Limit))
    ));
    assert_eq!(observation.stores_created.load(Ordering::SeqCst), 1);
    release.now();
    assert_eq!(first.wait(), Ok((204, vec![])));
    clean(&observation);
    assert_eq!(server.usage(), Ok((0, 0)));
    assert_eq!(
        server.admit(&crate::input(b"fresh")).expect("serial slot recovers").wait(),
        Ok((204, vec![]))
    );
    server.shutdown().expect("actual owned workers join");
    assert_eq!(observation.clock_reads.load(Ordering::SeqCst), 2);
    assert_eq!(observation.stores_created.load(Ordering::SeqCst), 2);
    clean(&observation);
}
