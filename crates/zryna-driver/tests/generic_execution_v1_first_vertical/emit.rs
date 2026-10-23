//! Freeze genuine new wire before feeding one retained new seal to all target owners.
use super::{authority, source::Evidence};
use object::{Object, ObjectSection, ObjectSymbol};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) struct Artifacts {
    pub(super) directory: PathBuf,
}
pub(super) fn all(
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
        .join(format!("m416-new-execution-driver-{}-{case}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("new exact evidence directory");
    freeze_wire(&directory, case, wire);
    let js_ordinary = zryna_backend_javascript::generic_execution_v1::emit(program)
        .expect("same NEW seal ordinary JavaScript issuer");
    let js = zryna_backend_javascript::generic_execution_v1::emit_observed(program)
        .expect("distinct sameprogram private-observed JavaScript artifact");
    let wasm = zryna_backend_webassembly::generic_execution_v1::emit(program)
        .expect("same NEW seal ordinary Wasm issuer");
    let observed = zryna_backend_webassembly::generic_execution_v1::emit_observed(program)
        .expect("distinct sameprogram audited test-observed Wasm artifact");
    let mir = zryna_native_mir::generic_execution_v1::lower(program)
        .expect("same NEW seal independent native MIR issuer");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified native target");
    let native = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("source-bound NEW native object issuer");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(native.bytes(), &mir)
        .expect("independent new native object inventory");
    let symbols = mir
        .functions()
        .iter()
        .map(zryna_native_mir::generic_execution_v1::FunctionPlan::symbol)
        .collect::<Vec<_>>();
    let audit = object_audit(native.bytes(), &symbols);
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    let expected = authority::read("SOURCE_EXPECTED.json");
    let source_freeze = authority::read("SOURCE_FREEZE.json");
    let mut files = serde_json::Map::new();
    for (name, bytes) in [
        ("program.mjs", js.as_source().as_bytes()),
        ("program-ordinary.mjs", js_ordinary.source.as_bytes()),
        ("program-ordinary.wasm", wasm.bytes()),
        ("program-observed.wasm", observed.as_bytes()),
        ("program.o", native.bytes()),
        ("ownership-runtime-v1.h", header.as_slice()),
        ("main.zry", evidence.source.as_slice()),
        ("provider-native.json", evidence.native.as_slice()),
        ("provider-typescript.json", evidence.typescript.as_slice()),
        ("SOURCE_EXPECTED.json", expected.as_slice()),
        ("SOURCE_FREEZE.json", source_freeze.as_slice()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("retained exact evidence");
        files.insert(name.into(), authority::digest(bytes).into());
    }
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("frozen genuine wire"), wire);
    files.insert("program.zir".into(), authority::digest(wire).into());
    let receipt = serde_json::json!({"case":case,"domain":"generic.execution.v1","same_retained_new_seal":true,"claim_productions":1,"independent_new_issuances":1,"files_sha256":files,"source_rule":"exact complete main.zry literal, existing entry preserved; no appended wrapper","javascript_observation":"explicit opaque private-observed artifact; module-private ordinary runtime artifact retained separately","wasm_observation":"explicit distinct audited test artifact, sameprogram and checked memory plan; ordinary artifact retained separately","object_audit":audit});
    std::fs::write(
        directory.join("same-new-seal.json"),
        serde_json::to_vec_pretty(&receipt).expect("receipt JSON"),
    )
    .expect("exact identity receipt");
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
