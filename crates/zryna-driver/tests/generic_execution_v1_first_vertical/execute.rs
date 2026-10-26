//! Exact fixed source observations; every fault is followed by a pristine retry.
use super::{authority, emit::Artifacts, process, runtime_oracle};
use std::{
    path::PathBuf,
    process::{Command, Output},
};
#[path = "execute/native.rs"]
mod native;
pub(super) struct Counts {
    pub(super) observations: [usize; 3],
    pub(super) native_faults: usize,
}
fn root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_first_vertical")
}
fn successful(command: Command, seconds: u64) -> Output {
    let output = process::run(command, seconds);
    assert!(
        output.status.success(),
        "child failed {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected child diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
pub(super) fn check_tools() {
    let mut node = Command::new("node");
    node.arg("--version");
    assert_eq!(successful(node, 10).stdout, b"v22.22.1\n");
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.arg("-dumpmachine");
    assert_eq!(successful(gcc, 10).stdout, b"x86_64-linux-gnu\n");
}
pub(super) fn all(case: &str, artifacts: &Artifacts) -> Counts {
    let count = 1 + 2 * runtime_oracle::faults(case).len();
    let mut observations = [0; 3];
    for (index, target) in ["javascript", "webassembly"].into_iter().enumerate() {
        let mut command = Command::new("node");
        command.arg(root().join("portable.mjs")).arg(target).arg(&artifacts.directory).arg(case);
        let output = successful(command, 10);
        let receipt: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("real portable observation receipt");
        assert_eq!(receipt["target"], target);
        assert_eq!(receipt["case"], case);
        assert_eq!(receipt["actualAcquisitionsReleasedExactlyOnce"], true);
        let rows =
            receipt["observations"].as_array().expect("every actual fault/retry observation");
        assert_eq!(rows.len(), count);
        for (ordinal, row) in rows.iter().enumerate() {
            let success = ordinal % 2 == 0;
            assert_eq!(row["outcome"], if success { "return" } else { "trap" });
            assert_eq!(row["actualAcquisitionsReleasedExactlyOnce"], true);
            if success {
                assert_eq!(row["result"], runtime_oracle::result(case));
            } else {
                assert!(row.get("result").is_none());
                let expected = authority::case(case);
                assert_eq!(row["fault"], expected["faults"][ordinal / 2]);
            }
        }
        observations[index] = rows.len();
        std::fs::write(
            artifacts.directory.join(format!("{target}-observations.json")),
            output.stdout,
        )
        .expect("retained individual portable observations");
    }
    let (native_count, native_faults) = native::all(case, artifacts);
    observations[2] = native_count;
    assert_eq!(observations, [count; 3]);
    Counts { observations, native_faults }
}
