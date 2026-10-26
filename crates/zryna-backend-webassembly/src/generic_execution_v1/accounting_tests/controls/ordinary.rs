//! Public ordinary source rows do not assume native million-allocation success in sixteen MiB.
use super::source;
use crate::generic_execution_v1::{self as target, accounting_tests::deadline::Deadline};
#[test]
fn birth_public_zero_small_and_same_instance_reset() {
    actual("control-birth-call-loop", 7);
}
#[test]
fn live_counts_public_zero_small_and_same_instance_reset() {
    actual("control-live-count-call-loop", 31);
}
#[test]
fn expired_public_zero_small_and_same_instance_reset() {
    actual("control-expired-call-loop", 37);
}
fn actual(case: &str, scalar: i32) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifact =
                target::emit(program).expect("actual ordinary whole-seal control target");
            assert_eq!(
                artifact.bytes(),
                target::emit(program).expect("deterministic ordinary bytes").bytes()
            );
            let mut config = wasmtime::Config::new();
            config.epoch_interruption(true);
            let engine = wasmtime::Engine::new(&config).expect("host containment engine");
            let module =
                wasmtime::Module::new(&engine, artifact.bytes()).expect("actual audited module");
            assert_eq!(module.imports().len(), 0);
            assert_eq!(module.exports().map(|e| e.name()).collect::<Vec<_>>(), vec!["entry"]);
            let mut store = wasmtime::Store::new(&engine, ());
            store.set_epoch_deadline(1);
            let instance = wasmtime::Instance::new(&mut store, &module, &[])
                .expect("ordinary private memory instance");
            let entry = instance
                .get_typed_func::<i32, i32>(&mut store, "entry")
                .expect("actual scalar ABI");
            let directory = std::env::temp_dir()
                .join(format!("m416-wasm-control-public-{}-{case}-{provider}", std::process::id()));
            std::fs::create_dir(&directory).expect("new evidence directory");
            std::fs::write(directory.join("program.wasm"), artifact.bytes())
                .expect("audited bytes");
            std::fs::write(directory.join("program.zir"), wire).expect("genuine unchanged wire");
            let _deadline = Deadline::new(engine);
            let mut actual = Vec::new();
            for arg in [0, 3, 3] {
                let result = entry.call(&mut store, arg);
                actual.push(serde_json::json!({"argument":arg,"returned":result.as_ref().ok().copied(),"error":result.as_ref().err().map(ToString::to_string),"trap":result.as_ref().err().and_then(|e|e.downcast_ref::<wasmtime::Trap>()).map(|t|format!("{t:?}"))}));
                std::fs::write(
                    directory.join("actual-before-checks.json"),
                    serde_json::to_vec_pretty(&actual).expect("actual observations"),
                )
                .expect("retain before checks");
                assert_eq!(result.expect("actual ordinary normal return"), arg * scalar);
            }
        });
    }
}
