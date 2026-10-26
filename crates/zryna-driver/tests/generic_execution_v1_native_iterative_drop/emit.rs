//! Reuse the real audited object/runtime emitter and bind only a private local address delta.
use super::{artifact, digest};
use object::{Object, ObjectSymbol};
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn native(
    case: &str,
    provider: &str,
    program: &VerifiedExecution<'_>,
    wire: &[u8],
) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m416-native-drop-{}-{case}-{provider}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos()
    ));
    std::fs::create_dir(&directory).expect("unique actual evidence");
    std::fs::write(directory.join("program.zir"), wire).expect("real new carrier before targets");
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("genuine MIR");
    artifact::native(program, &mir, &directory).expect("same-seal audited ordinary object");
    let bytes = std::fs::read(directory.join("program.o")).expect("ordinary object");
    let file = object::File::parse(bytes.as_slice()).expect("ordinary ELF");
    let address = |name: &str| {
        file.symbols()
            .find(|s| s.name().ok() == Some(name))
            .expect("independently audited symbol")
            .address()
    };
    let delta = i128::from(address("zryna_gexecution_tls_address"))
        - i128::from(address("zryna_v1_e_entry"));
    let delta = i64::try_from(delta).expect("checked existing object-local projection");
    let mut binding = String::new();
    writeln!(binding, "#define M416_TLS_DELTA INT64_C({delta})").expect("binding");
    std::fs::write(directory.join("private_binding.h"), binding)
        .expect("no export or object patch");
    let receipt = serde_json::json!({"case":case,"provider":provider,
        "same_execution_wire_sha256":digest(wire),"ordinary_object_sha256":digest(&bytes),
        "tls_getter_delta":delta,"scope":"private test-only checked local symbol projection"});
    std::fs::write(
        directory.join("native-input-identity.json"),
        serde_json::to_vec_pretty(&receipt).expect("metadata"),
    )
    .expect("actual object identity");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_iterative_drop");
    for name in ["observer.c", "observer.h", "wrappers.c", "plain.c"] {
        std::fs::copy(root.join(name), directory.join(name)).expect("retain exact test harness");
    }
    directory
}
