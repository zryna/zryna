//! Actual public frontend/factory and Node/Chrome generated-loader byte-boundary proof.

use super::{WorkspaceCase, assert_success, command_output, node_executable, read_json};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const SOURCE_SHA: &str = "ef631a1b99fbf55bff698e50c0bd336f1ba32b1ead1f44c9db2f047e3534ecc8";
const EXTRA_SOURCE_SHA: &str = "4a7955dc78e5a66ded0b11671914993e4ac20c151f1909c4db0399a74dc1e306";
const COMPONENT_SHA: &str = "128b13f47a9195a2e927bf6cdea34588a7194656ad8b265b8e346c8c43d93c97";
const CORE_SHA: &str = "5877a25cc493884b22885d1e1a3a68c2d171ce80ee4ae0e4cd7e1e6e48590b8a";
const INTERFACE_SHA: &str = "3557a78a99ec1cf774ee360c879f0484de5592b2d76bbb59fec8616015969c05";
const WIT_SHA: &str = "72e017162d5080960589e749c8b5a4f662e5f604bf317e7f7adcd085246b3a24";

fn source(wide: usize) -> String {
    let mut text = String::new();
    let suffix = "x".repeat(123);
    for index in 0..1_863 {
        writeln!(
            &mut text,
            "export function f{index:04x}{suffix}(): i32 {{ return {}; }}",
            if index < wide { 64 } else { 0 }
        )
        .expect("writing ordinary source");
    }
    text
}

fn build(case: &WorkspaceCase, stem: &str) -> std::process::Output {
    command_output(
        case,
        &[
            "build",
            &case.source_relative,
            "--profile",
            "browser-component-v1",
            "--target",
            "component",
            "--name",
            stem,
            "--json",
        ],
    )
}

fn artifacts(case: &WorkspaceCase, stem: &str) -> (PathBuf, PathBuf) {
    let directory = case.bundle(stem, "build").join("component");
    (directory.join(format!("{stem}.mjs")), directory.join(format!("{stem}.wasm")))
}

fn check_exact(case: &WorkspaceCase, stem: &str) -> Value {
    let bundle = case.bundle(stem, "build");
    let manifest = read_json(&bundle.join("zryna-browser-manifest-v1.json"));
    assert_eq!(manifest["profile"], "zryna-browser-component-v1");
    assert_eq!(manifest["source_sha256"], SOURCE_SHA);
    assert_eq!(manifest["browser"]["world"], "zryna:capability-profiles/browser@0.1.0");
    assert_eq!(manifest["browser"]["wit_source_sha256"], WIT_SHA);
    assert_eq!(manifest["browser"]["core_sha256"], CORE_SHA);
    assert_eq!(manifest["browser"]["interface_sha256"], INTERFACE_SHA);
    assert_eq!(manifest["browser"]["component_sha256"], COMPONENT_SHA);
    let records = manifest["artifacts"].as_array().expect("complete artifact records");
    assert_eq!(records.len(), 3);
    for record in records {
        let bytes = fs::read(bundle.join(record["path"].as_str().expect("artifact path")))
            .expect("actual published artifact");
        assert_eq!(record["bytes"], bytes.len());
        assert_eq!(record["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    }
    let component = fs::read(artifacts(case, stem).1).expect("actual factory component");
    assert_eq!(component.len(), 1_048_576);
    assert_eq!(format!("{:x}", Sha256::digest(&component)), COMPONENT_SHA);
    manifest
}

pub(super) fn run(browser_root: Option<&Path>) {
    let mut case = WorkspaceCase::new();
    let exact_text = source(106);
    let extra_text = source(107);
    assert_eq!(exact_text.len(), 309_364);
    assert_eq!(extra_text.len(), 309_365);
    assert_eq!(format!("{:x}", Sha256::digest(exact_text.as_bytes())), SOURCE_SHA);
    assert_eq!(format!("{:x}", Sha256::digest(extra_text.as_bytes())), EXTRA_SOURCE_SHA);
    fs::write(case.root.join(&case.source_relative), &exact_text).expect("ordinary exact source");
    let exact = case.stem("browser_byte_exact", "build");
    assert_success(&build(&case, &exact));
    let first = check_exact(&case, &exact);
    let replay = case.stem("browser_byte_replay", "build");
    assert_success(&build(&case, &replay));
    let second = check_exact(&case, &replay);
    assert_eq!(
        first["browser"], second["browser"],
        "fresh public factories replay identical bindings"
    );
    assert_eq!(
        fs::read(artifacts(&case, &exact).0).expect("first loader"),
        fs::read(artifacts(&case, &replay).0).expect("replayed loader")
    );
    fs::write(case.root.join(&case.source_relative), extra_text)
        .expect("ordinary first-extra source");
    let recovery = case.stem("browser_byte_recovery", "build");
    let rejected = build(&case, &recovery);
    assert_eq!(rejected.status.code(), Some(4));
    let response: Value =
        serde_json::from_slice(&rejected.stdout).expect("actual factory rejection");
    assert_eq!(response["diagnostics"][0]["code"], "ZRYNA-W4021");
    assert_eq!(response["diagnostics"][0]["message"], "scalar component exceeds 1048576 bytes");
    assert_eq!(response["ok"], false);
    assert!(response["manifest"].is_null());
    assert!(!case.bundle(&recovery, "build").exists(), "first extra publishes no bundle");
    fs::write(
        case.root.join(&case.source_relative),
        include_str!("../../../../examples/universal/add.zry"),
    )
    .expect("fresh ordinary recovery source");
    assert_success(&build(&case, &recovery));
    let (loader, component) = artifacts(&case, &exact);
    let (ordinary_loader, ordinary_component) = artifacts(&case, &recovery);
    let rendered = |path: &Path| path.to_string_lossy().trim_start_matches(r"\\?\").to_owned();
    let output = Command::new(node_executable())
        .arg(rendered(&case.root.join("tests/browser-component/byte-boundary.mjs")))
        .args([
            rendered(&loader),
            rendered(&component),
            rendered(&ordinary_loader),
            rendered(&ordinary_component),
        ])
        .args(browser_root.map(rendered))
        .current_dir(&case.root)
        .output()
        .expect("actual generated public loader boundary fixture");
    assert_success(&output);
    let observed: Value =
        serde_json::from_slice(&output.stdout).expect("closed Node/Chrome boundary observations");
    let host = json!({"executedExports":1863,"wideReturns":106,"narrowReturns":1757,
        "firstExtraRejection":"ZRYNA-B3991","ordinaryRecovery":42});
    assert_eq!(observed["node"], host);
    assert_eq!(observed["browser"], if browser_root.is_some() { host } else { Value::Null });
    println!(
        "browserByteBoundary={}",
        json!({
            "schema":"zryna.browser-byte-boundary-observation.v1",
            "sourceSha256":SOURCE_SHA,"firstExtraSourceSha256":EXTRA_SOURCE_SHA,
            "sourceBytes":309_364,"firstExtraSourceBytes":309_365,"functions":1863,
            "world":"zryna:capability-profiles/browser@0.1.0","witSourceSha256":WIT_SHA,
            "coreSha256":CORE_SHA,"interfaceSha256":INTERFACE_SHA,
            "componentBytes":observed["componentBytes"],"componentSha256":observed["componentSha256"],
            "firstExtraComponentBytes":observed["firstExtraComponentBytes"],
            "firstExtraComponentSha256":observed["firstExtraComponentSha256"],
            "factory":{"exactAccepted":true,"replayIdentical":true,"firstExtraExitCode":4,
                "firstExtraRejection":"ZRYNA-W4021","firstExtraBundleAbsent":true,"ordinaryRecovery":true},
            "node":observed["node"],"browser":observed["browser"],
            "cleanup":{"browserConfirmed":browser_root.is_some(),"childExitCode":output.status.code()}
        })
    );
}
