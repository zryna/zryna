//! One genuine new execution seal binds every ordinary target and its retained source evidence.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn all(
    case: &str,
    provider: &str,
    program: &VerifiedExecution<'_>,
    wire: &[u8],
) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir()
        .join(format!("m416-execution-scalar-{}-{case}-{provider}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).expect("exclusive retained evidence directory");
    std::fs::write(directory.join("program.zir"), wire)
        .expect("genuine wire before target outputs");
    let mut permissions =
        std::fs::metadata(directory.join("program.zir")).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(directory.join("program.zir"), permissions)
        .expect("freeze wire before all emissions");
    let native = super::native::write(program, &directory);
    let js = zryna_backend_javascript::generic_execution_v1::emit(program)
        .expect("same NEWseal ordinary JS");
    let wasm = zryna_backend_webassembly::generic_execution_v1::emit(program)
        .expect("same NEWseal ordinary Wasm");
    std::fs::write(directory.join("program.mjs"), js.source.as_bytes()).expect("actual JS bytes");
    std::fs::write(directory.join("program.wasm"), wasm.bytes()).expect("actual Wasm bytes");
    super::metadata::write(program, &directory);
    retain(case, &directory);
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("unchanged source wire"), wire);
    let receipt = serde_json::json!({"case":case,"provider":provider,"wire_sha256":digest(wire),"native":native,"js_sha256":digest(js.source.as_bytes()),"wasm_sha256":digest(wasm.bytes()),"expected_executed_canonical_entries":0,"note":"zero executed entries is checked by all17 observer wrappers; no no-relocation contract invented"});
    std::fs::write(
        directory.join("same-new-seal-targets.json"),
        serde_json::to_vec_pretty(&receipt).expect("receipt JSON"),
    )
    .expect("retained authority receipt");
    directory
}
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn retain(case: &str, directory: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-scalar-replay");
    for name in [
        format!("literals/{case}.zry"),
        format!("{case}-native.json"),
        format!("{case}-typescript.json"),
        "SOURCE_EXPECTED_FREEZE.json".into(),
        "packet-identities.json".into(),
        "provider-receipts.json".into(),
    ] {
        let destination = if name.starts_with("literals/") {
            directory.join("main.zry")
        } else {
            directory.join(&name)
        };
        std::fs::copy(fixture.join(&name), destination)
            .expect("unchanged source/oracle/auth retained");
    }
    assert_eq!(
        digest(
            &std::fs::read(directory.join("SOURCE_EXPECTED_FREEZE.json"))
                .expect("pre-output oracle")
        ),
        "82f4d49efba2890e1c821298060628812e146d6bd1fb01db5124e7fd5dc91452"
    );
}
