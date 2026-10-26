//! Capture all actual ordinary-object private results before comparing independent frozen expectations.
use super::{emit::Artifacts, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
#[path = "check.rs"]
mod check;
#[path = "compile.rs"]
mod compile;
pub(super) fn all(case: &str, artifact: &Artifacts) -> usize {
    let directory = &artifact.directory;
    let bytes = include_bytes!("PRIVATE_EXPECTED_FREEZE.json");
    assert_eq!(
        super::emit::digest(bytes),
        "3ac2764bf965720ff991d5b11bf6ad19cc28d9d7137be46684000297895ac694"
    );
    std::fs::write(directory.join("PRIVATE_EXPECTED_FREEZE.json"), bytes)
        .expect("retain pre-output oracle");
    compile::compile(directory);
    let oracle: serde_json::Value =
        serde_json::from_slice(bytes).expect("fixed eight source probes");
    let mut actual = Vec::new();
    for row in oracle["cases"].as_array().expect("eight") {
        if row["case"] == case {
            actual.push((row, one(row, directory)));
        }
    }
    let observations = actual
        .iter()
        .map(|(row, run)| serde_json::json!({"id":row["id"],"observation":run}))
        .collect::<Vec<_>>();
    std::fs::write(directory.join("actual-private-control-observations.json"),serde_json::to_vec_pretty(&serde_json::json!({
    "case":case,"observations":observations,"expectations_checked":false,"object_bytes_changed":false,
    "evidence_class":"actual ordinary-object private ABI, seeded invocation counters, fixed canonical observer and real malloc fault"
 })).expect("actual observation receipt")).expect("retain all case observations before checks");
    for (row, run) in &actual {
        check::check(row, run);
    }
    actual.len()
}
fn one(row: &serde_json::Value, directory: &Path) -> serde_json::Value {
    let id = row["id"].as_str().expect("frozen identifier");
    let path = directory.join(format!("{id}.actual-private.bin"));
    let mut command = Command::new(directory.join("private-program-native"));
    for value in row["pre"].as_array().expect("four seeds") {
        command.arg(value.as_u64().expect("seed").to_string());
    }
    command
        .arg(row["malloc_fault_ordinal"].as_u64().expect("fault ordinal").to_string())
        .arg(&path);
    let output = process::run(command, 10);
    std::fs::write(directory.join(format!("{id}.stdout")), &output.stdout).expect("retain stdout");
    std::fs::write(directory.join(format!("{id}.stderr")), &output.stderr).expect("retain stderr");
    let binary = std::fs::read(path);
    let fields = binary.as_ref().ok().map(|bytes| {
        bytes
            .chunks_exact(8)
            .map(|n| u64::from_le_bytes(n.try_into().expect("u64")))
            .collect::<Vec<_>>()
    });
    let receipt = serde_json::json!({"id":id,"exit_code":output.status.code(),"signal":output.status.signal(),
    "status":output.status.to_string(),"stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len(),
    "binary_bytes":binary.as_ref().ok().map(Vec::len),"binary_sha256":binary.as_ref().ok().map(|b|super::emit::digest(b)),
    "binary_read_error":binary.as_ref().err().map(ToString::to_string),"fields":fields,
    "expectations_checked":false});
    let receipt_path = directory.join(format!("{id}.actual-private-before-checks.json"));
    std::fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("actual early receipt"),
    )
    .expect("retain before checking any result");
    println!("native control private observation: {}", receipt_path.display());
    receipt
}
