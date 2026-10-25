//! Same-owner standalone carrier and exact runtime template; ordinary object is never linked.
use sha2::{Digest as _, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use zryna_abi::{Invocation, ScalarValue};
use zryna_backend_native::{
    ValidatedNativeObjectArtifact, generic_execution_v1::ValidatedInvocationArtifact,
};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn exercise(
    program: &VerifiedExecution<'_>,
    wire: &[u8],
    row: &serde_json::Value,
    provider: &str,
    retry: bool,
) {
    let directory = directory(row, provider, retry);
    std::fs::write(directory.join("program.zir"), wire).expect("wire before target emission");
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("same NEW MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("fixed target");
    let ordinary = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("audited ordinary baseline");
    let key = if retry { "retry_arguments" } else { "arguments" };
    let args = row[key]
        .as_array()
        .expect("baked arguments")
        .iter()
        .map(|n| {
            ScalarValue::I32(
                i32::try_from(n.as_i64().expect("i32 argument")).expect("bounded argument"),
            )
        })
        .collect();
    let carrier = zryna_backend_native::generic_execution_v1::emit_invocation(
        &mir,
        Invocation::new("entry".into(), args),
        target,
    )
    .expect("typed source-owned reserved carrier");
    assert!(std::ptr::eq(carrier.source(), program));
    assert!(std::ptr::eq(carrier.mir(), &raw const mir));
    assert_eq!(carrier.entry_symbol(), "zryna_gexecution_invoke_v1");
    assert_eq!(carrier.invocation().export().result(), zryna_abi::ScalarType::I32);
    carrier.audit_bytes(carrier.bytes()).expect("selected-entry independent audit");
    write_material(program, wire, &directory, &ordinary, &carrier);
    super::execute::run(&directory, row, retry);
    carrier
        .audit_bytes(
            &std::fs::read(directory.join("program.o")).expect("retained carrier after execution"),
        )
        .expect("same opaque carrier after process");
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("retained wire"), wire);
    assert_eq!(
        ordinary,
        zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
            .expect("ordinary byte replay")
    );
    assert_eq!(
        std::fs::read(directory.join("program.o")).expect("retained carrier"),
        carrier.bytes()
    );
}
fn directory(row: &serde_json::Value, provider: &str, retry: bool) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m416-native-invoker-{}-{}-{}-{}",
        std::process::id(),
        row["name"].as_str().expect("name"),
        provider,
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).expect("create-only retained test material");
    let receipt = serde_json::json!({"provider":provider,"case":row,"retry":retry,
        "request_changes_for_bounds_retry_only":row["arguments"]!=row["retry_arguments"],
        "scope":"private declared-status interposer; no production fault flag or RuntimeSource override"});
    std::fs::write(directory.join("case.json"), serde_json::to_vec_pretty(&receipt).expect("JSON"))
        .expect("case before output");
    directory
}
fn write_material(
    program: &VerifiedExecution<'_>,
    wire: &[u8],
    directory: &Path,
    ordinary: &ValidatedNativeObjectArtifact,
    carrier: &ValidatedInvocationArtifact<'_, '_, '_>,
) {
    std::fs::write(directory.join("ordinary.o"), ordinary.bytes()).expect("ordinary retained only");
    std::fs::write(directory.join("program.o"), carrier.bytes()).expect("carrier linked alone");
    std::fs::write(
        directory.join("zryna_ownership_runtime_v1.h"),
        &program.runtime().declarations().native_header,
    )
    .expect("same17ABI");
    super::metadata::write(program, directory);
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_invocation_traps");
    for name in ["observer.c", "observer.h", "allocation.h", "strings.h", "counts.h"] {
        std::fs::copy(source.join(name), directory.join(name))
            .expect("exact private test observer");
    }
    let receipt = serde_json::json!({"wire_sha256":digest(wire),"ordinary_sha256":digest(ordinary.bytes()),
        "carrier_sha256":digest(carrier.bytes()),"entry":carrier.entry_symbol(),
        "baked_arguments":carrier.invocation().arguments(),"product_runtime_modified":false});
    std::fs::write(
        directory.join("same-owner.json"),
        serde_json::to_vec_pretty(&receipt).expect("JSON"),
    )
    .expect("actual binding before execution");
}
