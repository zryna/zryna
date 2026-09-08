//! Each sealed command receives one fresh store, and every completed or failed call tears it down.

use wasmtime::{
    Store,
    component::{Component, Instance},
};
use zryna_backend_webassembly::ValidatedCommandComponent;
use zryna_diagnostics::Diagnostic;

use super::{
    denied::{self, Denial, DeniedState},
    envelope::{self, Deadline},
};

pub(super) struct Session {
    store: Option<Store<DeniedState>>,
    instance: Instance,
    deadline: Option<Deadline>,
    pub(super) denial: Option<Denial>,
    pub(super) denial_entries: u32,
}

impl Session {
    pub(super) fn new(component: &ValidatedCommandComponent) -> Result<Self, Diagnostic> {
        let engine =
            envelope::engine().map_err(|_| error("command engine configuration failed"))?;
        let compiled = Component::new(&engine, component.bytes())
            .map_err(|_| error("audited command could not be compiled by the runtime"))?;
        Self::instantiate(&engine, &compiled, envelope::pure_limits())
    }

    #[cfg(test)]
    pub(super) fn probe(probe: &super::tests::probes::Probe) -> Result<Self, Diagnostic> {
        let engine = envelope::engine().map_err(|_| error("probe engine configuration failed"))?;
        let compiled = Component::new(&engine, probe.bytes())
            .map_err(|_| error("probe compilation failed"))?;
        let limits = wasmtime::StoreLimitsBuilder::new()
            .instances(2)
            .memories(1)
            .tables(0)
            .memory_size(64 * 1024)
            .table_elements(0)
            .trap_on_grow_failure(true)
            .build();
        Self::instantiate(&engine, &compiled, limits)
    }

    fn instantiate(
        engine: &wasmtime::Engine,
        compiled: &Component,
        limits: wasmtime::StoreLimits,
    ) -> Result<Self, Diagnostic> {
        let state =
            DeniedState::new(limits).map_err(|_| error("command store identity exhausted"))?;
        let linker = denied::linker(engine, compiled, &state)
            .map_err(|_| error("command denied import binding failed"))?;
        let mut store = Store::new(engine, state);
        store.limiter(|state| &mut state.limits);
        store.set_fuel(envelope::FUEL).map_err(|_| error("command fuel limit could not be set"))?;
        store.set_epoch_deadline(1);
        let deadline = Deadline::start(engine)
            .map_err(|_| error("command deadline watchdog could not start"))?;
        let instance = linker
            .instantiate(&mut store, compiled)
            .map_err(|_| error("command instantiation failed"))?;
        Ok(Self {
            store: Some(store),
            instance,
            deadline: Some(deadline),
            denial: None,
            denial_entries: 0,
        })
    }

    pub(super) fn invoke(&mut self) -> Result<Result<(), ()>, Diagnostic> {
        let mut store =
            self.store.take().ok_or_else(|| error("command instance has been invalidated"))?;
        let result = (|| {
            let interface = self
                .instance
                .get_export_index(&mut store, None, "wasi:cli/run@0.2.12")
                .ok_or_else(|| error("command run interface is missing at runtime"))?;
            let run = self
                .instance
                .get_export_index(&mut store, Some(&interface), "run")
                .ok_or_else(|| error("command run function is missing at runtime"))?;
            let run = self
                .instance
                .get_typed_func::<(), (Result<(), ()>,)>(&mut store, &run)
                .map_err(|_| error("command run function has a different runtime type"))?;
            // In the pinned runtime, TypedFunc::call includes canonical post-return handling.
            run.call(&mut store, ()).map(|(result,)| result).map_err(|_| {
                error("command trapped, was denied, or exceeded its execution envelope")
            })
        })();
        self.denial.clone_from(&store.data().first_denial);
        self.denial_entries = store.data().entries;
        let fatal = store.data().fatal;
        drop(store);
        self.deadline.take();
        if fatal {
            return Err(error("command attempted a denied host capability"));
        }
        result
    }
}

fn error(message: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C4022",
        None,
        message,
        "run a freshly prepared pure command within the denied host policy",
    )
}
