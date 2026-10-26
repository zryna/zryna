//! Canonical byte limit is independent of the fixed target's allocation arena exhaustion.
use super::container_source as source;
use super::{artifact, code, memory, model};
use wasm_encoder::{ExportKind, ExportSection, Module, RawSection};
use wasmparser::{ExternalKind, Parser, Payload};

#[test]
fn canonical_maximum_requests_are_allocation_failures_and_first_extra_is_capacity() {
    assert!(source::SETTLED.contains(&"vec-copy"));
    for provider in source::PROVIDERS {
        source::with_program("vec-copy", provider, |program| {
            let plan =
                memory::verify(memory::derive(program, false).expect("genuine claim"), program)
                    .expect("actual fixed memory authority");
            let model = model::Model::new(program, &plan).expect("same genuine new seal");
            let code = code::build(&model).expect("actual lowered code");
            let ordinary = artifact::encode(&model, &code).expect("actual ordinary artifact");
            artifact::independent_audit(&ordinary, &model).expect("pristine independent owner");
            artifact::audit(&ordinary, &model, &code).expect("pristine complete artifact owner");
            let probe = private_probe(&ordinary, &model);
            assert!(
                artifact::audit(&probe, &model, &code).is_err(),
                "explicit test exports never gain ordinary artifact authority"
            );
            for string in [false, true] {
                exercise(&probe, plan.claims().scratch_start, string);
            }
        });
    }
}
fn private_probe(bytes: &[u8], model: &model::Model<'_, '_, '_>) -> Vec<u8> {
    let mut module = Module::new();
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.expect("audited ordinary sections");
        match payload {
            Payload::ExportSection(reader) => {
                let mut exports = ExportSection::new();
                for export in reader {
                    let export = export.expect("original scalar export");
                    assert_eq!(export.kind, ExternalKind::Func);
                    exports.export(export.name, ExportKind::Func, export.index);
                }
                exports.export("$probe_raw", ExportKind::Func, model.container_base);
                exports.export("$probe_string", ExportKind::Func, 4);
                exports.export("$probe_memory", ExportKind::Memory, 0);
                for (name, index) in [
                    ("$probe_heap", 0),
                    ("$probe_status", 1),
                    ("$probe_attempts", 5),
                    ("$probe_acquisitions", 7),
                    ("$probe_live", 8),
                ] {
                    exports.export(name, ExportKind::Global, index);
                }
                module.section(&exports);
            }
            other => {
                if let Some((id, range)) = other.as_section() {
                    let start =
                        usize::try_from(range.start).expect("bounded audited section start");
                    let end = usize::try_from(range.end).expect("bounded audited section end");
                    module.section(&RawSection { id, data: &bytes[start..end] });
                }
            }
        }
    }
    module.finish()
}
fn global(instance: &wasmtime::Instance, store: &mut wasmtime::Store<()>, name: &str) -> i32 {
    instance
        .get_global(&mut *store, name)
        .expect("explicit private test state")
        .get(store)
        .i32()
        .expect("actual i32 counter")
}
fn exercise(bytes: &[u8], scratch: u32, string: bool) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, bytes).expect("test-only unchanged kernel bodies");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("no imports");
    let raw = instance
        .get_typed_func::<(i32, i32), i64>(&mut store, "$probe_raw")
        .expect("private raw adapter");
    let strings = instance
        .get_typed_func::<(i32, i32, i32, i32, i32), i32>(&mut store, "$probe_string")
        .expect("private canonical String adapter");
    let entry =
        instance.get_typed_func::<(), i32>(&mut store, "entry").expect("genuine original entry");
    let memory =
        instance.get_memory(&mut store, "$probe_memory").expect("test-only memory inspection");
    let maximum = zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES;
    for request in maximum - 15..=maximum + 1 {
        let heap = global(&instance, &mut store, "$probe_heap");
        let acquisitions = global(&instance, &mut store, "$probe_acquisitions");
        let attempts = global(&instance, &mut store, "$probe_attempts");
        assert_eq!(global(&instance, &mut store, "$probe_live"), 0);
        let expected = if request <= maximum { 1 } else { 2 };
        let size = u32::try_from(request).expect("canonical request carrier").cast_signed();
        if string {
            let start = usize::try_from(scratch).expect("checked scratch address");
            // Canonical ABI requires the caller to initialize every out record to zero.
            memory.data_mut(&mut store)[start..start + 12].fill(0);
            assert_eq!(
                strings
                    .call(&mut store, (0, size, 0, 1, scratch.cast_signed()))
                    .expect("declared helper status, not an out-of-bounds trap"),
                expected
            );
            assert!(
                memory.data(&store)[start..start + 12].iter().all(|byte| *byte == 0),
                "failed String output stays canonical zero"
            );
        } else {
            let packed = raw
                .call(&mut store, (size, 1))
                .expect("declared packed helper result")
                .cast_unsigned();
            assert_eq!(packed >> 32, u64::try_from(expected).expect("nonnegative declared status"));
            assert_eq!(packed & u64::from(u32::MAX), 0, "failed pointer output is zero");
        }
        assert_eq!(
            global(&instance, &mut store, "$probe_status"),
            if expected == 1 { 2 } else { 3 }
        );
        assert_eq!(global(&instance, &mut store, "$probe_heap"), heap);
        assert_eq!(global(&instance, &mut store, "$probe_acquisitions"), acquisitions);
        assert_eq!(global(&instance, &mut store, "$probe_live"), 0);
        assert_eq!(global(&instance, &mut store, "$probe_attempts"), attempts + 1);
        assert_eq!(entry.call(&mut store, ()).expect("genuine pristine source retry"), 14);
    }
}
