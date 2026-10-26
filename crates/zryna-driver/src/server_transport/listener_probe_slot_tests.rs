use super::*;
use std::panic::catch_unwind;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

const CONTROL_WAIT: Duration = Duration::from_secs(2);
struct Release(Option<mpsc::Sender<()>>);
impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[test]
fn plain_threads_share_one_probe_slot_without_serializing_the_test_harness() {
    let slot = Arc::new(Slot::new());
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let second_done = Arc::new(AtomicBool::new(false));
    let (entered, entering) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let releasing = Release(Some(release));
    let first = {
        let slot = Arc::clone(&slot);
        let active = Arc::clone(&active);
        let maximum = Arc::clone(&maximum);
        thread::spawn(move || {
            slot.run(PROBE_TIME, |_| {
                maximum.fetch_max(active.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                entered.send(()).expect("first slot entered");
                released.recv_timeout(CONTROL_WAIT).expect("release first probe");
                active.fetch_sub(1, Ordering::SeqCst);
            })
        })
    };
    entering.recv_timeout(CONTROL_WAIT).expect("actual first probe entered");
    let second = {
        let slot = Arc::clone(&slot);
        let active = Arc::clone(&active);
        let maximum = Arc::clone(&maximum);
        let second_done = Arc::clone(&second_done);
        thread::spawn(move || {
            slot.run(PROBE_TIME, |_| {
                maximum.fetch_max(active.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                active.fetch_sub(1, Ordering::SeqCst);
                second_done.store(true, Ordering::SeqCst);
            })
        })
    };
    let until = Instant::now() + PROBE_TIME;
    while !slot.contended.load(Ordering::SeqCst)
        && !second_done.load(Ordering::SeqCst)
        && Instant::now() < until
    {
        thread::yield_now();
    }
    // The old nonholding baseline demonstrably overlaps, rather than passing
    // because the second thread was never scheduled.
    assert_eq!(maximum.load(Ordering::SeqCst), 1, "actual concurrent probe count");
    assert!(slot.contended.load(Ordering::SeqCst), "actual waiter reached held slot");
    drop(releasing);
    first.join().expect("first probe thread").expect("first acquired");
    second.join().expect("second probe thread").expect("second acquired after release");
    assert_eq!(maximum.load(Ordering::SeqCst), 1);
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert!(second_done.load(Ordering::SeqCst));
}

#[test]
fn held_slot_wait_expires_before_query_and_pristine_query_recovers() {
    let slot = Arc::new(Slot::new());
    let (entered, entering) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let releasing = Release(Some(release));
    let first = {
        let slot = Arc::clone(&slot);
        thread::spawn(move || {
            slot.run(PROBE_TIME, |_| {
                entered.send(()).expect("holder entered");
                released.recv_timeout(CONTROL_WAIT).expect("release holder");
            })
        })
    };
    entering.recv_timeout(CONTROL_WAIT).expect("actual slot held");
    let called = AtomicBool::new(false);
    let error = slot
        .run(PROBE_TIME, |_| called.store(true, Ordering::SeqCst))
        .expect_err("late waiter must not start a query");
    match error {
        AcquireError::Timeout(waited) => assert!(waited >= PROBE_TIME && waited < CONTROL_WAIT),
        AcquireError::Poisoned => panic!("unrelated poison is not wait-deadline evidence"),
    }
    assert!(!called.load(Ordering::SeqCst));
    drop(releasing);
    first.join().expect("holder thread").expect("holder acquired");
    assert_eq!(slot.run(PROBE_TIME, |_| 7).expect("pristine query"), 7);
}

#[test]
fn negative_probe_panic_is_resumed_after_normal_unlock() {
    let slot = Slot::new();
    let panic = catch_unwind(|| slot.run(PROBE_TIME, |_| panic!("owned probe failure")))
        .expect_err("original negative probe panic");
    assert_eq!(panic.downcast_ref::<&str>(), Some(&"owned probe failure"));
    assert_eq!(slot.run(PROBE_TIME, |_| 9).expect("panic cannot poison slot"), 9);
    let called = AtomicBool::new(false);
    assert!(matches!(
        slot.run(Duration::ZERO, |_| called.store(true, Ordering::SeqCst)),
        Err(AcquireError::Timeout(_))
    ));
    assert!(!called.load(Ordering::SeqCst), "expired slot cannot start query");
}
