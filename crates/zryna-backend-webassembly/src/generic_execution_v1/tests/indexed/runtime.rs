//! Execute actual public bytes under the unchanged scalar and explicitly limited observer projection.
use serde_json::Value;
use sha2::{Digest, Sha256};
#[path = "deadline.rs"]
mod deadline;
mod trace;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn check(case: &str, provider: &str, wire: &[u8], bytes: &[u8], observed: bool) {
    let frozen = include_bytes!("WASM_OBSERVER_EXPECTED_FREEZE.json");
    assert_eq!(digest(frozen), "798ccbc7f15d67ef24b8a9a1b5b96871cde86132d28d3913263b8315041e692b");
    let oracle: Value =
        serde_json::from_slice(frozen).expect("pre-output source-only Wasm projection");
    let row = oracle["cases"]
        .as_array()
        .expect("twelve cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same case");
    let source = include_bytes!(
        "../../../../../zryna-driver/tests/generic_execution_v1_native_indexed_runtime/OBSERVED_EXPECTED_FREEZE.json"
    );
    assert_eq!(digest(source), "e4ffbd6b69f42d8d023fa9265b51a3bc1b08cebdc679caaa0cd3bb852032bad3");
    let source: Value = serde_json::from_slice(source).expect("unchanged source oracle");
    let source = source["cases"]
        .as_array()
        .expect("source cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same source");
    assert_eq!(row["sourceSha256"], source["sourceSha256"]);
    let scalar_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../zryna-backend-javascript/src/generic_execution_v1/indexed_tests/RUNTIME_EXPECTED_FREEZE.json"
    ));
    assert_eq!(
        digest(scalar_bytes),
        "d8115763d70c5f3d804f4756fb3aa6ca34da58f8f7242ebaba7c7f356a25c8d5"
    );
    let scalar: Value =
        serde_json::from_slice(scalar_bytes).expect("unchanged scalar/bounds oracle");
    let scalar = scalar["cases"]
        .as_array()
        .expect("scalar source cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same source case");
    let normal = row["normal"].as_array().expect("projection normal");
    let scalar = scalar["observations"].as_array().expect("original observations");
    assert_eq!(normal.len(), scalar.len());
    for (a, b) in normal.iter().zip(scalar) {
        assert_eq!(a["arguments"], b["arguments"]);
        assert_eq!(a["result"], b["result"]);
        assert_eq!(a["trap"], b["trap"]);
    }
    let directory = std::env::temp_dir()
        .join(format!("m416-wasm-indexed-{}-{case}-{provider}-{observed}", std::process::id()));
    std::fs::create_dir(&directory).expect("new actual evidence directory");
    std::fs::write(directory.join("program.wasm"), bytes).expect("audited public module");
    std::fs::write(directory.join("program.zir"), wire).expect("genuine issuer wire");
    let mut config = wasmtime::Config::new();
    config.epoch_interruption(true);
    let engine = wasmtime::Engine::new(&config).expect("host engine");
    let module = wasmtime::Module::new(&engine, bytes).expect("actual audited bytes");
    assert_eq!(module.imports().len(), 0);
    assert_eq!(
        module.exports().map(|export| export.name()).collect::<Vec<_>>(),
        if observed { vec!["entry", "$zryna$observation"] } else { vec!["entry"] }
    );
    let mut store = wasmtime::Store::new(&engine, ());
    store.set_epoch_deadline(1);
    let instance =
        wasmtime::Instance::new(&mut store, &module, &[]).expect("defined private memory");
    let _deadline = deadline::Deadline::new(engine);
    let mut actual = Vec::new();
    for repeat in 0..2 {
        for run in row["normal"].as_array().expect("frozen source rows") {
            run_one(
                &mut store,
                &instance,
                run,
                observed,
                &directory,
                &mut actual,
                &format!("normal:{repeat}"),
            );
        }
    }
    if observed {
        for run in row["faults"].as_array().expect("unchanged allocation faults") {
            run_one(&mut store, &instance, run, true, &directory, &mut actual, "allocation-fault");
            let pristine = row["normal"]
                .as_array()
                .expect("normal rows")
                .iter()
                .find(|normal| normal["arguments"] == run["arguments"])
                .expect("same argument pristine row");
            run_one(
                &mut store,
                &instance,
                pristine,
                true,
                &directory,
                &mut actual,
                "pristine-after-fault",
            );
        }
    }
    std::fs::write(directory.join("complete.json"),serde_json::to_vec_pretty(&serde_json::json!({"case":case,"provider":provider,"observed":observed,"sourceSha256":row["sourceSha256"],"moduleSha256":digest(bytes),"calls":actual.len(),"complete":true,"observer_boundary":oracle["boundary"]})).expect("receipt")).expect("retain completion");
}
fn invoke(
    store: &mut wasmtime::Store<()>,
    instance: &wasmtime::Instance,
    run: &Value,
) -> wasmtime::Result<i32> {
    let args = run["arguments"]
        .as_array()
        .expect("original scalar ABI inputs")
        .iter()
        .map(|v| {
            wasmtime::Val::I32(v.as_bool().map_or_else(
                || i32::try_from(v.as_i64().expect("i32 source arg")).expect("bounded source arg"),
                i32::from,
            ))
        })
        .collect::<Vec<_>>();
    let entry = instance.get_func(&mut *store, "entry").expect("source entry");
    let mut results = [wasmtime::Val::I32(0)];
    entry.call(&mut *store, &args, &mut results)?;
    Ok(results[0].i32().expect("original i32 result"))
}
fn run_one(
    store: &mut wasmtime::Store<()>,
    instance: &wasmtime::Instance,
    run: &Value,
    observed: bool,
    directory: &std::path::Path,
    actual: &mut Vec<Value>,
    label: &str,
) {
    let getter = if observed {
        Some(
            instance
                .get_typed_func::<i32, i32>(&mut *store, "$zryna$observation")
                .expect("existing private observer"),
        )
    } else {
        None
    };
    if let Some(getter) = &getter {
        let ordinal =
            run["fault"]["ordinal"].as_u64().map(|x| u32::try_from(x).expect("bounded fault"));
        let selector = ordinal.map_or(0x2000_0001, |ordinal| 0x2200_0000 | ordinal);
        assert_eq!(
            getter
                .call(&mut *store, selector.cast_signed())
                .expect("configure existing allocation selector after full cleanup"),
            0
        );
    }
    let outcome = invoke(store, instance, run);
    actual.push(serde_json::json!({"label":label,"arguments":run["arguments"],"fault":run["fault"],"returned":outcome.as_ref().ok().copied(),"trap":outcome.as_ref().err().and_then(|error|error.downcast_ref::<wasmtime::Trap>()).map(|trap|format!("{trap:?}")),"error":outcome.as_ref().err().map(ToString::to_string)}));
    retain(directory, actual);
    if let Some(getter) = &getter {
        let (category, words) = trace::snapshot(store, getter);
        actual.last_mut().expect("actual row")["category"] = category.into();
        actual.last_mut().expect("actual row")["traceWords"] = serde_json::json!(words);
        retain(directory, actual);
        assert_eq!(i64::from(category), run["category"].as_i64().expect("frozen category"));
        trace::check(&words, run);
        assert_eq!(
            getter
                .call(&mut *store, 0x2000_0001)
                .expect("all successful physical owners cleaned on exit"),
            0
        );
    }
    if let Some(value) = run.get("result") {
        assert_eq!(
            outcome.expect("source return"),
            i32::try_from(value.as_i64().expect("scalar result")).expect("i32 result")
        );
    } else {
        let error = outcome.expect_err("source bounds/allocation trap after cleanup");
        assert_eq!(
            error.downcast_ref::<wasmtime::Trap>(),
            Some(&wasmtime::Trap::UnreachableCodeReached),
            "host containment or engine faults cannot count as semantic traps"
        );
    }
}
fn retain(directory: &std::path::Path, actual: &[Value]) {
    std::fs::write(
        directory.join("actual-before-assertions.json"),
        serde_json::to_vec_pretty(actual).expect("bounded actual trace receipt"),
    )
    .expect("retain outcomes before assertions");
}
