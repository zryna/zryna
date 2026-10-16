use super::*;
use std::{thread, time::Duration};

fn future() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn already_cancelled_connect_refuses_before_fixed_endpoint_acquisition() {
    let cancel = AtomicBool::new(true);
    assert!(matches!(open_cancellable(future(), u32::MAX, &cancel), Err(Error::Transport)));
}

#[test]
fn blocked_read_observes_cancellation_without_renewing_global_deadline() {
    let (stream, _peer) = UnixStream::pair().expect("private test pair");
    stream.set_nonblocking(true).expect("nonblocking");
    let cancel = AtomicBool::new(false);
    thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(Duration::from_millis(20));
            cancel.store(true, Ordering::Release);
        });
        let mut wire = Wire { stream, deadline: future(), cancel: Some(&cancel) };
        let start = Instant::now();
        let error = wire.read(&mut [0; 1]).expect_err("cancelled stalled read");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(start.elapsed() < Duration::from_secs(1));
    });
}

#[test]
fn blocked_write_and_connect_poll_observe_same_cancel_guard() {
    let (mut stream, _peer) = UnixStream::pair().expect("private test pair");
    stream.set_nonblocking(true).expect("nonblocking");
    let buffer = [0u8; 8192];
    while stream.write(&buffer).is_ok() {}
    let cancel = AtomicBool::new(false);
    thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(Duration::from_millis(20));
            cancel.store(true, Ordering::Release);
        });
        let start = Instant::now();
        assert_eq!(
            wait(&stream, PollFlags::POLLOUT, future(), Some(&cancel)),
            Err(Error::Transport)
        );
        let mut wire = Wire { stream, deadline: future(), cancel: Some(&cancel) };
        assert_eq!(wire.write(&buffer).expect_err("cancelled write").kind(), io::ErrorKind::Other);
        assert!(start.elapsed() < Duration::from_secs(1));
    });
}

#[test]
fn uncancelled_native_wire_still_uses_original_absolute_deadline() {
    let (stream, _peer) = UnixStream::pair().expect("private test pair");
    stream.set_nonblocking(true).expect("nonblocking");
    let deadline = Instant::now() + Duration::from_millis(20);
    let mut wire = Wire { stream, deadline, cancel: None };
    assert_eq!(
        wire.read(&mut [0; 1]).expect_err("original deadline").kind(),
        io::ErrorKind::TimedOut
    );
}
