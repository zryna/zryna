//! Actual private production host operation/provider quota, not a second public guest call.

use super::super::{Approval, Observation, grants::Grants};
use super::*;
use std::time::Duration;
use wasmtime::{AsContextMut, component::Val};

#[test]
fn private_clock_operation_one_read_then_second_limit_preserves_unpublished_output() {
    let limits = crate::server_transport::Config::parse(
        br#"{"listen":"127.0.0.1:0","attempts":1,"header_bytes":1024,"body_bytes":32,"request_ms":5000,"service_ms":30000}"#,
    )
    .expect("public configuration")
    .envelope();
    let observation = Arc::new(Observation::default());
    let now = Instant::now();
    let control = Arc::new(Control {
        revoked: AtomicBool::new(false),
        deadline: now + Duration::from_secs(30),
        engine: super::super::envelope::engine().expect("unchanged server engine"),
        state: Mutex::new(State { activated: true, done: true, result: None, input: None }),
        wake: Condvar::new(),
        worker: Mutex::new(None),
        observation: Arc::clone(&observation),
    });
    let grants = Grants::admit(
        br#"{"world":"zryna:capability-profiles/server@0.1.0","requests":["clock"],"clock":{"monotonic_reads":1,"subscriptions":0,"timers":0}}"#,
        Approval::one_monotonic_clock_read(),
    )
    .expect("exact public one-read request and root approval");
    for attempt in 1..=2 {
        let host = Host::new(limits, grants, Arc::clone(&observation), Arc::clone(&control), now);
        let mut store = Store::new(&control.engine, host);
        store.limiter(|host| &mut host.limits);
        store.set_fuel(limits.fuel).expect("unchanged production fuel");
        store.set_epoch_deadline(1);
        store.data_mut().store_started();
        let mut output = [Val::U64(u64::MAX)];
        assert_eq!(
            operations::invoke(
                &mut store.as_context_mut(),
                "wasi:clocks/monotonic-clock@0.2.12",
                "now",
                &[],
                &mut output,
            ),
            Ok(())
        );
        assert_eq!(observation.clock_reads.load(Ordering::SeqCst), attempt);
        output[0] = Val::U64(u64::MAX);
        assert_eq!(
            operations::invoke(
                &mut store.as_context_mut(),
                "wasi:clocks/monotonic-clock@0.2.12",
                "now",
                &[],
                &mut output,
            ),
            Err(Error::Limit)
        );
        assert!(matches!(output[0], Val::U64(u64::MAX)), "quota rejection publishes no timestamp");
        assert_eq!(observation.clock_reads.load(Ordering::SeqCst), attempt);
        assert_eq!(
            observation.denials.load(Ordering::SeqCst),
            0,
            "quota failure is not missing grant"
        );
        drop(store);
        assert_eq!(observation.live(), (0, 0));
        assert_eq!(observation.stores_created.load(Ordering::SeqCst), attempt);
        assert_eq!(observation.stores_destroyed.load(Ordering::SeqCst), attempt);
    }
}
