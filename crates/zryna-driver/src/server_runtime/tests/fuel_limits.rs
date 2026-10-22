//! Private raw-core metering bound to the unchanged public server envelope.

use crate::runtime_boundary_tests as fuel_meter;

#[test]
fn private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion() {
    let limits = crate::server_transport::Config::parse(
        br#"{"listen":"127.0.0.1:0","attempts":1,"header_bytes":1024,"body_bytes":32,"request_ms":5000,"service_ms":30000}"#,
    )
    .expect("public configuration")
    .envelope();
    assert_eq!(limits.fuel, 100_000);
    limits.validate().expect("exact private envelope fuel admission");
    let extra = super::Envelope { fuel: 100_001, ..limits };
    assert!(matches!(extra.validate(), Err(super::Error::Limit)));
    let engine = super::super::envelope::engine().expect("unchanged server engine");
    fuel_meter::measured_production_exhaustion(&engine, limits.store_limits(), limits.fuel);
    let observation = std::sync::Arc::new(super::Observation::default());
    let server = super::server(
        "202",
        super::ServerOperation::Reply,
        super::EMPTY,
        super::Approval::deny_all(),
        limits,
        &observation,
    );
    assert_eq!(server.admit(&crate::input(b"ok")).expect("fresh admit").wait(), Ok((202, vec![])));
    server.shutdown().expect("fresh public arrangement joins");
    super::clean(&observation);
}
