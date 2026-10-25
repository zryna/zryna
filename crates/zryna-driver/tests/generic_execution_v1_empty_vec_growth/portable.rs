//! Finite ordinary JavaScript and ordinary/observed Wasm parity from the same genuine seal.
use std::{path::Path, process::Command};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn run(program: &VerifiedExecution<'_>, directory: &Path) {
    let js = zryna_backend_javascript::generic_execution_v1::emit(program)
        .expect("ordinary same-seal JS");
    std::fs::write(directory.join("ordinary.mjs"), &js.source).expect("actual JS bytes");
    std::fs::write(directory.join("javascript.mjs"), include_bytes!("javascript.mjs"))
        .expect("frozen JS runner");
    let mut node = Command::new("node");
    node.arg(directory.join("javascript.mjs")).arg(directory.join("ordinary.mjs"));
    let output = super::process::run(node, 30);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"result:18\n");
    std::fs::write(directory.join("javascript.stdout"), output.stdout)
        .expect("actual parity receipt");
    let wasm = zryna_backend_webassembly::generic_execution_v1::emit(program)
        .expect("ordinary same-seal Wasm");
    let observed = zryna_backend_webassembly::generic_execution_v1::emit_observed(program)
        .expect("separate observed same-seal Wasm");
    std::fs::write(directory.join("ordinary.wasm"), wasm.bytes()).expect("actual ordinary Wasm");
    std::fs::write(directory.join("observed.wasm"), observed.as_bytes())
        .expect("actual observed Wasm");
    wasm_check(wasm.bytes(), false);
    wasm_check(observed.as_bytes(), true);
}
fn wasm_check(bytes: &[u8], observed: bool) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, bytes).expect("actual audited Wasm bytes");
    assert_eq!(module.imports().len(), 0);
    assert_eq!(
        module.exports().map(|e| e.name()).collect::<Vec<_>>(),
        if observed { vec!["entry", "$zryna$observation"] } else { vec!["entry"] }
    );
    let mut store = wasmtime::Store::new(&engine, ());
    let instance =
        wasmtime::Instance::new(&mut store, &module, &[]).expect("private bounded memory");
    let entry =
        instance.get_typed_func::<(), i32>(&mut store, "entry").expect("original scalar ABI");
    if observed {
        let getter = instance
            .get_typed_func::<i32, i32>(&mut store, "$zryna$observation")
            .expect("existing explicit test protocol");
        for ordinal in [0, 1, 0, 2, 0] {
            let selector = if ordinal == 0 { 0x2000_0001 } else { 0x2200_0000 | ordinal };
            assert_eq!(getter.call(&mut store, selector).expect("reset after exact cleanup"), 0);
            if ordinal == 1 {
                let error =
                    entry.call(&mut store, ()).expect_err("first actual positive allocation fails");
                assert_eq!(
                    error.downcast_ref::<wasmtime::Trap>(),
                    Some(&wasmtime::Trap::UnreachableCodeReached)
                );
                assert_eq!(getter.call(&mut store, 0).expect("allocation source trap identity"), 2);
            } else {
                assert_eq!(
                    entry.call(&mut store, ()).expect("fault2 unreachable or pristine retry"),
                    18
                );
                assert_eq!(getter.call(&mut store, 0).expect("no source trap"), 0);
            }
        }
    } else {
        for _ in 0..3 {
            assert_eq!(entry.call(&mut store, ()).expect("ordinary result and retries"), 18);
        }
    }
}
