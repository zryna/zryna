//! Root-run actual scalar processes and Wasmtime instances, preserving the source-frozen rows.
use super::process;
use serde_json::Value;
use std::{path::Path, process::Command};
const SYMBOLS: [&str; 17] = [
    "allocate",
    "grow",
    "release",
    "string_from_utf8_copy",
    "string_clone",
    "string_concat",
    "string_release",
    "vec_allocate",
    "vec_reserve",
    "vec_release_storage",
    "strong_clone",
    "weak_downgrade",
    "weak_clone",
    "weak_upgrade",
    "strong_release_begin",
    "strong_release_finish",
    "weak_release",
];
pub(super) fn all(case: &str, directory: &Path) -> usize {
    compile(directory);
    let mut node = Command::new("node");
    node.arg(directory.join("javascript.mjs")).arg(directory.join("program.mjs")).arg(case);
    let js = success(node);
    std::fs::write(directory.join("javascript-observations.json"), js.stdout)
        .expect("actual JS observations");
    let row = super::source::row(case);
    let rows = row["runs"].as_array().expect("frozen scalar inputs/results");
    let mut native = Vec::new();
    for run in rows {
        let mut command = Command::new(directory.join("program-native"));
        for value in run["arguments"].as_array().expect("source scalar arguments") {
            command.arg(value.as_bool().map_or_else(
                || value.as_i64().expect("exact i32").to_string(),
                |v| i32::from(v).to_string(),
            ));
        }
        let output = success(command);
        let expected = run["i32"].as_i64().expect("pre-output source scalar result");
        assert_eq!(output.stdout, format!("result:{expected}\n").repeat(3).as_bytes());
        native.push(serde_json::json!({"arguments":run["arguments"],"i32":expected,"same_process_retries":3,"actual_stdout":String::from_utf8(output.stdout).expect("scalar stdout"),"executed_canonical_entries":0}));
    }
    std::fs::write(
        directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&native).expect("native receipt JSON"),
    )
    .expect("actual native receipts");
    wasm(directory, rows);
    rows.len()
}
fn success(command: Command) -> std::process::Output {
    let output = process::run(command, 30);
    assert!(
        output.status.success(),
        "{:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}
fn compile(directory: &Path) {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generic_execution_v1_scalar_targets");
    for name in ["scalar.c", "zero_events.c", "javascript.mjs"] {
        std::fs::copy(root.join(name), directory.join(name)).expect("exact frozen harness sources");
    }
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.args([
        "-std=c11",
        "-O0",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-no-pie",
        "-Wl,--build-id=none",
        "-I",
    ])
    .arg(directory)
    .arg(directory.join("scalar.c"))
    .arg(directory.join("zero_events.c"))
    .arg(directory.join("canonical-runtime.c"))
    .arg(directory.join("program.o"));
    for name in SYMBOLS {
        gcc.arg(format!("-Wl,--wrap=zryna_rt_o1_{name}"));
    }
    gcc.arg("-o").arg(directory.join("program-native"));
    success(gcc);
}
fn wasm(directory: &Path, rows: &[Value]) {
    let bytes = std::fs::read(directory.join("program.wasm")).expect("actual audited Wasm bytes");
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &bytes).expect("actual Wasm module");
    assert_eq!(module.imports().len(), 0);
    assert_eq!(module.exports().map(|e| e.name()).collect::<Vec<_>>(), ["entry"]);
    let mut store = wasmtime::Store::new(&engine, ());
    let instance =
        wasmtime::Instance::new(&mut store, &module, &[]).expect("ordinary same-seal module");
    let entry = instance.get_func(&mut store, "entry").expect("exact scalar export");
    let mut observations = Vec::new();
    for run in rows {
        let arguments = run["arguments"]
            .as_array()
            .expect("frozen arguments")
            .iter()
            .map(|v| {
                wasmtime::Val::I32(v.as_bool().map_or_else(
                    || i32::try_from(v.as_i64().expect("source i32")).expect("i32 range"),
                    i32::from,
                ))
            })
            .collect::<Vec<_>>();
        let expected =
            i32::try_from(run["i32"].as_i64().expect("source result")).expect("i32 range");
        for _ in 0..3 {
            let mut result = [wasmtime::Val::I32(0)];
            entry.call(&mut store, &arguments, &mut result).expect("pure source does not trap");
            assert_eq!(result[0].i32(), Some(expected));
        }
        observations.push(serde_json::json!({"arguments":run["arguments"],"i32":expected,"same_instance_retries":3}));
    }
    std::fs::write(
        directory.join("wasm-observations.json"),
        serde_json::to_vec_pretty(&observations).expect("Wasm receipt JSON"),
    )
    .expect("actual Wasm observations");
}
