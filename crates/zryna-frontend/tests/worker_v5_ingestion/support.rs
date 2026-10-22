//! Authenticated production worker and hostile fixture launch support.

use serde_json::Value;
use std::{ffi::OsString, path::PathBuf, process::Command};
use zryna_frontend::{ProviderExpectationV5, WorkerFrontendV5, WorkerLimitsV5, WorkerSpecV5};
use zryna_source::{SourceFileInput, SourceMap};

pub(super) fn root() -> PathBuf {
    // Cargo supplies an absolute path; canonicalization adds a Windows verbatim prefix that
    // pinned Node rejects before its entrypoint can emit the authenticated handshake.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root")
        .to_path_buf()
}
pub(super) fn node() -> PathBuf {
    let output = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .expect("pinned Node is required");
    assert!(output.status.success());
    PathBuf::from(String::from_utf8(output.stdout).expect("Node path").trim())
}
pub(super) fn fixture(name: &str) -> (SourceMap, Value) {
    let corpus: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/provider-conformance-v5/corpus.json")).expect("corpus"),
    )
    .expect("JSON");
    let entry = corpus["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|entry| entry["id"] == name)
        .expect("case");
    let raw: Value = serde_json::from_slice(
        &std::fs::read(root().join(entry["reference"].as_str().expect("reference")))
            .expect("frozen DTO"),
    )
    .expect("JSON");
    let inputs = entry["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|unit| {
            let path = unit["path"].as_str().expect("path");
            let text = unit["fragments"]
                .as_array()
                .expect("fragments")
                .iter()
                .map(|fragment| {
                    fragment["text"].as_str().map_or_else(
                        || {
                            std::fs::read_to_string(
                                root().join(fragment["source"].as_str().expect("source path")),
                            )
                            .expect("source")
                        },
                        str::to_owned,
                    )
                })
                .collect();
            SourceFileInput { path: path.into(), text }
        })
        .collect();
    (SourceMap::build(inputs).expect("source map"), raw)
}
pub(super) fn worker(arguments: Vec<OsString>, limits: WorkerLimitsV5) -> WorkerFrontendV5 {
    WorkerFrontendV5::new(
        WorkerSpecV5::new(
            node(),
            arguments,
            root(),
            ProviderExpectationV5::new("typescript-6", "6.0.3").expect("identity"),
            limits,
        )
        .expect("bounded spec"),
    )
}
pub(super) fn production() -> WorkerFrontendV5 {
    worker(
        vec![root().join("adapters/typescript-6/src/worker-v5.mjs").into()],
        WorkerLimitsV5::default(),
    )
}
pub(super) fn launch_diagnostic() -> String {
    if !cfg!(windows) {
        return "Windows launch probe not applicable".into();
    }
    let output = Command::new(node())
        .args(["-e", include_str!("../fixtures/worker-v5-launch-diagnostic.cjs")])
        .arg(root())
        .arg(root().join("adapters/typescript-6/src/worker-v5.mjs"))
        .output()
        .expect("failure-only launch probe");
    format!(
        "status={:?}; stdout={}; stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
pub(super) fn hostile(
    mode: &str,
    limits: WorkerLimitsV5,
    seen: Option<&PathBuf>,
) -> WorkerFrontendV5 {
    let mut args = vec![
        root().join("crates/zryna-frontend/tests/fixtures/worker-v5-hostile.mjs").into(),
        mode.into(),
        root().join("tests/m7-syntax-fixtures/reference.json").into(),
    ];
    if let Some(path) = seen {
        args.push(path.into());
    }
    worker(args, limits)
}
