//! Test-only kernel probe coordination; waiting is separate from the query deadline.
use std::{
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::{Mutex, MutexGuard, TryLockError},
    thread,
    time::{Duration, Instant},
};

use super::PROBE_TIME;

struct Slot {
    lock: Mutex<()>,
    #[cfg(test)]
    contended: std::sync::atomic::AtomicBool,
}
#[derive(Debug)]
enum AcquireError {
    Timeout(Duration),
    Poisoned,
}
impl Slot {
    const fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            #[cfg(test)]
            contended: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn acquire(&self, maximum: Duration) -> Result<(MutexGuard<'_, ()>, Duration), AcquireError> {
        let started = Instant::now();
        let deadline = started.checked_add(maximum).expect("bounded probe slot wait");
        loop {
            if Instant::now() >= deadline {
                return Err(AcquireError::Timeout(started.elapsed()));
            }
            match self.lock.try_lock() {
                Ok(guard) => {
                    let waited = started.elapsed();
                    if Instant::now() >= deadline {
                        drop(guard);
                        return Err(AcquireError::Timeout(waited));
                    }
                    return Ok((guard, waited));
                }
                Err(TryLockError::Poisoned(_)) => return Err(AcquireError::Poisoned),
                Err(TryLockError::WouldBlock) => {
                    #[cfg(test)]
                    self.contended.store(true, std::sync::atomic::Ordering::SeqCst);
                    thread::yield_now();
                }
            }
        }
    }

    fn run<R>(
        &self,
        maximum: Duration,
        probe: impl FnOnce(Duration) -> R,
    ) -> Result<R, AcquireError> {
        let (guard, waited) = self.acquire(maximum)?;
        let result = catch_unwind(AssertUnwindSafe(|| probe(waited)));
        // Intentional negative probes panic. Unlock normally before resuming so
        // their failure cannot poison the shared slot for unrelated probes.
        drop(guard);
        match result {
            Ok(value) => Ok(value),
            Err(panic) => resume_unwind(panic),
        }
    }
}
static SLOT: Slot = Slot::new();

pub(super) fn run<R>(probe: impl FnOnce(Duration) -> R) -> R {
    match SLOT.run(PROBE_TIME, probe) {
        Ok(value) => value,
        Err(AcquireError::Timeout(waited)) => {
            panic!(
                "original TCP probe slot wait expired: separate_wait={waited:?} wait_limit={PROBE_TIME:?}; query not started"
            )
        }
        Err(AcquireError::Poisoned) => {
            panic!("original TCP probe slot poisoned; query not started")
        }
    }
}

#[cfg(test)]
#[path = "listener_probe_slot_tests.rs"]
mod tests;
