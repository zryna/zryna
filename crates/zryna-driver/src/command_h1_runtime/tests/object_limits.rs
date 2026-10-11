//! Private raw-core object counts using the unchanged production H1 limiter.

use super::super::envelope;
use wasm_encoder::{
    ExportKind, ExportSection, MemorySection, MemoryType, Module as EncodedModule, RefType,
    TableSection, TableType,
};
use wasmtime::{Engine, Instance, Module, Store, StoreLimits};

fn store(engine: &Engine) -> Store<StoreLimits> {
    let mut store = Store::new(engine, envelope::limits());
    store.limiter(|limits| limits);
    store.set_fuel(envelope::FUEL).expect("unchanged production fuel");
    store.set_epoch_deadline(1);
    store
}

#[test]
fn private_h1_one_memory_object_admits_one_and_rejects_second_across_instances() {
    let engine = envelope::engine().expect("unchanged H1 engine");
    let mut encoded = EncodedModule::new();
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(1),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    encoded.section(&memories);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    encoded.section(&exports);
    let module = Module::new(&engine, encoded.finish()).expect("one-memory WASM1 module");
    let mut store = store(&engine);
    let first = Instance::new(&mut store, &module, &[]).expect("first memory object admitted");
    let memory = first.get_memory(&mut store, "memory").expect("actual first memory");
    assert_eq!(memory.data_size(&store), 65_536);
    // The independent instance budget permits two instances; both individual
    // one-page memories are below 16 MiB. Only the one-memory object quota rejects.
    assert!(Instance::new(&mut store, &module, &[]).is_err());
    assert_eq!(memory.data_size(&store), 65_536, "first admitted memory remains intact");
    drop(store);
    let mut fresh = self::store(&engine);
    Instance::new(&mut fresh, &module, &[]).expect("fresh memory object quota recovers");
}

#[test]
fn private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table() {
    let engine = envelope::engine().expect("unchanged H1 engine");
    let empty = Module::new(&engine, EncodedModule::new().finish()).expect("zero-table fixture");
    let mut encoded = EncodedModule::new();
    let mut tables = TableSection::new();
    // Zero elements still request one actual table object, separating table
    // object count from the independent zero-element limit.
    tables.table(TableType {
        element_type: RefType::FUNCREF,
        table64: false,
        minimum: 0,
        maximum: Some(0),
        shared: false,
    });
    encoded.section(&tables);
    let table = Module::new(&engine, encoded.finish()).expect("WASM1 table declaration");
    let mut store = store(&engine);
    Instance::new(&mut store, &empty, &[]).expect("exact zero table objects admitted");
    assert!(Instance::new(&mut store, &table, &[]).is_err(), "first table object rejects");
    drop(store);
    let mut fresh = self::store(&engine);
    Instance::new(&mut fresh, &empty, &[]).expect("fresh zero-table instance recovers");
}
