//! Portable Vec subset: empty frontier; positive/reserve zero+small only, no arena assumptions.
#[path = "../../../../zryna-driver/tests/generic_execution_v1_native_accounting_vec/source.rs"]
mod source;
use sha2::{Digest, Sha256};
#[test]
fn empty_vec_status_frontier_exact_extra_and_same_instance_retry() {
    actual("empty-vec-call-loop", true);
}
#[test]
fn positive_vec_small_rows_preserve_cleanup_without_native_threshold_assumption() {
    actual("nonempty-vec-call-loop", false);
}
#[test]
fn first_growth_and_within_capacity_reserve_small_rows_use_canonical_storage() {
    actual("reserve-vec-call-loop", false);
}
fn actual(case: &str, full: bool) {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../zryna-backend-javascript/src/generic_execution_v1/vec_accounting_tests/RUNTIME_EXPECTED_FREEZE.json"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "74e37c21295455022586c9115c6c1ed3b9e6975c1864304241660d5b55949a64"
    );
    let oracle: serde_json::Value = serde_json::from_slice(bytes).expect("unchanged15 rows");
    let rows = &oracle["cases"]
        .as_array()
        .expect("three sources")
        .iter()
        .find(|row| row["case"] == case)
        .expect("exact case")["rows"];
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifact =
                crate::generic_execution_v1::emit(program).expect("genuine ordinary Vec source");
            assert_eq!(
                artifact.bytes(),
                crate::generic_execution_v1::emit(program).expect("deterministic replay").bytes()
            );
            let directory = std::env::temp_dir()
                .join(format!("m416-wasm-vec-accounting-{}-{case}-{provider}", std::process::id()));
            std::fs::create_dir(&directory).expect("new evidence");
            std::fs::write(directory.join("program.wasm"), artifact.bytes())
                .expect("retain audited bytes");
            std::fs::write(directory.join("program.zir"), wire).expect("same NEW seal");
            let mut config = wasmtime::Config::new();
            config.epoch_interruption(true);
            let engine = wasmtime::Engine::new(&config).expect("host engine");
            let module = wasmtime::Module::new(&engine, artifact.bytes()).expect("actual module");
            assert_eq!(module.imports().len(), 0);
            assert_eq!(
                module.exports().map(|export| export.name()).collect::<Vec<_>>(),
                vec!["entry"]
            );
            let mut store = wasmtime::Store::new(&engine, ());
            store.set_epoch_deadline(1);
            let instance =
                wasmtime::Instance::new(&mut store, &module, &[]).expect("ordinary instance");
            let entry =
                instance.get_typed_func::<i32, i32>(&mut store, "entry").expect("scalar entry");
            let _deadline = super::deadline::Deadline::new(engine);
            let mut actual = Vec::new();
            let mut selected = 0;
            for row in rows
                .as_array()
                .expect("five rows")
                .iter()
                .filter(|row| full || row["label"] == "zero" || row["label"] == "small")
            {
                selected += 1;
                let arg = i32::try_from(row["arguments"][0].as_i64().expect("fixed argument"))
                    .expect("i32 argument");
                for repetition in 0..row["invocations_same_process"].as_u64().expect("repeats") {
                    let result = entry.call(&mut store, arg);
                    actual.push(serde_json::json!({"row":row["label"],"argument":arg,"repetition":repetition,"returned":result.as_ref().ok().copied(),"trap":result.as_ref().err().and_then(|error|error.downcast_ref::<wasmtime::Trap>()).map(|trap|format!("{trap:?}")),"error":result.as_ref().err().map(ToString::to_string)}));
                    std::fs::write(
                        directory.join("actual-before-checks.json"),
                        serde_json::to_vec_pretty(&actual).expect("summary"),
                    )
                    .expect("retain before assertions");
                    if row["outcome"] == "return" {
                        assert_eq!(
                            result.expect("normal source return"),
                            i32::try_from(row["scalar_result"].as_i64().expect("frozen result"))
                                .expect("i32 result")
                        );
                    } else {
                        let failure = result.expect_err("first extra traps after cleanup");
                        assert_eq!(
                            failure.downcast_ref::<wasmtime::Trap>(),
                            Some(&wasmtime::Trap::UnreachableCodeReached),
                            "host interruption is not source CAPACITY"
                        );
                        let retry = entry.call(&mut store, 1);
                        actual.push(serde_json::json!({"row":"pristine-retry","argument":1,"returned":retry.as_ref().ok().copied(),"error":retry.as_ref().err().map(ToString::to_string)}));
                        std::fs::write(
                            directory.join("actual-before-checks.json"),
                            serde_json::to_vec_pretty(&actual).expect("retry summary"),
                        )
                        .expect("retain retry before assertion");
                        assert_eq!(retry.expect("pristine same instance"), 1);
                    }
                }
            }
            assert_eq!(selected, if full { 5 } else { 2 });
        });
    }
}

mod private_vec;

pub(super) fn with_program(
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    source::with_program("reserve-vec-call-loop", "native", check);
}
