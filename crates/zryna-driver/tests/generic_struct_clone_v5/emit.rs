//! All three emitters consume one identical retained opaque program.
use super::source::Evidence;
use object::{Object, ObjectSection, ObjectSymbol};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram;

pub(super) struct Artifacts {
    pub(super) directory: PathBuf,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn all_targets(
    case: &str,
    program: &VerifiedOwnedProgram<'_>,
    wire: &[u8],
    evidence: &Evidence,
) -> Artifacts {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir()
        .join(format!("m416-struct-clone-v5-driver-{}-{case}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("new evidence directory");
    let wire_path = freeze_wire(&directory, case, wire);
    let js = zryna_backend_javascript::generic_owned_v2::emit(program).expect("same-seal-v5 JS");
    let wasm =
        zryna_backend_webassembly::generic_owned_v2::emit(program).expect("same-seal-v5 core Wasm");
    let mir = zryna_native_mir::generic_owned_v2::lower(program).expect("same-seal-v5 native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified native target");
    let native = zryna_backend_native::generic_owned_v2::emit_object(&mir, target)
        .expect("same-seal-v5 native object");
    zryna_backend_native::generic_owned_v2::validate_object_inventory(native.bytes(), &mir)
        .expect("independent native object inventory audit");
    let audit = object_audit(native.bytes(), &mir);
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    let original_expected = super::oracle::read_fixture(&format!("literals/{case}-expected.json"));
    let source_freeze = super::oracle::read_fixture("SOURCE_EXPECTED_FREEZE.json");
    let mut hashes = serde_json::Map::new();
    for (name, bytes) in [
        ("program.mjs", js.source.as_bytes()),
        ("program.wasm", wasm.bytes()),
        ("program.o", native.bytes()),
        ("ownership-runtime-v1.h", header.as_slice()),
        ("main.zry", evidence.source.as_slice()),
        ("original-expected.json", original_expected.as_slice()),
        ("SOURCE_EXPECTED_FREEZE.json", source_freeze.as_slice()),
        ("provider-native.json", evidence.native.as_slice()),
        ("provider-typescript.json", evidence.typescript.as_slice()),
        ("execution-oracle.json", include_bytes!("execution-oracle.json").as_slice()),
        ("fixture-identities.json", include_bytes!("fixture-identities.json").as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("retained exact evidence bytes");
        hashes.insert(name.into(), digest(bytes).into());
    }
    std::fs::write(
        directory.join("object-inventory-audit.json"),
        serde_json::to_vec_pretty(&audit).expect("inventory audit JSON"),
    )
    .expect("retained inventory audit");
    assert_eq!(std::fs::read(&wire_path).expect("retained wire"), wire);
    hashes.insert("program.zir".into(), digest(wire).into());
    let receipt = serde_json::json!({
        "same_retained_seal":true,"claim_productions":1,"independent_v5_issuances":1,
        "case":case,"wire_sha256":digest(wire),"files_sha256":hashes,
        "source_rule":"exact complete main.zry literal including existing entry; no appended wrapper",
        "source_provider_parity":"actual native lexer/parser plus both authenticated frozen provider packets; complete files equal",
    });
    std::fs::write(
        directory.join("same-seal.json"),
        serde_json::to_vec_pretty(&receipt).expect("same-seal-v5 receipt JSON"),
    )
    .expect("retained same-seal-v5 receipt");
    Artifacts { directory }
}

fn freeze_wire(directory: &Path, case: &str, wire: &[u8]) -> PathBuf {
    // Issuance has already succeeded. Freeze these exact genuine wire bytes BEFORE emission.
    let wire_path = directory.join("program.zir");
    std::fs::write(&wire_path, wire).expect("genuine source-issued v5 wire freeze");
    let mut permissions = std::fs::metadata(&wire_path).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&wire_path, permissions).expect("immutable pre-emission wire");
    let freeze = serde_json::json!({"case":case,"wire_sha256":digest(wire),
        "issuer":"genuine independent v5 source/type/ownership/layout/runtime issuer",
        "frozen_before_first_backend_emission":true});
    std::fs::write(
        directory.join("wire-freeze.json"),
        serde_json::to_vec_pretty(&freeze).expect("wire receipt"),
    )
    .expect("retained pre-emission wire identity");
    println!("Genuine v5 wire frozen before emission: {}", directory.display());
    wire_path
}

fn object_audit(
    bytes: &[u8],
    mir: &zryna_native_mir::generic_owned_v2::VerifiedProgram<'_, '_>,
) -> serde_json::Value {
    let object = object::File::parse(bytes).expect("audited native object inventory");
    let sections = object
        .sections()
        .map(|section| {
            serde_json::json!({
                "name":section.name().expect("ELF section name"), "size":section.size(),
                "kind":format!("{:?}", section.kind()),
                "relocations":section.relocations().map(|(offset, relocation)| serde_json::json!({
                    "offset":offset,"kind":format!("{:?}",relocation.kind()),
                    "target":format!("{:?}",relocation.target()),"addend":relocation.addend(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let symbols = object
        .symbols()
        .map(|symbol| {
            serde_json::json!({
                "name":symbol.name().expect("ELF symbol name"), "size":symbol.size(),
                "undefined":symbol.is_undefined(),"kind":format!("{:?}",symbol.kind()),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "independent_inventory_validation_passed":true,"object_sha256":digest(bytes),
        "architecture":format!("{:?}",object.architecture()),"sections":sections,"symbols":symbols,
        "sealed_mir_functions":mir.functions().iter().map(zryna_native_mir::generic_owned_v2::FunctionPlan::symbol).collect::<Vec<_>>(),
    })
}
