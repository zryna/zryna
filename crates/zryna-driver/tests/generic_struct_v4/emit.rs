//! Every target consumes the identical retained opaque program, without re-production.
use super::source::Case;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram;

pub(super) struct Artifacts {
    pub(super) directory: PathBuf,
}

pub(super) fn all_targets(
    case: Case,
    program: &VerifiedOwnedProgram<'_>,
    wire: &[u8],
) -> Artifacts {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "m416-struct-v4-driver-{}-{}-{unique}",
        std::process::id(),
        case.name()
    ));
    std::fs::create_dir(&directory).expect("new evidence directory");
    let js = zryna_backend_javascript::generic_owned_v2::emit(program).expect("same-seal JS");
    let wasm =
        zryna_backend_webassembly::generic_owned_v2::emit(program).expect("same-seal core Wasm");
    let mir = zryna_native_mir::generic_owned_v2::lower(program).expect("bounded native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified target");
    let native = zryna_backend_native::generic_owned_v2::emit_object(&mir, target)
        .expect("same-seal native object");
    zryna_backend_native::generic_owned_v2::validate_object_inventory(native.bytes(), &mir)
        .expect("independent native object audit");
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    for (name, bytes) in [
        ("program.mjs", js.source.as_bytes()),
        ("program.wasm", wasm.bytes()),
        ("program.o", native.bytes()),
        ("program.zir", wire),
        ("ownership-runtime-v1.h", header.as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("actual emitted evidence");
    }
    let receipt = serde_json::json!({"same_retained_seal":true,"case":case.name(),"wire_sha256":format!("{:x}",Sha256::digest(wire)),"source_provider_parity":"actual native lexer/parser and authenticated TypeScript packets"});
    std::fs::write(
        directory.join("same-seal.json"),
        serde_json::to_vec_pretty(&receipt).expect("receipt JSON"),
    )
    .expect("same-seal receipt");
    Artifacts { directory }
}
