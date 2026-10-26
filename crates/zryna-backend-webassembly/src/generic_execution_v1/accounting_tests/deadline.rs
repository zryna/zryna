//! Host-only interruption deadline; never a source runtime quota. Always cancel and reap.
use std::{sync::mpsc, thread, time::Duration};
pub(super) struct Deadline {
    cancel: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Deadline {
    pub(super) fn new(engine: wasmtime::Engine) -> Self {
        let (cancel, receive) = mpsc::channel();
        let thread = thread::spawn(move || {
            if matches!(
                receive.recv_timeout(Duration::from_mins(1)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                engine.increment_epoch();
            }
        });
        Self { cancel: Some(cancel), thread: Some(thread) }
    }
}
impl Drop for Deadline {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
