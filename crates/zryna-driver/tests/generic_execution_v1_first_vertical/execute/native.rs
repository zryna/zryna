//! Separate native processes retain actual pointer/release records before exact SIGILL.
use super::{Artifacts, Command, process, root, runtime_oracle, successful};
use std::os::unix::process::ExitStatusExt;
pub(super) fn all(case: &str, artifacts: &Artifacts) -> (usize, usize) {
    let executable = artifacts.directory.join("program-native");
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
    .arg(&artifacts.directory)
    .arg(root().join("runtime.c"))
    .arg(artifacts.directory.join("program.o"))
    .arg("-o")
    .arg(&executable);
    successful(gcc, 30);
    let mut observations = vec![one(case, artifacts, None, "initial")];
    let faults = runtime_oracle::faults(case);
    for fault in &faults {
        let label = format!("{}-{}", fault.kind, fault.ordinal);
        observations.push(one(case, artifacts, Some(fault), &label));
        observations.push(one(case, artifacts, None, &format!("retry-after-{label}")));
    }
    assert_eq!(observations.len(), 1 + 2 * faults.len());
    let count = observations.len();
    let receipt = serde_json::json!({"target":"native","case":case,"actualAcquisitionsReleasedExactlyOnce":true,"observations":observations});
    std::fs::write(
        artifacts.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&receipt).expect("native receipt JSON"),
    )
    .expect("retained native evidence");
    (count, faults.len())
}
fn one(
    case: &str,
    artifacts: &Artifacts,
    fault: Option<&runtime_oracle::Fault>,
    name: &str,
) -> serde_json::Value {
    let trace = artifacts.directory.join(format!("native-{name}.identities.jsonl"));
    let mut command = Command::new(artifacts.directory.join("program-native"));
    command
        .arg(fault.map_or("none", |fault| fault.kind.as_str()))
        .arg(fault.map_or(0, |fault| fault.ordinal).to_string())
        .arg(runtime_oracle::result(case).to_string())
        .arg(runtime_oracle::allocations(case).to_string())
        .arg(&trace);
    let output = process::run(command, 10);
    assert!(
        output.stderr.is_empty(),
        "native diagnostics {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if fault.is_some() {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "cleanup completed before controlled allocation-category native trap"
        );
        assert!(output.stdout.is_empty());
    } else {
        assert!(output.status.success());
        assert_eq!(output.stdout, format!("result:{}\n", runtime_oracle::result(case)).as_bytes());
    }
    let bytes = std::fs::read(&trace).expect("unbuffered real native pointer trace");
    assert!(bytes.len() <= 65_536);
    let text = String::from_utf8(bytes).expect("native identity JSONL");
    let records = text
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).expect("actual runtime identity record")
        })
        .collect::<Vec<_>>();
    runtime_oracle::check(case, fault, &records);
    std::fs::write(artifacts.directory.join(format!("native-{name}.stdout")), &output.stdout)
        .expect("retained native stdout");
    serde_json::json!({"name":name,"outcome":if fault.is_some() {"trap"} else {"return"},"result":fault.is_none().then(||runtime_oracle::result(case)),"signal":output.status.signal(),"exit_code":output.status.code(),"fault":fault.map(|fault|serde_json::json!({"kind":fault.kind,"ordinal":fault.ordinal})),"records":records,"actualAcquisitionsReleasedExactlyOnce":true})
}
