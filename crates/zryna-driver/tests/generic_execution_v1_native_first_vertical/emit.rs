//! Genuine new source seal feeds only the new native MIR and object owners.
use super::{authority, source::Evidence};
use object::{Object, ObjectSection, ObjectSymbol};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) struct Artifacts {
    pub(super) directory: PathBuf,
}
pub(super) fn native(
    case: &str,
    program: &VerifiedExecution<'_>,
    wire: &[u8],
    evidence: &Evidence,
) -> Artifacts {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir()
        .join(format!("m416-new-execution-native-driver-{}-{case}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("new exact native evidence directory");
    freeze_wire(&directory, case, wire);
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("NEW native MIR owner");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified native target");
    let artifact = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("NEW source-bound native object owner");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(artifact.bytes(), &mir)
        .expect("independent new native object inventory");
    let symbols = mir
        .functions()
        .iter()
        .map(zryna_native_mir::generic_execution_v1::FunctionPlan::symbol)
        .collect::<Vec<_>>();
    let audit = object_audit(artifact.bytes(), &symbols);
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    let expected = authority::read("SOURCE_EXPECTED.json");
    let source_freeze = authority::read("SOURCE_FREEZE.json");
    let mut files = serde_json::Map::new();
    for (name, bytes) in [
        ("program.o", artifact.bytes()),
        ("ownership-runtime-v1.h", header.as_slice()),
        ("main.zry", evidence.source.as_slice()),
        ("provider-native.json", evidence.native.as_slice()),
        ("provider-typescript.json", evidence.typescript.as_slice()),
        ("SOURCE_EXPECTED.json", expected.as_slice()),
        ("SOURCE_FREEZE.json", source_freeze.as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("retained exact native evidence");
        files.insert(name.into(), authority::digest(bytes).into());
    }
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("frozen genuine wire"), wire);
    files.insert("program.zir".into(), authority::digest(wire).into());
    let receipt = serde_json::json!({"case":case,"domain":"generic.execution.v1","target":"native-only","same_retained_new_seal":true,"claim_productions":1,"independent_new_issuances":1,"files_sha256":files,"source_rule":"exact complete main.zry literal, existing entry preserved; no appended wrapper","object_audit":audit});
    std::fs::write(
        directory.join("same-new-seal-native-only.json"),
        serde_json::to_vec_pretty(&receipt).expect("receipt JSON"),
    )
    .expect("exact native identity receipt");
    Artifacts { directory }
}
fn freeze_wire(directory: &Path, case: &str, wire: &[u8]) {
    let path = directory.join("program.zir");
    std::fs::write(&path, wire).expect("genuine source-issued new wire BEFORE emission");
    let mut permissions = std::fs::metadata(&path).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).expect("immutable pre-emission wire");
    let receipt = serde_json::json!({"case":case,"wire_sha256":authority::digest(wire),"domain":"generic.execution.v1","frozen_before_first_backend_emission":true});
    std::fs::write(
        directory.join("wire-freeze.json"),
        serde_json::to_vec_pretty(&receipt).expect("wire receipt"),
    )
    .expect("retained wire freeze");
}
fn object_audit(bytes: &[u8], functions: &[&str]) -> serde_json::Value {
    let file = object::File::parse(bytes).expect("independently inventory-validated native object");
    let sections = file.sections().map(|section| serde_json::json!({"name":section.name().expect("ELF name"),"size":section.size(),"kind":format!("{:?}",section.kind()),"relocations":section.relocations().map(|(offset,relocation)|serde_json::json!({"offset":offset,"target":format!("{:?}",relocation.target()),"kind":format!("{:?}",relocation.kind())})).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let symbols = file.symbols().map(|symbol|serde_json::json!({"name":symbol.name().expect("ELF symbol"),"undefined":symbol.is_undefined(),"size":symbol.size()})).collect::<Vec<_>>();
    serde_json::json!({"independent_inventory_validation_passed":true,"object_sha256":authority::digest(bytes),"sections":sections,"symbols":symbols,"new_mir_functions":functions})
}
