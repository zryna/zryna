//! Test-only controlled packed failures; original source owners and wrapper bytes remain binding.
use super::{
    Invocation, Outcome, ScalarType, ScalarValue, decode_packed, emit_invocation, indexed,
};
use crate::generic_execution_v1::{invocation::ValidatedInvocationArtifact, model::Model};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, raw::TrapIdentity};
mod instrument;
mod source;
mod transport;
fn oracle() -> Value {
    let bytes = include_bytes!("controlled/EXPECTED_BEFORE_EXECUTION.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "d44fca929d9ea1725916141c3f28992f598a99c1ce7e304dcf1a40217cac80b3"
    );
    serde_json::from_slice(bytes).expect("frozen source/role expectations before guest output")
}
#[test]
fn five_private_controlled_source_statuses_pack_after_nonempty_string_cleanup_and_retry() {
    let expected = oracle();
    assert_eq!(expected["logical_call_census"], 15);
    assert_eq!(expected["physical_packed_calls"], 27);
    let mut instances = 0;
    for row in expected["rows"].as_array().expect("five frozen rows") {
        if row["name"] == "refcount" {
            source::with_mixed(|program| run(program, row, "typescript"));
            instances += 1;
        } else {
            for provider in indexed::PROVIDERS {
                indexed::with_program("vec-shared-clone", provider, |program, _, _| {
                    run(program, row, provider);
                });
                instances += 1;
            }
        }
    }
    assert_eq!(instances, 9);
}
fn run(program: &VerifiedExecution<'_>, row: &Value, provider: &str) {
    let arguments = if row["name"] == "refcount" { vec![] } else { vec![ScalarValue::I32(0)] };
    let carrier = emit_invocation(program, Invocation::new("entry".into(), arguments))
        .expect("actual original typed request, same NEW execution owner");
    carrier.audit_bytes(carrier.as_bytes()).expect("complete actual immutable carrier audit");
    super::super::with_model(program, carrier.invocation(), |model, _| {
        execute(&carrier, model, row, provider);
        Ok(())
    })
    .expect("same-owner source-bound private test model");
}
fn execute(
    carrier: &ValidatedInvocationArtifact<'_, '_>,
    model: &Model<'_, '_, '_>,
    row: &Value,
    provider: &str,
) {
    let changed = instrument::build(carrier.as_bytes(), model, row);
    assert!(
        carrier.audit_bytes(&changed).is_err(),
        "untrusted private copies never carry production authority"
    );
    let directory = std::env::temp_dir().join(format!(
        "m416-wasm-packed-five-{}-{}-{provider}",
        std::process::id(),
        row["name"].as_str().expect("name")
    ));
    std::fs::create_dir(&directory).expect("fresh owning artifact retention");
    for (name, bytes) in [
        ("audited-carrier.wasm", carrier.as_bytes()),
        ("untrusted-instrumented.wasm", changed.as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes)
            .expect("retain exact actual material before compilation");
    }
    std::fs::write(
        directory.join("expected-before-execution.json"),
        serde_json::to_vec_pretty(row).expect("fixed oracle"),
    )
    .expect("retain before execution");
    std::fs::write(directory.join("material-before-compilation.json"),serde_json::to_vec_pretty(&serde_json::json!({"original_sha256":format!("{:x}",Sha256::digest(carrier.as_bytes())),"private_sha256":format!("{:x}",Sha256::digest(&changed)),"source_base":model.source_base,"raw_string_from":crate::generic_execution_v1::model::STRING_FROM,"raw_string_clone":crate::generic_execution_v1::model::STRING_CLONE,"raw_string_release":crate::generic_execution_v1::model::STRING_RELEASE,"raw_strong_clone":model.container_base+crate::generic_execution_v1::model::containers::STRONG_CLONE,"production_authority":false})).expect("material+role bindings")).expect("retain actual material before compilation");
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = wasmtime::Engine::new(&config).expect("existing finite Wasmtime test host");
    let module = wasmtime::Module::new(&engine, &changed).expect("welltyped private module");
    assert_eq!(module.imports().len(), 0);
    assert_eq!(
        module.exports().map(|e| e.name()).collect::<Vec<_>>(),
        vec![carrier.entry_export(), "test$configure", "test$observe"]
    );
    let mut store = wasmtime::Store::new(&engine, ());
    store.set_fuel(10_000_000).expect("same finite fuel bank; never refilled");
    let instance =
        wasmtime::Instance::new(&mut store, &module, &[]).expect("private instantiation");
    let invoke = instance
        .get_typed_func::<(), i64>(&mut store, carrier.entry_export())
        .expect("unchanged packed invoker");
    let configure = instance
        .get_typed_func::<i32, i32>(&mut store, "test$configure")
        .expect("private control only");
    let observe = instance
        .get_typed_func::<i32, i32>(&mut store, "test$observe")
        .expect("read-only private projection");
    let mut observations = Vec::new();
    for (label, fault) in
        [("pristine", false), ("controlled-failure", true), ("pristine-retry", false)]
    {
        assert_eq!(
            configure.call(&mut store, i32::from(fault)).expect("only after full cleanup"),
            0
        );
        let (packed, snapshot) = observe_invocation(
            &directory,
            &mut store,
            &invoke,
            &observe,
            label,
            fault,
            &mut observations,
        );
        let expected = if fault {
            Outcome::Trap(identity(row["logical_status"].as_u64().expect("code")))
        } else {
            Outcome::Value(ScalarValue::I32(
                i32::try_from(row["pristine_value"].as_i64().expect("value")).expect("i32"),
            ))
        };
        assert_eq!(decode_packed(packed, ScalarType::I32), Ok(expected));
        assert_eq!(
            packed >> 32,
            if fault { row["logical_status"].as_u64().expect("code") } else { 0 }
        );
        if fault {
            assert_eq!(
                u32::try_from(packed & u64::from(u32::MAX)).expect("low32"),
                0,
                "no stale scalar on failure"
            );
        }
        check(&snapshot, row, fault);
    }
    carrier.audit_bytes(carrier.as_bytes()).expect("private test never mutates original authority");
}
fn retain(directory: &Path, observations: &[Value]) {
    std::fs::write(
        directory.join("actual-before-assertions.json"),
        serde_json::to_vec_pretty(observations).expect("bounded actual receipt"),
    )
    .expect("retain outcome before assertion");
}
fn observe_invocation(
    directory: &Path,
    store: &mut wasmtime::Store<()>,
    invoke: &wasmtime::TypedFunc<(), i64>,
    observe: &wasmtime::TypedFunc<i32, i32>,
    label: &str,
    fault: bool,
    observations: &mut Vec<Value>,
) -> (u64, Vec<u32>) {
    let result = invoke.call(&mut *store, ());
    let fuel_before = store.get_fuel();
    observations.push(serde_json::json!({
        "label":label,"private_seeded_fault":fault,
        "packed":result.as_ref().ok().map(|value|value.cast_unsigned()),
        "host_error":result.as_ref().err().map(ToString::to_string),
        "snapshot":null,"snapshot_errors":null,
        "remaining_fuel_before_observation":fuel_before.as_ref().ok(),
        "fuel_error_before_observation":fuel_before.as_ref().err().map(ToString::to_string)
    }));
    // Persist the invoker outcome even if every later read-only observation fails.
    retain(directory, observations);
    let snapshots = (0..28)
        .map(|index| observe.call(&mut *store, index).map(i32::cast_unsigned))
        .collect::<Vec<_>>();
    let fuel_after = store.get_fuel();
    let row = observations.last_mut().expect("just-retained invocation row");
    row["snapshot"] =
        serde_json::json!(snapshots.iter().map(|value| value.as_ref().ok()).collect::<Vec<_>>());
    row["snapshot_errors"] = serde_json::json!(
        snapshots
            .iter()
            .map(|value| value.as_ref().err().map(ToString::to_string))
            .collect::<Vec<_>>()
    );
    row["remaining_fuel_after_observation"] = serde_json::json!(fuel_after.as_ref().ok());
    row["fuel_error_after_observation"] =
        serde_json::json!(fuel_after.as_ref().err().map(ToString::to_string));
    retain(directory, observations);
    let packed = result
        .expect("host errors, fuel exhaustion or interrupts never count as source traps")
        .cast_unsigned();
    fuel_before.expect("original finite fuel bank");
    fuel_after.expect("unchanged finite fuel bank after observation");
    let snapshot = snapshots
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("all read-only observations must succeed");
    (packed, snapshot)
}
fn identity(code: u64) -> TrapIdentity {
    match code {
        1 => TrapIdentity::Bounds,
        2 => TrapIdentity::Allocation,
        3 => TrapIdentity::Capacity,
        4 => TrapIdentity::Refcount,
        5 => TrapIdentity::Utf8,
        _ => panic!("frozen five codes"),
    }
}
fn check(snapshot: &[u32], row: &Value, fault: bool) {
    let prefix = if fault { "fault" } else { "pristine" };
    assert_eq!(
        snapshot[3],
        u32::try_from(row[format!("{prefix}_release_count")].as_u64().expect("release count"))
            .expect("small count")
    );
    for (length, at, key) in [(4, 5, "first"), (13, 14, "last")] {
        let expected = row[format!("{prefix}_{key}")]
            .as_array()
            .expect("exact UTF8")
            .iter()
            .map(|v| u32::try_from(v.as_u64().expect("byte")).expect("u32"))
            .collect::<Vec<_>>();
        assert_eq!(usize::try_from(snapshot[length]).expect("small length"), expected.len());
        assert_eq!(&snapshot[at..at + expected.len()], expected);
        assert!(snapshot[at + expected.len()..at + 8].iter().all(|v| *v == 0));
    }
    assert_eq!(snapshot[2], u32::from(fault), "one owning-role failure only");
    if fault {
        assert!(
            snapshot[22] > 0 && snapshot[23] > 0,
            "nonempty preexisting cleanup obligations at failure"
        );
    }
    assert_eq!(
        &snapshot[24..28],
        &[0, 0, 0, 0],
        "physical L, held H, busy and recursive used all clean"
    );
}
