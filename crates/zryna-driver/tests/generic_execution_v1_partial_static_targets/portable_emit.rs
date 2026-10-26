//! Distinct ordinary/observed artifacts from one retained genuine NEW execution seal.
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn write(program: &VerifiedExecution<'_>, directory: &Path) {
    let ordinary = zryna_backend_javascript::generic_execution_v1::emit(program)
        .expect("same NEWseal ordinary JavaScript partial-place consumer");
    let observed = zryna_backend_javascript::generic_execution_v1::emit_observed(program)
        .expect("same NEWseal private-observed JavaScript partial-place consumer");
    let wasm = zryna_backend_webassembly::generic_execution_v1::emit(program)
        .expect("same NEWseal core Wasm partial-place consumer");
    let wasm_observed = zryna_backend_webassembly::generic_execution_v1::emit_observed(program)
        .expect("same NEWseal private-observed core Wasm partial-place consumer");
    for (name, bytes) in [
        ("program-ordinary.mjs", ordinary.source.as_bytes()),
        ("program-observed.mjs", observed.as_source().as_bytes()),
        ("program-ordinary.wasm", wasm.bytes()),
        ("program-observed.wasm", wasm_observed.as_bytes()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("retained exact target artifact");
    }
}
