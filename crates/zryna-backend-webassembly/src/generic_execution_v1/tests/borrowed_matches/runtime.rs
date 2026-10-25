//! Execute actual ordinary/observed bytes against the unchanged pre-output target oracle.
use serde_json::Value;
use sha2::{Digest, Sha256};
mod trace;
pub(super) fn check(case: &str, bytes: &[u8], observed: bool) {
    let oracle = include_bytes!(
        "../../../../../zryna-driver/tests/generic_execution_v1_borrowed_match_targets/RUNTIME_EXPECTED.json"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "d743e826942ba7b1f2a8cd064d332a5218c87e9e2fbd964aefcc88663fa1c1ec"
    );
    let oracle: Value = serde_json::from_slice(oracle).expect("unchanged pre-output oracle");
    let row = oracle["cases"]
        .as_array()
        .expect("eleven original cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same genuine source");
    assert_eq!(row["source_sha256"], super::source::row(case)["sha256"]);
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, bytes).expect("actual audited Wasm bytes");
    assert_eq!(module.imports().len(), 0);
    let exports = module.exports().map(|e| e.name()).collect::<Vec<_>>();
    assert_eq!(exports, if observed { vec!["entry", "$zryna$observation"] } else { vec!["entry"] });
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("private owned memory");
    for run in row["runs"].as_array().expect("frozen scalar argument rows") {
        if observed {
            observe(&mut store, &instance, run);
        } else {
            for _ in 0..3 {
                success(&mut store, &instance, run);
            }
        }
    }
}
fn invoke(
    store: &mut wasmtime::Store<()>,
    instance: &wasmtime::Instance,
    run: &Value,
) -> wasmtime::Result<i32> {
    let arguments = run["arguments"]
        .as_array()
        .expect("frozen scalar ABI arguments")
        .iter()
        .map(|value| {
            wasmtime::Val::I32(value.as_bool().map_or_else(
                || i32::try_from(value.as_i64().expect("frozen i32 argument")).expect("i32 range"),
                i32::from,
            ))
        })
        .collect::<Vec<_>>();
    let entry = instance.get_func(&mut *store, "entry").expect("source scalar export");
    let mut results = [wasmtime::Val::I32(0)];
    entry.call(&mut *store, &arguments, &mut results)?;
    Ok(results[0].i32().expect("original i32 result ABI"))
}
fn success(store: &mut wasmtime::Store<()>, instance: &wasmtime::Instance, run: &Value) {
    let expected =
        i32::try_from(run["pristine"]["scalar_result"].as_i64().expect("original result"))
            .expect("source i32 result");
    assert_eq!(invoke(store, instance, run).expect("actual pristine scalar return"), expected);
}
fn observe(store: &mut wasmtime::Store<()>, instance: &wasmtime::Instance, run: &Value) {
    let getter = instance
        .get_typed_func::<i32, i32>(&mut *store, "$zryna$observation")
        .expect("separate existing observation policy");
    assert_eq!(getter.call(&mut *store, 0x2000_0001).expect("reset no live roots"), 0);
    success(store, instance, run);
    trace::check(store, &getter, run, None);
    for fault in run["faults"].as_array().expect("unchanged fault rows") {
        let kind = match fault["kind"].as_str().expect("canonical failure kind") {
            "allocation" => 2_u32,
            "stringClone" => 6,
            _ => panic!("no new observer fault protocol"),
        };
        let ordinal = u32::try_from(fault["ordinal"].as_u64().expect("actual ordinal"))
            .expect("bounded ordinal");
        let selector = 0x2000_0000_u32 | (kind << 24) | ordinal;
        assert_eq!(getter.call(&mut *store, selector.cast_signed()).expect("configure fault"), 0);
        let failure = invoke(store, instance, run).expect_err("unpublished failed result traps");
        assert_eq!(
            failure.downcast_ref::<wasmtime::Trap>(),
            Some(&wasmtime::Trap::UnreachableCodeReached)
        );
        trace::check(store, &getter, run, Some(fault));
        assert_eq!(
            getter.call(&mut *store, 0x2000_0001).expect("all failed-path roots released"),
            0
        );
        success(store, instance, run);
        trace::check(store, &getter, run, None);
    }
}
