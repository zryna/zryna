//! Actual NEW-owner staged material; mutations are rejected before affected tool use.
use super::{source, with_host};
use crate::native::NativeProcessLimits;
use crate::native::NativeStage;
use crate::native::generic_execution::{dependencies, harness, process};
use process::{Context, ExpectedInputs};
use std::fs;
use zryna_abi::Invocation;
fn with_inputs(
    inspect: impl FnOnce(&NativeStage, &dependencies::Dependencies, ExpectedInputs<'_>),
) {
    with_host(|root, tools| {
        source::with_program("mixed", |program| {
            let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("genuine MIR");
            let target = zryna_backend_native::select_object_target(
                zryna_backend_native::NATIVE_OBJECT_TARGET,
            )
            .expect("exact target");
            let ordinary = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
                .expect("ordinary owner");
            let carrier = zryna_backend_native::generic_execution_v1::emit_invocation(
                &mir,
                Invocation::new("entry".into(), vec![]),
                target,
            )
            .expect("genuine carrier");
            let material =
                crate::generic_runtime_v1::RuntimeSource::issue(program, &mir, &ordinary)
                    .expect("same-owner runtime");
            let harness =
                harness::render(carrier.invocation().export().result()).expect("fixed harness");
            let stage =
                NativeStage::create(root, "generic-input-controls").expect("held private stage");
            stage.write_input(&stage.object, carrier.bytes()).expect("carrier input");
            stage.write_input(&stage.runtime_source, material.bytes()).expect("runtime input");
            stage.write_input(&stage.harness, &harness).expect("harness input");
            let dependencies =
                dependencies::Dependencies::capture(&stage, tools, NativeProcessLimits::default())
                    .expect("actual GNU provider capture");
            inspect(
                &stage,
                &dependencies,
                ExpectedInputs {
                    program: carrier.bytes(),
                    runtime: material.bytes(),
                    harness: &harness,
                },
            );
            assert!(stage.cleanup().is_empty());
        });
    });
}
fn expected<'a>(bytes: ExpectedInputs<'a>, name: &str) -> &'a [u8] {
    match name {
        "program.o" => bytes.program,
        "runtime.c" => bytes.runtime,
        "invocation.c" => bytes.harness,
        _ => panic!("fixed written input"),
    }
}
fn replace(stage: &NativeStage, name: &str, bytes: &[u8]) {
    let path = stage.capability_file_path(name).expect("fixed input path");
    let permissions = fs::metadata(&path).expect("original metadata").permissions();
    let replacement = stage.capability_directory_path().join("replacement");
    fs::write(&replacement, bytes).expect("new live replacement inode");
    fs::set_permissions(&replacement, permissions).expect("same original mode");
    fs::rename(replacement, path).expect("same bytes distinct inode");
}
fn rejected(errors: &[zryna_diagnostics::Diagnostic]) {
    assert!(!errors.is_empty());
    assert!(errors.iter().all(|error| error.code == "ZRYNA-C4166"), "{errors:?}");
}

mod compiled;
mod written;
