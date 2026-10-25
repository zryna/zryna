//! Retain all owning artifacts and actual phase status before asserting target acceptance.
use super::{
    execute,
    source::{digest, input},
};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn retain(case: &str, provider: &str, program: &VerifiedExecution<'_>, wire: &[u8]) {
    let directory = std::env::temp_dir()
        .join(format!("m416-recursive-portable-windows-{}-{case}-{provider}", std::process::id()));
    std::fs::create_dir(&directory).expect("new retained observation directory");
    std::fs::write(directory.join("program.zir"), wire).expect("wire before target emission");
    let mut mode =
        std::fs::metadata(directory.join("program.zir")).expect("wire state").permissions();
    mode.set_readonly(true);
    std::fs::set_permissions(directory.join("program.zir"), mode).expect("read-only original wire");
    let js = zryna_backend_javascript::generic_execution_v1::emit(program);
    let wasm = zryna_backend_webassembly::generic_execution_v1::emit(program);
    let observed_js = zryna_backend_javascript::generic_execution_v1::emit_observed(program);
    let observed_wasm = zryna_backend_webassembly::generic_execution_v1::emit_observed(program);
    if let Ok(artifact) = &js {
        std::fs::write(directory.join("program.mjs"), &artifact.source).expect("ordinary JS");
    }
    if let Ok(artifact) = &wasm {
        std::fs::write(directory.join("program.wasm"), artifact.bytes()).expect("ordinary Wasm");
    }
    if let Ok(artifact) = &observed_js {
        std::fs::write(directory.join("program.observed.mjs"), artifact.as_source())
            .expect("observed JS");
    }
    if let Ok(artifact) = &observed_wasm {
        std::fs::write(directory.join("program.observed.wasm"), artifact.as_bytes())
            .expect("observed Wasm");
    }
    std::fs::write(
        directory.join("SOURCE_EXPECTED_FREEZE.json"),
        input("SOURCE_EXPECTED_FREEZE.json"),
    )
    .expect("unchanged source oracle");
    let phases = serde_json::json!({"case":case,"provider":provider,"wire_sha256":digest(wire),
        "javascript":js.as_ref().err().map(|error|format!("{error:?}")),"webassembly":wasm.as_ref().err().map(|error|format!("{error:?}")),
        "observed_javascript":observed_js.as_ref().err().map(|error|format!("{error:?}")),"observed_webassembly":observed_wasm.as_ref().err().map(|error|format!("{error:?}")),
        "scope":"Windows deep JS full allocation/release ledger; core-Wasm exact String projection/controlled failure/retry only; Native not attempted.",
        "depths":[0,1,8192,65536],"node_execution_seconds":20,"stream_bytes":65536,"existing_windows_cleanup_seconds":5});
    std::fs::write(
        directory.join("phases.json"),
        serde_json::to_vec_pretty(&phases).expect("phase JSON"),
    )
    .expect("retain phases before assertions");
    println!("portable recursive source {case}/{provider}: {}", directory.display());
    assert!(js.is_ok() && wasm.is_ok() && observed_js.is_ok() && observed_wasm.is_ok(), "{phases}");
    memory(wasm.as_ref().expect("retained ordinary Wasm").bytes());
    memory(observed_wasm.as_ref().expect("retained private Wasm").as_bytes());
    execute::javascript(case, provider, &directory);
    execute::webassembly(case, provider, &directory);
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("retained wire"), wire);
}
fn memory(bytes: &[u8]) {
    let mut count = 0;
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        if let wasmparser::Payload::MemorySection(reader) = payload.expect("owned actual Wasm") {
            for memory in reader {
                let memory = memory.expect("owned defined memory");
                count += 1;
                assert_eq!(memory.initial, 256);
                assert_eq!(memory.maximum, Some(256));
                assert!(!memory.memory64 && !memory.shared);
            }
        }
    }
    assert_eq!(count, 1, "existing fixed256page memory, never memory export authority");
}
