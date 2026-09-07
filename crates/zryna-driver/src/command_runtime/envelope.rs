//! Runtime execution limits are separate from compilation and whole-process memory usage.

use std::{sync::mpsc, thread::JoinHandle, time::Duration};

use wasmtime::{Config, Engine, StoreLimits, StoreLimitsBuilder, WasmFeatures};

pub(super) const FUEL: u64 = 100_000;
pub(super) const EXECUTION_DEADLINE: Duration = Duration::from_secs(5);

pub(super) fn engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config
        .wasm_features(WasmFeatures::all(), false)
        .wasm_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL), true)
        .consume_fuel(true)
        .epoch_interruption(true)
        .max_wasm_stack(64 * 1024)
        .wasm_backtrace(false);
    Engine::new(&config)
}

pub(super) fn pure_limits() -> StoreLimits {
    StoreLimitsBuilder::new()
        .instances(2)
        .memories(0)
        .tables(0)
        .memory_size(0)
        .table_elements(0)
        .trap_on_grow_failure(true)
        .build()
}

/// Owns and joins the watchdog. Each execution uses its own engine, epoch and fresh store.
pub(super) struct Deadline {
    stop: mpsc::SyncSender<()>,
    worker: Option<JoinHandle<()>>,
}

impl Deadline {
    pub(super) fn start(engine: &Engine) -> std::io::Result<Self> {
        let (stop, receiver) = mpsc::sync_channel(1);
        let engine = engine.clone();
        let worker =
            std::thread::Builder::new().name("command-deadline".into()).spawn(move || {
                if matches!(
                    receiver.recv_timeout(EXECUTION_DEADLINE),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    engine.increment_epoch();
                }
            })?;
        Ok(Self { stop, worker: Some(worker) })
    }
}

impl Drop for Deadline {
    fn drop(&mut self) {
        let _ = self.stop.try_send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
