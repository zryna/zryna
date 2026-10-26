//! Test-only lifetime exclusion; helper thread unlocks normally even during caller unwinding.
use std::{
    io,
    sync::mpsc,
    thread::{self, JoinHandle},
};

pub(super) struct Held {
    release: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}
impl Held {
    pub(super) fn acquire() -> io::Result<Self> {
        let (ready, readiness) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let worker =
            thread::Builder::new().name("listener-spawn-exclusion".into()).spawn(move || {
                let guard = match zryna_process::snapshot_writer() {
                    Ok(guard) => guard,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                // Final product keeps this exact gate across readiness and release.
                if ready.send(Ok(())).is_ok() {
                    let _ = released.recv();
                }
                drop(guard);
            })?;
        // Own cleanup BEFORE waiting: setup failure/unwind disconnects and joins.
        let held = Self { release: Some(release), worker: Some(worker) };
        match readiness.recv() {
            Ok(Ok(())) => Ok(held),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(io::Error::other("listener exclusion helper ended before readiness")),
        }
    }

    fn close(&mut self) -> io::Result<()> {
        // A disconnect also releases the helper; no panic/ack protocol can strand it.
        drop(self.release.take());
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| io::Error::other("listener exclusion helper panicked"))?;
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> io::Result<()> {
        self.close()
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
#[path = "listener_spawn_guard_tests.rs"]
mod tests;
