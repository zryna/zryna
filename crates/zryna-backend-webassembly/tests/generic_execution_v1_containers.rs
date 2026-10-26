//! Genuine settled container seals; original sources and expected values remain unchanged.
use zryna_backend_webassembly::generic_execution_v1::{emit, emit_observed, memory};
#[path = "generic_execution_v1_containers/source.rs"]
mod source;
const CASES: [(&str, i32, u32); 4] = [
    ("vec-copy", 14, 2),
    ("vec-owned-prepare", 7, 6),
    ("shared-weak-count", 7, 2),
    ("opaque-count-unused", 7, 0),
];
#[test]
fn genuine_container_sources_have_exact_results_private_memory_and_pristine_reentry() {
    for (case, expected, _) in CASES {
        assert!(source::SETTLED.contains(&case));
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program| {
                let plan = memory::verify(memory::derive(program, false).expect("claims"), program)
                    .expect("independent actual container frame/layout authority");
                assert_eq!(plan.claims().memory_pages, 256);
                assert!(
                    plan.claims().heap_start
                        >= plan.claims().frames_start + plan.claims().frames_bytes
                );
                let artifact = emit(program).expect("settled container target emission");
                assert_eq!(artifact.bytes(), emit(program).expect("deterministic retry").bytes());
                let engine = wasmtime::Engine::default();
                let module =
                    wasmtime::Module::new(&engine, artifact.bytes()).expect("audited Wasm");
                assert_eq!(module.imports().len(), 0);
                assert_eq!(module.exports().map(|v| v.name()).collect::<Vec<_>>(), vec!["entry"]);
                let mut store = wasmtime::Store::new(&engine, ());
                let instance =
                    wasmtime::Instance::new(&mut store, &module, &[]).expect("private memory");
                let entry =
                    instance.get_typed_func::<(), i32>(&mut store, "entry").expect("scalar ABI");
                assert_eq!(entry.call(&mut store, ()).expect("actual container result"), expected);
                assert_eq!(entry.call(&mut store, ()).expect("pristine reentry"), expected);
            });
        }
    }
}
#[test]
fn each_actual_container_allocation_failure_cleans_up_before_pristine_retry() {
    for (case, expected, allocations) in CASES {
        if allocations == 0 {
            continue;
        }
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program| {
                let artifact = emit_observed(program).expect("separate checked observation policy");
                for ordinal in 1..=allocations {
                    fault_and_retry(artifact.as_bytes(), 2, ordinal, expected);
                }
            });
        }
    }
}
#[test]
fn vec_clone_child_entry_failures_rollback_prefix_then_outer_owners() {
    for provider in source::PROVIDERS {
        source::with_program("vec-owned-prepare", provider, |program| {
            let artifact = emit_observed(program).expect("separate checked observation policy");
            for ordinal in 1..=2 {
                fault_and_retry(artifact.as_bytes(), 6, ordinal, 7);
            }
        });
    }
}
fn fault_and_retry(bytes: &[u8], kind: u32, ordinal: u32, expected: i32) {
    let engine = wasmtime::Engine::default();
    let module =
        wasmtime::Module::new(&engine, bytes).expect("independently audited observed module");
    assert_eq!(module.imports().len(), 0);
    assert_eq!(
        module.exports().map(|v| v.name()).collect::<Vec<_>>(),
        vec!["entry", "$zryna$observation"]
    );
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("private owned memory");
    let entry = instance.get_typed_func::<(), i32>(&mut store, "entry").expect("source export");
    let observer = instance
        .get_typed_func::<i32, i32>(&mut store, "$zryna$observation")
        .expect("checked observer");
    let config = 0x2000_0000 | (kind << 24) | ordinal;
    assert_eq!(observer.call(&mut store, config.cast_signed()).expect("configure fault"), 0);
    assert!(entry.call(&mut store, ()).is_err(), "actual injected resource failure");
    assert_eq!(observer.call(&mut store, 0).expect("canonical fault category"), 2);
    assert_eq!(observer.call(&mut store, 0x2000_0001).expect("reset requires no live owner"), 0);
    assert_eq!(entry.call(&mut store, ()).expect("actual pristine retry"), expected);
    assert_eq!(entry.call(&mut store, ()).expect("second pristine retry"), expected);
}
