//! Mutations of actual production-compiled source fail its new runtime object's strict audit.

use super::*;
use object::{Object as _, ObjectSection as _, ObjectSymbol as _};

fn word(bytes: &[u8], offset: usize, width: usize) -> u64 {
    let mut value = [0; 8];
    value[..width].copy_from_slice(&bytes[offset..offset + width]);
    u64::from_le_bytes(value)
}

fn put(bytes: &mut [u8], offset: usize, width: usize, value: u64) {
    bytes[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
}

#[test]
fn actual_generic_runtime_requires_strong_exact_abi_natural_registry_and_nx() {
    with_host(|root, tools| {
        source::with_program("mixed", |program| {
            let mir =
                zryna_native_mir::generic_execution_v1::lower(program).expect("actual native MIR");
            let target = zryna_backend_native::select_object_target(
                zryna_backend_native::NATIVE_OBJECT_TARGET,
            )
            .expect("fixed ordinary target");
            let ordinary = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
                .expect("audited ordinary object");
            let source = crate::generic_runtime_v1::RuntimeSource::issue(program, &mir, &ordinary)
                .expect("actual same-owner material");
            let stage = crate::native::NativeStage::create(root, "generic-runtime-controls")
                .expect("production private stage");
            stage
                .write_input(&stage.runtime_source, source.bytes())
                .expect("typed source material");
            let dependencies =
                dependencies::Dependencies::capture(&stage, tools, NativeProcessLimits::default())
                    .expect("captured GNU capability");
            let carrier = zryna_backend_native::generic_execution_v1::emit_invocation(
                &mir,
                Invocation::new("entry".into(), vec![]),
                target,
            )
            .expect("typed same-owner carrier");
            let harness =
                harness::render(carrier.invocation().export().result()).expect("fixed harness");
            stage.write_input(&stage.object, carrier.bytes()).expect("typed carrier bytes");
            stage.write_input(&stage.harness, &harness).expect("fixed harness bytes");
            let mut context = process::Context::capture(
                &stage,
                &dependencies,
                NativeProcessLimits::default(),
                process::ExpectedInputs {
                    program: carrier.bytes(),
                    runtime: source.bytes(),
                    harness: &harness,
                },
            )
            .expect("exact written input identity and bytes");
            let runtime = context.compile_runtime().expect("actual production compile route");
            audit_runtime_object(&source, &runtime).expect("actual exact runtime object");
            mutations(&source, &runtime);
            audit_runtime_object(&source, &runtime).expect("unchanged pristine recovery");
            assert!(stage.cleanup().is_empty());
        });
    });
}

fn mutations(source: &crate::generic_runtime_v1::RuntimeSource<'_, '_, '_>, runtime: &[u8]) {
    let file = object::File::parse(runtime).expect("actual ELF object");
    let headers = usize::try_from(word(runtime, 40, 8)).expect("bounded header offset");
    let section =
        |name| headers + file.section_by_name(name).expect("required section").index().0 * 64;
    for (offset, width, value) in [
        (section(".bss") + 48, 8, 32),
        (section(".bss") + 32, 8, 32760),
        (section(".bss") + 8, 8, 0x403),
        (section(".note.GNU-stack") + 8, 8, 4),
    ] {
        let mut changed = runtime.to_vec();
        put(&mut changed, offset, width, value);
        assert!(audit_runtime_object(source, &changed).is_err());
    }
    let table = file.section_by_name(".symtab").expect("actual symbol table");
    let offset = usize::try_from(table.file_range().expect("file-backed symbols").0)
        .expect("bounded offset");
    let symbol = file
        .symbols()
        .find(|symbol| symbol.name() == Ok("zryna_rt_o1_allocate"))
        .expect("canonical strong definition");
    let mut weak = runtime.to_vec();
    weak[offset + symbol.index().0 * 24 + 4] = 0x22;
    assert!(audit_runtime_object(source, &weak).is_err());
}
