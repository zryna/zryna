//! Independent actual backend calls receive the identical retained genuine v5 program.
use super::source::Evidence;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn freeze(
    case: &str,
    wire: &[u8],
    expected: &serde_json::Value,
    evidence: &Evidence,
) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir()
        .join(format!("m416-clone-compact-dag-{}-{case}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("new resource evidence directory");
    let mut hashes = serde_json::Map::new();
    for (name, bytes) in [
        ("program.zir", wire),
        ("main.zry", evidence.source.as_slice()),
        ("provider-native.json", evidence.native.as_slice()),
        ("provider-typescript.json", evidence.typescript.as_slice()),
        ("runtime-expected.json", evidence.runtime_expected.as_slice()),
        ("resource-expected.json", evidence.resource_expected.as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("exact genuine evidence");
        hashes.insert(name.into(), hash(bytes).into());
    }
    let wire_path = directory.join("program.zir");
    let mut permissions = std::fs::metadata(&wire_path).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&wire_path, permissions)
        .expect("freeze genuine wire BEFORE all backends");
    let receipt = serde_json::json!({"case":case,"same_retained_v5_seal":true,
        "source_productions":1,"independent_v5_issuances":1,"wire_frozen_before_all_backends":true,
        "files_sha256":hashes,"independent_possible_string_occurrences":expected["possibleStringOccurrences"]});
    std::fs::write(
        directory.join("same-seal.json"),
        serde_json::to_vec_pretty(&receipt).expect("receipt JSON"),
    )
    .expect("retained genuine phase receipt");
    println!(
        "Genuine compact source/IR seal frozen BEFORE backend qualification: {}",
        directory.display()
    );
    directory
}
pub(super) fn observe(
    program: &VerifiedOwnedProgram<'_>,
    expected: &serde_json::Value,
    directory: &Path,
) -> serde_json::Value {
    // Record all three independent observations before asserting any expected outcome.
    let native = native(program, expected, directory);
    let javascript = match zryna_backend_javascript::generic_owned_v2::emit(program) {
        Ok(artifact) => {
            std::fs::write(directory.join("program.mjs"), artifact.source.as_bytes())
                .expect("actual JS");
            serde_json::json!({"outcome":"accept","bytes":artifact.source.len(),"sha256":hash(artifact.source.as_bytes())})
        }
        Err(error) => {
            serde_json::json!({"outcome":if error.code() == "ZRYNA-J2003" { "budget" } else { "error" },"phase":"javascript-emission","diagnostic":error.code(),"message":error.message()})
        }
    };
    let webassembly = match zryna_backend_webassembly::generic_owned_v2::emit(program) {
        Ok(artifact) => {
            std::fs::write(directory.join("program.wasm"), artifact.bytes()).expect("actual Wasm");
            serde_json::json!({"outcome":"accept","bytes":artifact.bytes().len(),"sha256":hash(artifact.bytes())})
        }
        Err(error) => {
            serde_json::json!({"outcome":if error.code() == "ZRYNA-W4001" { "budget" } else { "error" },"phase":"webassembly-emission","diagnostic":error.code(),"message":error.message()})
        }
    };
    serde_json::json!({"native":native,"javascript":javascript,"webassembly":webassembly})
}
fn native(
    program: &VerifiedOwnedProgram<'_>,
    expected: &serde_json::Value,
    directory: &Path,
) -> serde_json::Value {
    let mir = match zryna_native_mir::generic_owned_v2::lower(program) {
        Ok(mir) => mir,
        Err(error) => {
            return serde_json::json!({"outcome":if error.code() == "ZRYNA-N7001" { "budget" } else { "error" },"phase":"native-mir","diagnostic":error.code(),"message":error.message()});
        }
    };
    if expected["native"]["outcome"] == "budget" {
        return serde_json::json!({"outcome":"accept","phase":"native-mir","object_emission_skipped_after_unexpected_acceptance":true});
    }
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified ELF target");
    match zryna_backend_native::generic_owned_v2::emit_object(&mir, target) {
        Ok(artifact) => {
            zryna_backend_native::generic_owned_v2::validate_object_inventory(
                artifact.bytes(),
                &mir,
            )
            .expect("actual native object inventory audit");
            std::fs::write(directory.join("program.o"), artifact.bytes())
                .expect("actual audited ELF object");
            serde_json::json!({"outcome":"accept","phase":"native-object","object_inventory_validated":true,"bytes":artifact.bytes().len(),"sha256":hash(artifact.bytes())})
        }
        Err(error) => {
            serde_json::json!({"outcome":"error","phase":"native-object","diagnostic":error.code(),"message":error.message()})
        }
    }
}
