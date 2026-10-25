//! Reuse the exact frozen actual-pointer observer, without any portable backend binding.
use super::{emit::Artifacts, process, runtime_oracle};
use std::{
    path::PathBuf,
    process::{Command, Output},
};
#[path = "../generic_execution_v1_first_vertical/execute/native.rs"]
mod native;
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
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.arg("-dumpmachine");
    assert_eq!(successful(gcc, 10).stdout, b"x86_64-linux-gnu\n");
}
pub(super) fn all(case: &str, artifacts: &Artifacts) -> (usize, usize) {
    native::all(case, artifacts)
}
