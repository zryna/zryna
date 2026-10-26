//! Existing empty-String status frontier on genuine audited ordinary Wasm bytes.
#[path = "../../../zryna-driver/tests/generic_execution_v1_native_accounting_string/source.rs"]
mod source;
use sha2::{Digest, Sha256};
mod deadline;
#[test]
fn empty_string_status_exact_extra_and_same_instance_retry() {
    let oracle = include_bytes!(
        "../../../zryna-driver/tests/generic_execution_v1_native_accounting_string/RUNTIME_EXPECTED_FREEZE.json"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "fe2636923a20fd8ca2c6fb176aedc0eaf91d607bcd584555be335c38c4dee490"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(oracle).expect("unchanged source-law oracle");
    for provider in source::PROVIDERS {
        source::with_program("empty-call-loop", provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let artifact = super::emit(program).expect("genuine ordinary new seal");
            assert_eq!(
                artifact.bytes(),
                super::emit(program).expect("deterministic replay").bytes()
            );
            let directory = std::env::temp_dir()
                .join(format!("m416-wasm-empty-accounting-{}-{provider}", std::process::id()));
            std::fs::create_dir(&directory).expect("new actual evidence directory");
            std::fs::write(directory.join("program.wasm"), artifact.bytes())
                .expect("retain actual bytes");
            std::fs::write(directory.join("program.zir"), wire).expect("same new seal");
            let mut config = wasmtime::Config::new();
            config.epoch_interruption(true);
            let engine = wasmtime::Engine::new(&config).expect("host containment engine");
            let module =
                wasmtime::Module::new(&engine, artifact.bytes()).expect("actual audited bytes");
            assert_eq!(module.imports().len(), 0);
            assert_eq!(module.exports().map(|e| e.name()).collect::<Vec<_>>(), vec!["entry"]);
            let mut store = wasmtime::Store::new(&engine, ());
            store.set_epoch_deadline(1);
            let instance =
                wasmtime::Instance::new(&mut store, &module, &[]).expect("ordinary owned memory");
            let entry =
                instance.get_typed_func::<i32, i32>(&mut store, "entry").expect("scalar ABI");
            let _deadline = deadline::Deadline::new(engine.clone());
            let mut actual = Vec::new();
            for row in oracle["runs"]
                .as_array()
                .expect("fourteen frozen rows")
                .iter()
                .filter(|r| r["case"] == "empty-call-loop")
            {
                let arg = i32::try_from(
                    row["entry_arguments_per_invocation"][0].as_i64().expect("fixed i32"),
                )
                .expect("i32 range");
                for repetition in
                    0..row["invocations_in_same_process"].as_u64().expect("fixed repeats")
                {
                    let result = entry.call(&mut store, arg);
                    let returned = result.as_ref().ok().copied();
                    let trap = result
                        .as_ref()
                        .err()
                        .and_then(|e| e.downcast_ref::<wasmtime::Trap>())
                        .map(|t| format!("{t:?}"));
                    actual.push(serde_json::json!({"row":row["id"],"argument":arg,"repetition":repetition,"returned":returned,"trap":trap,"error":result.as_ref().err().map(ToString::to_string)}));
                    std::fs::write(
                        directory.join("actual-before-checks.json"),
                        serde_json::to_vec_pretty(&actual).expect("summary"),
                    )
                    .expect("retain actual before assertions");
                    if row["expected_per_invocation"]["source_failure_status"] == 0 {
                        assert_eq!(result.expect("actual ordinary normal return"), arg);
                    } else {
                        let failure = result.expect_err("first extra must trap after cleanup");
                        assert_eq!(
                            failure.downcast_ref::<wasmtime::Trap>(),
                            Some(&wasmtime::Trap::UnreachableCodeReached),
                            "engine interruption is host containment, never source CAPACITY"
                        );
                        let retry = entry.call(&mut store, 3);
                        actual.push(serde_json::json!({"row":"pristine-retry-after-denial","argument":3,"returned":retry.as_ref().ok().copied(),"error":retry.as_ref().err().map(ToString::to_string)}));
                        std::fs::write(
                            directory.join("actual-before-checks.json"),
                            serde_json::to_vec_pretty(&actual).expect("summary"),
                        )
                        .expect("retain retry before assertion");
                        assert_eq!(retry.expect("same instance pristine retry"), 3);
                    }
                }
            }
        });
    }
}

mod private_controls;

mod vectors;

pub(super) fn vec_program(
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    vectors::with_program(check);
}

mod controls;
pub(super) fn control_program(
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    controls::with_program("control-live-count-call-loop", "native", check);
}
