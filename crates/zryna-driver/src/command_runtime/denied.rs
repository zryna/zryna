//! Denied dynamic component imports with distinct resource identities and one-store ownership.

use std::sync::{
    Arc,
    atomic::{AtomicU32, AtomicU64, Ordering},
};

use wasmtime::{
    Engine, StoreLimits,
    component::{Component, Linker, ResourceType, types::ComponentItem},
};

static NEXT_STORE: AtomicU64 = AtomicU64::new(1);
static NEXT_RESOURCE_TYPE: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Denial {
    pub(super) interface: Arc<str>,
    pub(super) operation: Arc<str>,
    pub(super) resource_drop: bool,
}

pub(super) struct DeniedState {
    pub(super) limits: StoreLimits,
    owner: u64,
    pub(super) fatal: bool,
    pub(super) entries: u32,
    pub(super) first_denial: Option<Denial>,
}

impl DeniedState {
    pub(super) fn new(limits: StoreLimits) -> wasmtime::Result<Self> {
        let owner = NEXT_STORE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| wasmtime::format_err!("command store identities exhausted"))?;
        Ok(Self { limits, owner, fatal: false, entries: 0, first_denial: None })
    }

    fn deny(&mut self, owner: u64, denial: &Denial) -> wasmtime::Result<()> {
        self.fatal = true;
        if self.owner != owner {
            return Err(wasmtime::format_err!("command import belongs to another store"));
        }
        self.entries = self.entries.saturating_add(1);
        if self.first_denial.is_none() {
            self.first_denial = Some(denial.clone());
        }
        // No external provider is present; trapping precedes every result or resource construction.
        Err(wasmtime::format_err!("command host capability denied"))
    }
}

pub(super) fn linker(
    engine: &Engine,
    component: &Component,
    state: &DeniedState,
) -> wasmtime::Result<Linker<DeniedState>> {
    let mut linker: Linker<DeniedState> = Linker::new(engine);
    let component_type = component.component_type();
    let mut resources = Vec::<(ResourceType, ResourceType)>::new();
    let imports = component_type.imports(engine);
    if imports.len() > 16 {
        return Err(wasmtime::format_err!("command host interface limit exceeded"));
    }
    let mut items = 0;
    for (name, import) in imports {
        if name.len() > 256 || import.implements.is_some() || import.external_id.is_some() {
            return Err(wasmtime::format_err!("command host interface identity differs"));
        }
        let ComponentItem::ComponentInstance(interface) = import.ty else {
            return Err(wasmtime::format_err!("command host import is not an interface"));
        };
        let mut instance = linker.instance(name)?;
        for (operation, export) in interface.exports(engine) {
            items += 1;
            if items > 4096
                || operation.len() > 256
                || export.implements.is_some()
                || export.external_id.is_some()
            {
                return Err(wasmtime::format_err!(
                    "command host interface item exceeds its envelope"
                ));
            }
            let owner = state.owner;
            let mut denial = Denial {
                interface: Arc::from(name),
                operation: Arc::from(operation),
                resource_drop: false,
            };
            match export.ty {
                // Wasmtime supplies the exact component function type and performs canonical
                // argument lifting. The authenticated full graph has already excluded async functions.
                ComponentItem::ComponentFunc(_) => instance
                    .func_new(operation, move |mut store, _, _, _| {
                        store.data_mut().deny(owner, &denial)
                    })?,
                ComponentItem::Resource(guest_type) => {
                    let host_type = if let Some((_, host_type)) =
                        resources.iter().find(|(ty, _)| *ty == guest_type)
                    {
                        *host_type
                    } else {
                        let id = NEXT_RESOURCE_TYPE
                            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                                id.checked_add(1)
                            })
                            .map_err(|_| {
                                wasmtime::format_err!("command resource type identities exhausted")
                            })?;
                        let host_type = ResourceType::host_dynamic(id);
                        resources.push((guest_type, host_type));
                        host_type
                    };
                    denial.resource_drop = true;
                    instance.resource(operation, host_type, move |mut store, _| {
                        store.data_mut().deny(owner, &denial)
                    })?;
                }
                ComponentItem::Type(_) => {}
                _ => {
                    return Err(wasmtime::format_err!(
                        "command host interface contains an unsupported item"
                    ));
                }
            }
        }
    }
    Ok(linker)
}
