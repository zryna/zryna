//! One genuine new wire/seal reaches only checked new native owners and canonical runtime metadata.
use object::{Object, ObjectSection, ObjectSymbol};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) struct Artifacts {
    pub(super) directory: PathBuf,
}
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn native(case: &str, program: &VerifiedExecution<'_>, wire: &[u8]) -> Artifacts {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir()
        .join(format!("m416-execution-native-push-upgrade-{}-{case}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("exclusive native evidence directory");
    freeze_wire(&directory, wire);
    let mir = zryna_native_mir::generic_execution_v1::lower(program)
        .expect("genuine whole-seal native MIR owner");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified native target");
    let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("new container native object owner");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), &mir)
        .expect("independent native object inventory");
    let mut functions = mir
        .functions()
        .iter()
        .map(zryna_native_mir::generic_execution_v1::FunctionPlan::symbol)
        .collect::<Vec<_>>();
    if let Some(containers) = mir.containers() {
        functions.extend(containers.clone_symbols().flatten());
        functions.extend(containers.drop_symbols().flatten());
    }
    let audit = object_audit(object.bytes(), &functions);
    std::fs::write(directory.join("program.o"), object.bytes()).expect("retained actual object");
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    std::fs::write(directory.join("zryna_ownership_runtime_v1.h"), header)
        .expect("same checked ABI header");
    super::metadata::write(program, &directory);
    retain_inputs(case, &directory);
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("frozen wire"), wire);
    let files = std::fs::read_dir(&directory)
        .expect("retained inputs")
        .map(|entry| {
            let entry = entry.expect("artifact entry");
            let bytes = std::fs::read(entry.path()).expect("actual retained artifact");
            (entry.file_name().into_string().expect("artifact UTF8 name"), digest(&bytes))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let receipt = serde_json::json!({"case":case,"domain":"generic.execution.v1","target":"native","claim_productions":1,"independent_new_issuances":1,"source_identity":format!("{:?}",program.layouts(zryna_layout::StorageTarget::LinuxX8664V1).source_map_identity()),"files_sha256":files,"wire_sha256":digest(wire),"object_sha256":digest(object.bytes()),"object_audit":audit,"runtime":"canonical inherited source; element marker only from same NEWseal metadata; external --wrap observation/fault seam"});
    std::fs::write(
        directory.join("same-new-seal-native.json"),
        serde_json::to_vec_pretty(&receipt).expect("receipt JSON"),
    )
    .expect("retained same-seal evidence");
    Artifacts { directory }
}
fn freeze_wire(directory: &Path, wire: &[u8]) {
    let path = directory.join("program.zir");
    std::fs::write(&path, wire).expect("freeze actual new-domain wire BEFORE lowering/emission");
    let mut permissions = std::fs::metadata(&path).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(path, permissions).expect("read-only genuine wire");
    std::fs::write(directory.join("wire-freeze.json"), serde_json::to_vec_pretty(&serde_json::json!({"wire_sha256":digest(wire),"before_first_native_lower_or_emission":true})).expect("wire freeze JSON")).expect("freeze receipt");
}
fn retain_inputs(case: &str, directory: &Path) {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-execution-containers");
    let expected = super::source::row(case);
    let source =
        std::fs::read(root.join("literals").join(format!("{case}.zry"))).expect("original literal");
    assert_eq!(digest(&source), expected["sha256"]);
    std::fs::write(directory.join("main.zry"), source)
        .expect("complete literal retained unchanged");
    for name in [
        format!("{case}-native.json"),
        format!("{case}-typescript.json"),
        "packet-identities.json".into(),
        "SOURCE_EXPECTED_FREEZE.json".into(),
    ] {
        std::fs::copy(root.join(&name), directory.join(&name))
            .expect("original named input retained");
    }
    std::fs::write(
        directory.join("RUNTIME_EXPECTED.json"),
        include_bytes!("RUNTIME_EXPECTED.json"),
    )
    .expect("unchanged independent runtime oracle");
}
fn object_audit(bytes: &[u8], functions: &[&str]) -> serde_json::Value {
    let file = object::File::parse(bytes).expect("independent ELF inventory");
    let actual = file
        .symbols()
        .filter(|symbol| symbol.is_definition() && symbol.kind() == object::SymbolKind::Text)
        .map(|symbol| symbol.name().expect("ELF symbol"))
        .collect::<std::collections::BTreeSet<_>>();
    let mut expected = functions.iter().copied().collect::<std::collections::BTreeSet<_>>();
    expected.insert("zryna_v1_e_entry");
    assert_eq!(
        actual, expected,
        "source functions + opaque retained Clone/drop helpers + scalar entry only"
    );
    let sections = file.sections().map(|section|serde_json::json!({"name":section.name().expect("section"),"size":section.size(),"relocations":section.relocations().count()})).collect::<Vec<_>>();
    serde_json::json!({"defined_text_symbols":actual,"expected_retained_symbols":expected,"sections":sections,"validated_object_inventory":true})
}
