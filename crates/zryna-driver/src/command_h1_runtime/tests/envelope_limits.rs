//! Private core prerequisites using the unchanged public H1 engine and Store limits.
//! Raw modules here do not pass H1 artifact audit or claim public source admission.

use super::super::envelope;
use wasm_encoder::{ExportKind, ExportSection, MemorySection, MemoryType, Module as EncodedModule};
use wasmtime::{Instance, Module, Store};

fn store(engine: &wasmtime::Engine) -> Store<wasmtime::StoreLimits> {
    let mut store = Store::new(engine, envelope::limits());
    store.limiter(|limits| limits);
    store.set_fuel(envelope::FUEL).expect("fixed H1 fuel");
    store.set_epoch_deadline(1);
    store
}

#[test]
fn private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page() {
    let engine = envelope::engine().expect("unchanged H1 engine");
    let mut encoded = EncodedModule::new();
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 256,
        // Permit growth structurally so rejection belongs to the host's fixed 16 MiB limit.
        maximum: Some(257),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    encoded.section(&memories);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    encoded.section(&exports);
    let module = Module::new(&engine, encoded.finish()).expect("independent core fixture");
    let mut store = store(&engine);
    let instance = Instance::new(&mut store, &module, &[]).expect("exact 256 pages admitted");
    let memory = instance.get_memory(&mut store, "memory").expect("retained memory");
    assert_eq!(memory.data_size(&store), 16_777_216);
    assert!(memory.grow(&mut store, 1).is_err(), "first extra page exceeds fixed host limit");
    assert_eq!(memory.data_size(&store), 16_777_216);
    drop(store);
    let mut fresh = self::store(&engine);
    Instance::new(&mut fresh, &module, &[]).expect("fresh store quota recovers");
}

#[test]
fn private_h1_fixed_instance_limit_accepts_two_and_rejects_third() {
    let engine = envelope::engine().expect("unchanged H1 engine");
    let module = Module::new(&engine, EncodedModule::new().finish()).expect("empty core fixture");
    let mut store = store(&engine);
    Instance::new(&mut store, &module, &[]).expect("first instance admitted");
    Instance::new(&mut store, &module, &[]).expect("second instance admitted");
    assert!(Instance::new(&mut store, &module, &[]).is_err(), "third exceeds fixed host quota");
    drop(store);
    let mut fresh = self::store(&engine);
    Instance::new(&mut fresh, &module, &[]).expect("fresh store quota recovers");
}
