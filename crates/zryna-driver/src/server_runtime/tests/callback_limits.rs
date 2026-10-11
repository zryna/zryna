//! Private policy invocations, not additional admitted public guest callbacks.

use super::super::{Observation, grants::Grants};
use super::*;
use std::time::Duration;

#[test]
fn private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth() {
    let limits = crate::server_transport::Config::parse(
        br#"{"listen":"127.0.0.1:0","attempts":1,"header_bytes":1024,"body_bytes":32,"request_ms":5000,"service_ms":30000}"#,
    )
    .expect("public configuration")
    .envelope();
    assert_eq!(limits.callbacks, 8);
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
        br#"{"world":"zryna:capability-profiles/server@0.1.0","requests":[]}"#,
        super::super::Approval::deny_all(),
    )
    .expect("empty grants");
    for _ in 0..2 {
        let mut host =
            Host::new(limits, grants, Arc::clone(&observation), Arc::clone(&control), now);
        for _ in 0..8 {
            assert_eq!(host.enter(), Ok(()), "unchanged policy admits each of eight calls");
        }
        assert_eq!(host.enter(), Err(Error::Limit), "ninth policy invocation rejects");
        assert_eq!(host.enter(), Err(Error::Limit), "exhaustion cannot wrap into admission");
        assert!(host.response.is_none());
        drop(host);
        assert_eq!(observation.live(), (0, 0));
        assert_eq!(observation.clock_reads.load(Ordering::SeqCst), 0);
        assert_eq!(observation.denials.load(Ordering::SeqCst), 0);
    }
}
