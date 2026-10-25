//! Bounded private linker/process route; all actual bytes/status are retained before checks.
use std::{path::Path, process::Command};
fn retained(directory: &Path, label: &str, command: Command, seconds: u64) -> std::process::Output {
    let output = super::process::run(command, seconds);
    std::fs::write(directory.join(format!("{label}.stdout")), &output.stdout)
        .expect("retain stdout first");
    std::fs::write(directory.join(format!("{label}.stderr")), &output.stderr)
        .expect("retain stderr first");
    let status = serde_json::json!({"success":output.status.success(),"status":output.status.to_string(),
        "code":output.status.code(),"stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len()});
    std::fs::write(
        directory.join(format!("{label}.status.json")),
        serde_json::to_vec_pretty(&status).expect("status JSON"),
    )
    .expect("status before assertions");
    output
}
pub(super) fn run(directory: &Path, row: &serde_json::Value, retry: bool) {
    let inputs = snapshot(directory);
    let mut command = Command::new("/usr/bin/gcc");
    command
        .args([
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
        .arg(directory.join("observer.c"))
        .arg(directory.join("canonical-runtime.c"))
        .arg(directory.join("program.o"));
    let symbols: Vec<String> = serde_json::from_str(include_str!(
        "../generic_execution_v1_native_iterative_drop/symbols.json"
    ))
    .expect("existing17symbol inventory");
    assert_eq!(symbols.len(), 17);
    for symbol in symbols {
        command.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
    }
    command
        .args(["-Wl,--wrap=malloc", "-Wl,--wrap=free"])
        .arg("-o")
        .arg(directory.join("invocation"));
    let compiled = retained(directory, "compile", command, 30);
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    assert!(compiled.stderr.is_empty());
    let code = if retry { 0 } else { row["canonical_status"].as_u64().unwrap_or(0) };
    let mut command = Command::new(directory.join("invocation"));
    command.arg(code.to_string()).arg(directory.join("actual.bin"));
    let actual = retained(directory, "run", command, 5);
    assert!(actual.status.success(), "{}", String::from_utf8_lossy(&actual.stderr));
    assert!(actual.stderr.is_empty());
    let size = std::fs::metadata(directory.join("actual.bin")).expect("actual trace size").len();
    assert!(size <= 65_536, "bound before allocation/read");
    let bytes = std::fs::read(directory.join("actual.bin")).expect("complete actual trace");
    assert_eq!(snapshot(directory), inputs, "compiler and child never modify issued inputs");
    super::verify::check(row, retry, &actual.stdout, &bytes);
    println!("reserved native {:?}/retry={retry}: {}", row["name"], directory.display());
}

fn snapshot(directory: &Path) -> Vec<(String, Vec<u8>)> {
    [
        "program.o",
        "canonical-runtime.c",
        "zryna_ownership_runtime_v1.h",
        "runtime_metadata.h",
        "observer.c",
        "observer.h",
        "allocation.h",
        "strings.h",
        "counts.h",
    ]
    .into_iter()
    .map(|name| {
        let path = directory.join(name);
        let limit = if name == "program.o" {
            zryna_backend_native::MAX_NATIVE_OBJECT_BYTES as u64
        } else {
            65_536
        };
        assert!(
            std::fs::metadata(&path).expect("input size").len() <= limit,
            "bound before allocation/read"
        );
        (name.into(), std::fs::read(path).expect("issued input snapshot"))
    })
    .collect()
}
