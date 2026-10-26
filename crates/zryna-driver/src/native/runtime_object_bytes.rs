//! Pure bounded ELF runtime-object byte decoding; this grants no source or host authority.
//!
//! Both owning issuers map structural failures to their own unchanged diagnostics.

#[path = "native_c_v0/foreign_library/linked_output/runtime_object/inventory.rs"]
pub(in crate::native) mod inventory;
#[path = "native_c_v0/foreign_library/linked_output/runtime_object/relocations.rs"]
pub(in crate::native) mod relocations;
#[path = "native_c_v0/foreign_library/linked_output/runtime_object/tables.rs"]
pub(in crate::native) mod tables;

pub(in crate::native) const MAX_SECTIONS: usize = 16;
pub(in crate::native) const MAX_SYMBOLS: usize = 4096;
pub(in crate::native) const MAX_RELOCATIONS: usize = 32768;
// Names are a byte inventory constraint, never permission to select a host provider.
const IMPORTS: [&str; 4] = ["free", "malloc", "memcpy", "memset"];

#[derive(Clone, Copy, Debug)]
pub(in crate::native) enum Failure {
    Inventory,
    Limit,
}

fn error() -> Failure {
    Failure::Inventory
}

fn limit() -> Failure {
    Failure::Limit
}
