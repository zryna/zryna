//! Actual ordinary and observed execution; no producer-only assertion substitutes for runtime.
use super::{emit::Artifacts, process};
use std::{path::Path, process::Command};
pub(super) fn all(case: &str, artifacts: &Artifacts) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_borrowed_match_targets");
    for (script, input) in [
        ("javascript-ordinary.mjs", "program-ordinary.mjs"),
        ("javascript-observed.mjs", "program-observed.mjs"),
        ("webassembly.mjs", "program-ordinary.wasm"),
    ] {
        let mut command = Command::new("node");
        command.arg(root.join(script)).arg(artifacts.directory.join(input)).arg(case).arg("faults");
        let output = process::run(command, 20);
        assert!(
            output.status.success(),
            "{script}/{case}: {:?}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
        let receipt: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("actual target observations JSON");
        assert_eq!(receipt["case"], case);
        assert_eq!(receipt["pristineRetries"], true);
        std::fs::write(
            artifacts.directory.join(format!("{script}.observations.json")),
            output.stdout,
        )
        .expect("retained actual individual target observations");
    }
}
