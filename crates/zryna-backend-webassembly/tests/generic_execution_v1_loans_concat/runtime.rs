//! Current observer protocol only: exact String bytes/identities, failure cleanup and retry.
use serde_json::Value;
use sha2::{Digest, Sha256};
#[path = "runtime/trace.rs"]
mod trace;
fn expected(case: &str) -> (i32, Value, Value) {
    let original = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../zryna-backend-javascript/src/generic_execution_v1/loan_tests/RUNTIME_ORACLE.json"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(original)),
        "a772a260962503436a0d4fde7836f72db9a4fdc2857f79085b05f624a11d4264"
    );
    let original: Value =
        serde_json::from_slice(original).expect("unchanged source/native/JS oracle");
    let original = original["cases"]
        .as_array()
        .expect("thirteen fixed cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("actual source identity")
        .clone();
    let projection = include_bytes!("WASM_EXPECTED.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(projection)),
        "cdf3d4468d7618f1d69c69eca81e0e3a40e63186a3ac279a5e91837ece2ea36a"
    );
    let projection: Value =
        serde_json::from_slice(projection).expect("preimplementation target projection");
    let projection = projection["cases"]
        .as_array()
        .expect("thirteen projected cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same original case")
        .clone();
    let source = if super::LOANS.contains(&case) {
        super::loan_source::row(case)
    } else {
        super::concat_source::row(case)
    };
    assert_eq!(projection["result"], source["i32Result"]);
    assert_eq!(projection["result"], original["observation"]);
    let result = i32::try_from(projection["result"].as_i64().expect("frozen result"))
        .expect("literal i32 result");
    (result, projection, original)
}
fn instantiate(bytes: &[u8], observed: bool) -> (wasmtime::Store<()>, wasmtime::Instance) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, bytes).expect("actual fully audited Wasm");
    assert_eq!(module.imports().len(), 0);
    let exports = module.exports().map(|export| export.name()).collect::<Vec<_>>();
    assert_eq!(exports, if observed { vec!["entry", "$zryna$observation"] } else { vec!["entry"] });
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("private fixed memory");
    (store, instance)
}
pub(super) fn ordinary(case: &str, bytes: &[u8]) {
    let (expected, _, _) = expected(case);
    let (mut store, instance) = instantiate(bytes, false);
    let entry =
        instance.get_typed_func::<(), i32>(&mut store, "entry").expect("original scalar export");
    for _ in 0..3 {
        assert_eq!(entry.call(&mut store, ()).expect("ordinary reentry"), expected);
    }
}
pub(super) fn observed(case: &str, bytes: &[u8]) {
    let (expected, projection, original) = expected(case);
    let (mut store, instance) = instantiate(bytes, true);
    let entry = instance.get_typed_func::<(), i32>(&mut store, "entry").expect("exact entry");
    let observe = instance
        .get_typed_func::<i32, i32>(&mut store, "$zryna$observation")
        .expect("distinct current test-only observation policy");
    assert_eq!(entry.call(&mut store, ()).expect("pristine actual result"), expected);
    trace::check(&mut store, &observe, &original, None);
    for fault in projection["faults"].as_array().expect("frozen fault projection") {
        let kind =
            u32::try_from(fault["kind"].as_u64().expect("existing selector")).expect("small kind");
        assert!([2, 6].contains(&kind), "no protocol extension");
        let ordinal = u32::try_from(fault["ordinal"].as_u64().expect("fixed ordinal"))
            .expect("small ordinal");
        let source_fault = original["faults"]
            .as_array()
            .expect("unchanged original faults")
            .iter()
            .find(|f| {
                f["ordinal"] == fault["ordinal"]
                    && f["kind"] == if kind == 2 { "allocation" } else { "stringClone" }
            })
            .expect("target fault is an exact original projection");
        let config = 0x2000_0000_u32 | (kind << 24) | ordinal;
        assert_eq!(
            observe.call(&mut store, config.cast_signed()).expect("configure existing fault"),
            0
        );
        let failure =
            entry.call(&mut store, ()).expect_err("failed acquisition publishes no result");
        assert_eq!(
            failure.downcast_ref::<wasmtime::Trap>(),
            Some(&wasmtime::Trap::UnreachableCodeReached)
        );
        assert_eq!(observe.call(&mut store, 0).expect("actual canonical trap category"), 2);
        trace::check(&mut store, &observe, &original, Some(source_fault));
        assert_eq!(observe.call(&mut store, 0x2000_0001).expect("reset requires no live owner"), 0);
        assert_eq!(entry.call(&mut store, ()).expect("immediate pristine retry"), expected);
        trace::check(&mut store, &observe, &original, None);
    }
}
