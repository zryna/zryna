use super::super::*;
use std::{
    panic::catch_unwind,
    process::Command,
    sync::{atomic::AtomicBool, mpsc},
};

const WAIT: Duration = Duration::from_secs(2);

// Own both sides before any channel/assertion/network operation can unwind.
// Drop Running FIRST: it stops/joins Bound and releases the helper-owned gate;
// only then can the blocked actual spawner finish and be joined safely.
struct OwnedSpawn {
    running: Option<Running>,
    worker: Option<JoinHandle<std::process::ExitStatus>>,
}
impl OwnedSpawn {
    fn running(&self) -> &Running {
        self.running.as_ref().expect("owned Running")
    }
    fn finish(mut self) -> std::process::ExitStatus {
        self.running.take().expect("owned Running").finish();
        self.worker.take().expect("owned spawner").join().expect("spawner joined after finish")
    }
}
impl Drop for OwnedSpawn {
    fn drop(&mut self) {
        drop(self.running.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn panic_text(panic: &(dyn std::any::Any + Send)) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("exact fixture panic text")
}

#[test]
fn original_listener_lifetime_blocks_the_exact_actual_child_spawn_until_finish() {
    let mut owned =
        OwnedSpawn { running: Some(Running::start(config(1, 1000), "200")), worker: None };
    let entered = Arc::new(AtomicBool::new(false));
    let (identity, identities) = mpsc::channel();
    owned.worker = Some({
        let entered = Arc::clone(&entered);
        thread::spawn(move || {
            identity.send(thread::current().id()).expect("exact spawning thread");
            let mut child = zryna_process::spawn(|| {
                entered.store(true, Ordering::SeqCst);
                Command::new("/bin/true").spawn()
            })
            .expect("actual owned inert child");
            child.wait().expect("actual child reaped")
        })
    });
    let exact = identities.recv_timeout(WAIT).expect("spawner scheduled");
    let until = Instant::now() + WAIT;
    while !zryna_process::spawn_pending(exact)
        && !entered.load(Ordering::SeqCst)
        && Instant::now() < until
    {
        thread::yield_now();
    }
    assert!(
        zryna_process::spawn_pending(exact),
        "exact child creator must wait while original listener is live"
    );
    assert!(
        !entered.load(Ordering::SeqCst),
        "child creation cannot enter before strict closure proof"
    );
    let address = owned.running().address;
    assert!(exchange(address, &frame(address, b"held")).starts_with(b"HTTP/1.1 200 "));
    assert!(
        zryna_process::spawn_pending(exact),
        "exact child creator still waits after real HTTP exchange"
    );
    assert!(
        !entered.load(Ordering::SeqCst),
        "child creation cannot enter while Running still owns exclusion"
    );
    assert!(owned.finish().success());
    assert!(entered.load(Ordering::SeqCst));
}

#[test]
fn caller_panic_joins_original_listener_and_releases_gate_without_poison() {
    let panic = catch_unwind(|| {
        let _running = Running::start(config(1, 1000), "200");
        panic!("listener lifetime caller panic");
    })
    .expect_err("actual caller unwind");
    assert_eq!(panic_text(panic.as_ref()), "listener lifetime caller panic");
    zryna_process::spawn(|| Ok(())).expect("caller panic cannot poison shared gate");
    let running = Running::start(config(1, 1000), "200");
    assert!(
        exchange(running.address, &frame(running.address, b"retry")).starts_with(b"HTTP/1.1 200 ")
    );
    running.finish();
}

#[test]
fn rejected_bound_start_releases_the_already_owned_helper() {
    let valid = config(1, 1000);
    let runtime = Arc::new(RuntimeObservation::default());
    let compiled = prepared(valid, "200", &runtime);
    let mut invalid = valid;
    invalid.attempts = 0;
    let panic = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _running = Running::from_prepared(invalid, compiled, runtime);
    }))
    .expect_err("bound startup rejects zero attempts");
    assert_eq!(panic_text(panic.as_ref()), "bounded bind: Config");
    zryna_process::spawn(|| Ok(())).expect("failed startup released and joined gate helper");
}
