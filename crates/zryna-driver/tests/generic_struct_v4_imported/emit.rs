//! All three emitters consume one identical retained opaque program.
use super::source::Evidence;
use object::{Object, ObjectSection, ObjectSymbol};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
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
        .join(format!("m416-imported-struct-v4-driver-{}-{case}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("new evidence directory");
    let js = zryna_backend_javascript::generic_owned_v2::emit(program).expect("same-seal JS");
    let wasm =
        zryna_backend_webassembly::generic_owned_v2::emit(program).expect("same-seal core Wasm");
    let mir = zryna_native_mir::generic_owned_v2::lower(program).expect("same-seal native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified native target");
    let native = zryna_backend_native::generic_owned_v2::emit_object(&mir, target)
        .expect("same-seal native object");
    zryna_backend_native::generic_owned_v2::validate_object_inventory(native.bytes(), &mir)
        .expect("independent native object inventory audit");
    let object = object::File::parse(native.bytes()).expect("audited native object inventory");
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
    let audit = serde_json::json!({
        "independent_inventory_validation_passed":true,"object_sha256":digest(native.bytes()),
        "architecture":format!("{:?}",object.architecture()),"sections":sections,"symbols":symbols,
        "sealed_mir_functions":mir.functions().iter().map(zryna_native_mir::generic_owned_v2::FunctionPlan::symbol).collect::<Vec<_>>(),
    });
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    let mut hashes = serde_json::Map::new();
    for (name, bytes) in [
        ("program.mjs", js.source.as_bytes()),
        ("program.wasm", wasm.bytes()),
        ("program.o", native.bytes()),
        ("program.zir", wire),
        ("ownership-runtime-v1.h", header.as_slice()),
        ("main.zry", evidence.source.as_slice()),
        ("provider-native.json", evidence.native.as_slice()),
        ("provider-typescript.json", evidence.typescript.as_slice()),
        ("execution-oracle.json", include_bytes!("execution-oracle.json").as_slice()),
        ("fixture-identities.json", include_bytes!("fixture-identities.json").as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("retained exact evidence bytes");
        hashes.insert(name.into(), digest(bytes).into());
    }
    for (name, bytes) in &evidence.additional {
        std::fs::write(directory.join(name), bytes).expect("retained complete imported source");
        hashes.insert((*name).into(), digest(bytes).into());
    }
    std::fs::write(
        directory.join("object-inventory-audit.json"),
        serde_json::to_vec_pretty(&audit).expect("inventory audit JSON"),
    )
    .expect("retained inventory audit");
    let receipt = serde_json::json!({
        "same_retained_seal":true,"claim_productions":1,"independent_v4_issuances":1,
        "case":case,"wire_sha256":digest(wire),"files_sha256":hashes,
        "source_rule":"exact complete main.zry/values.zry/z-leaf.zry literals including existing entry; no appended wrapper",
        "source_provider_parity":"actual native lexer/parser plus both authenticated frozen provider packets; complete files equal",
    });
    std::fs::write(
        directory.join("same-seal.json"),
        serde_json::to_vec_pretty(&receipt).expect("same-seal receipt JSON"),
    )
    .expect("retained same-seal receipt");
    Artifacts { directory }
}
