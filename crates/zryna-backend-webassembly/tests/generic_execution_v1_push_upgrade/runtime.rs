//! Frozen native/source requirements projected only onto the existing Wasm allocation fault seam.
use serde_json::Value;
use sha2::{Digest, Sha256};
fn expected(case: &str) -> (i32, Value) {
    let native = include_bytes!("NATIVE_RUNTIME_EXPECTED.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(native)),
        "760338945f7103ed102fb4aca8d2aaf3aae9704cada1c313be433b376c60610f"
    );
    let native: Value =
        serde_json::from_slice(native).expect("unchanged authoritative native projection");
    let native = native["cases"]
        .as_array()
        .expect("three frozen source cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("actual original case");
    let source = super::source::row(case);
    assert_eq!(native["observation"], source["expected"]["result"]);
    let projection = include_bytes!("WASM_RUNTIME_EXPECTED.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(projection)),
        "010f87fe8fde10349448a6709669b988c845772c1322c6a8a7166ab45c23a099"
    );
    let projection: Value = serde_json::from_slice(projection).expect("pre-test Wasm projection");
    let projection = projection["cases"]
        .as_array()
        .expect("three projected cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same exact original case")
        .clone();
    assert_eq!(projection["result"], native["observation"]);
    (
        i32::try_from(projection["result"].as_i64().expect("frozen i32 result"))
            .expect("bounded source result"),
        projection,
    )
}
fn instantiate(bytes: &[u8], observed: bool) -> (wasmtime::Store<()>, wasmtime::Instance) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, bytes).expect("actual audited core Wasm");
    assert_eq!(module.imports().len(), 0);
    let exports = module.exports().map(|export| export.name()).collect::<Vec<_>>();
    assert_eq!(exports, if observed { vec!["entry", "$zryna$observation"] } else { vec!["entry"] });
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("private owned memory");
    (store, instance)
}
pub(super) fn ordinary(case: &str, bytes: &[u8]) {
    let (expected, _) = expected(case);
    let (mut store, instance) = instantiate(bytes, false);
    let entry =
        instance.get_typed_func::<(), i32>(&mut store, "entry").expect("exact scalar entry");
    for _ in 0..3 {
        assert_eq!(entry.call(&mut store, ()).expect("cleanup before pristine return"), expected);
    }
}
pub(super) fn observed_existing_allocation_faults(case: &str, bytes: &[u8]) {
    let (expected, projection) = expected(case);
    let (mut store, instance) = instantiate(bytes, true);
    let entry =
        instance.get_typed_func::<(), i32>(&mut store, "entry").expect("literal source entry");
    let observe = instance
        .get_typed_func::<i32, i32>(&mut store, "$zryna$observation")
        .expect("explicit existing observation policy");
    assert_eq!(entry.call(&mut store, ()).expect("pristine baseline"), expected);
    let mut calls = 1;
    let mut faults = 0;
    for fault in projection["faults"].as_array().expect("frozen native-derived faults") {
        // New count/reserve-status fault kinds remain a reviewable protocol plan only.
        if fault["kind"] != 2 {
            continue;
        }
        let ordinal =
            u32::try_from(fault["ordinal"].as_u64().expect("original allocation ordinal"))
                .expect("bounded fixture ordinal");
        let configure = 0x2200_0000_u32 | ordinal;
        assert_eq!(
            observe
                .call(&mut store, configure.cast_signed())
                .expect("configure allocation failure"),
            0
        );
        let failure =
            entry.call(&mut store, ()).expect_err("no result published on failed acquisition");
        assert_eq!(
            failure.downcast_ref::<wasmtime::Trap>(),
            Some(&wasmtime::Trap::UnreachableCodeReached)
        );
        assert_eq!(observe.call(&mut store, 0).expect("actual canonical trap category"), 2);
        assert_eq!(
            observe.call(&mut store, 0x2000_0001).expect("reset proves no live owner remains"),
            0
        );
        assert_eq!(entry.call(&mut store, ()).expect("immediate pristine retry"), expected);
        calls += 2;
        faults += 1;
    }
    assert_eq!((calls, faults), if case == "vec-push" { (5, 2) } else { (3, 1) });
    println!(
        "Wasm {case}: {calls} actual calls, {faults} allocation faults; status/identity extensions remain separate gates"
    );
}
