//! Actual returned private status, unchanged object and fixed small seeded observations.
use super::{emit::Artifacts, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
fn success(command: Command, seconds: u64) -> std::process::Output {
    let output = process::run(command, seconds);
    assert!(
        output.status.success(),
        "{:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}
pub(super) fn all(case: &str, artifacts: &Artifacts) -> usize {
    let directory = &artifacts.directory;
    let bytes = include_bytes!("PRIVATE_EXPECTED_FREEZE.json");
    std::fs::write(directory.join("PRIVATE_EXPECTED_FREEZE.json"), bytes)
        .expect("retain pre-output private oracle");
    compile(directory);
    assert_eq!(
        super::emit::digest(bytes),
        "4627532e0c92679e016a065040ac173c382118e1bceaff3fd0168f0b9fd8f9df"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(bytes).expect("pre-output seeded private oracle");
    let mut observations = Vec::new();
    for row in oracle["cases"].as_array().expect("eight frozen private probes") {
        if row["case"] == case {
            observations.push(one(row, directory));
        }
    }
    let count = observations.len();
    std::fs::write(directory.join("actual-private-string-observations.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"case":case,"observations":observations,
            "evidence_class":"actual ordinary-object private ABI under seeded context; not historical million-event execution",
            "object_bytes_changed":false,"public_abi_changed":false})).expect("private observations"))
        .expect("retained private observations");
    count
}
fn compile(directory: &Path) {
    let mut machine = Command::new("/usr/bin/gcc");
    machine.arg("-dumpmachine");
    assert_eq!(success(machine, 10).stdout, b"x86_64-linux-gnu\n");
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_private_accounting");
    for name in ["observer_base.c", "private_probe.c"] {
        std::fs::copy(base.join(name), directory.join(name)).expect("retained private observer");
    }
    let object =
        std::fs::read(directory.join("program.o")).expect("ordinary object before linking");
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.args([
        "-std=c11",
        "-O0",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-no-pie",
        "-Wl,--build-id=none",
        "-Wl,--no-relax",
        "-I",
    ])
    .arg(directory)
    .arg(directory.join("private_probe.c"))
    .arg(directory.join("canonical-runtime.c"))
    .arg(directory.join("program.o"));
    for symbol in ["string_from_utf8_copy", "string_clone", "string_release"] {
        gcc.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
    }
    gcc.arg("-Wl,--wrap=malloc")
        .arg("-Wl,--wrap=free")
        .arg("-o")
        .arg(directory.join("private-program-native"));
    success(gcc, 30);
    assert_eq!(std::fs::read(directory.join("program.o")).expect("unchanged linked input"), object);
    let inputs = [
        "program.o",
        "program.zir",
        "private_binding.h",
        "private-binding.json",
        "private_probe.c",
        "observer_base.c",
        "canonical-runtime.c",
        "PRIVATE_EXPECTED_FREEZE.json",
        "private-program-native",
    ]
    .into_iter()
    .map(|name| {
        let bytes = std::fs::read(directory.join(name)).expect("actual private fixture input");
        (name, super::emit::digest(&bytes))
    })
    .collect::<std::collections::BTreeMap<_, _>>();
    std::fs::write(
        directory.join("private-inputs-before-execution.json"),
        serde_json::to_vec_pretty(&inputs).expect("private input hashes"),
    )
    .expect("retain linked private fixture inputs");
}
fn one(row: &serde_json::Value, directory: &Path) -> serde_json::Value {
    let id = row["id"].as_str().expect("frozen identifier");
    let path = directory.join(format!("{id}.actual-private.bin"));
    let mut command = Command::new(directory.join("private-program-native"));
    for value in row["pre"].as_array().expect("four seed counters") {
        command.arg(value.as_u64().expect("frozen seed").to_string());
    }
    command.arg(if row["allocation_fault_armed"] == true { "1" } else { "0" }).arg(&path);
    let output = process::run(command, 10);
    std::fs::write(directory.join(format!("{id}.stdout")), &output.stdout).expect("early stdout");
    std::fs::write(directory.join(format!("{id}.stderr")), &output.stderr).expect("early stderr");
    let binary = std::fs::read(&path);
    let receipt = serde_json::json!({"id":id,"expectations_checked":false,
        "exit_code":output.status.code(),"signal":output.status.signal(),"status":output.status.to_string(),
        "stdout_sha256":super::emit::digest(&output.stdout),"stderr_sha256":super::emit::digest(&output.stderr),
        "binary":match &binary {Ok(bytes)=>serde_json::json!({"bytes":bytes.len(),"sha256":super::emit::digest(bytes)}),
            Err(error)=>serde_json::json!({"read_error":error.to_string()})}});
    let receipt_path = directory.join(format!("{id}.actual-private-before-checks.json"));
    std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).expect("early receipt"))
        .expect("retain actual evidence");
    println!("native private accounting evidence before checks: {}", receipt_path.display());
    assert!(output.status.success(), "{:?}", output.status);
    assert!(output.stderr.is_empty() && output.stdout.is_empty());
    let bytes = binary.expect("actual private summary");
    let parent = row["physical_parent_setup"].as_bool().expect("setup role");
    assert_eq!(bytes.len(), 18 * 8 * if parent { 3 } else { 2 });
    let records = bytes
        .chunks_exact(18 * 8)
        .map(|record| {
            record
                .chunks_exact(8)
                .map(|field| u64::from_le_bytes(field.try_into().expect("u64")))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    check(row, &records);
    serde_json::json!({"id":id,"fields":records,"actual_status":records[1][1],"binary_sha256":super::emit::digest(&bytes)})
}
fn counters(value: &serde_json::Value) -> Vec<u64> {
    value.as_array().expect("four counters").iter().map(|v| v.as_u64().expect("counter")).collect()
}
fn check(row: &serde_json::Value, records: &[Vec<u64>]) {
    let expected = &row["expected"];
    let pre = &records[0];
    let post = &records[1];
    assert_eq!(pre[0], 0);
    assert_eq!(post[0], 1);
    assert_eq!(&pre[3..7], counters(&row["pre"]));
    assert_eq!(pre[7], u64::from(row["allocation_fault_armed"].as_bool().expect("fault flag")));
    assert_eq!(post[1], expected["status"].as_u64().expect("actual returned status"));
    assert_eq!(post[2], expected["out"].as_u64().expect("publication"));
    assert_eq!(&post[3..7], counters(&expected["post"]));
    assert_eq!(
        post[7],
        u64::from(expected["fault_armed_after"].as_bool().expect("fault precedence"))
    );
    for (slot, name) in [
        (8, "from"),
        (9, "clone"),
        (10, "release"),
        (11, "attempts"),
        (12, "allocations"),
        (13, "frees"),
    ] {
        assert_eq!(
            post[slot].checked_sub(pre[slot]).expect("monotonic real event"),
            expected["step_delta"][name].as_u64().expect("pre-output event count"),
            "{name}"
        );
    }
    assert_eq!(post[14], expected["boundary_live"].as_u64().expect("live"));
    assert_eq!(post[15], expected["boundary_owners"].as_u64().expect("owners"));
    if !expected["cleanup_post"].is_null() {
        let cleanup = &records[2];
        assert_eq!(cleanup[0], 2);
        assert_eq!(cleanup[1], 0);
        assert_eq!(cleanup[2], 1);
        assert_eq!(&cleanup[3..7], counters(&expected["cleanup_post"]));
        assert_eq!(cleanup[8], post[8]);
        assert_eq!(cleanup[9], post[9]);
        assert_eq!(cleanup[10], post[10] + 1);
        assert_eq!(cleanup[11], post[11]);
        assert_eq!(cleanup[12], post[12]);
        assert_eq!(cleanup[13], post[13] + 1);
        assert_eq!(cleanup[14], 0);
        assert_eq!(cleanup[15], 0);
    }
}
