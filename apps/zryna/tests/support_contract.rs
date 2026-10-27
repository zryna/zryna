//! Executable contract tests for static support reporting outside a compiler checkout.

#![forbid(unsafe_code)]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct OutsideCheckout(PathBuf);

impl OutsideCheckout {
    fn new() -> Self {
        let sequence = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("zryna-support-contract-{}-{sequence}", std::process::id()));
        fs::create_dir(&root).expect("fresh outside-checkout directory");
        Self(root)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_zryna"));
        command.current_dir(&self.0);
        command
    }

    fn assert_empty(&self) {
        assert_eq!(fs::read_dir(&self.0).expect("test directory").count(), 0);
    }
}

impl Drop for OutsideCheckout {
    fn drop(&mut self) {
        fs::remove_dir(&self.0).expect("support must leave the directory empty");
    }
}

#[test]
fn support_json_reports_exact_compiled_contract_outside_checkout_without_side_effects() {
    let case = OutsideCheckout::new();
    let output = case.command().args(["support", "--json"]).output().expect("support starts");
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let page = include_str!("../../../docs/M4_SUPPORT.md");
    let (_, fence) = page
        .split_once("zryna.m4.support.v1:begin\n\n```json\n")
        .expect("independent canonical support fence");
    let (canonical, _) =
        fence.split_once("```\n\nzryna.m4.support.v1:end").expect("canonical support fence end");
    assert!(
        output.stdout == canonical.as_bytes(),
        "public support bytes must retain the complete canonical Markdown contract"
    );
    let document: Value = serde_json::from_slice(&output.stdout).expect("support JSON");
    assert_eq!(document["authority"], "static-contract");
    assert_eq!(document["authenticatedEvidence"], "not-evaluated");
    assert_eq!(document["hosts"].as_array().expect("hosts").len(), 2);
    assert_eq!(document["rows"].as_array().expect("rows").len(), 12);
    assert!(document.get("sourceCommit").is_none());
    assert!(document.get("sourceTree").is_none());
    for row in document["rows"].as_array().expect("rows") {
        assert_eq!(row["authenticatedExecution"], "not-evaluated");
    }
    case.assert_empty();
}

#[test]
fn support_rejects_execution_authority_and_evidence_selectors() {
    let case = OutsideCheckout::new();
    for arguments in [
        vec!["support", "main.zry"],
        vec!["support", "--root", "."],
        vec!["support", "--node", "node"],
        vec!["support", "--grant-file", "private.json"],
        vec!["support", "--profile", "server-status-v1"],
        vec!["support", "--evidence", "receipt.json"],
        vec!["support", "--json", "--json"],
    ] {
        let output = case.command().args(&arguments).output().expect("support rejection starts");
        assert_eq!(output.status.code(), Some(2), "{arguments:?}: {output:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    case.assert_empty();
}

#[test]
fn support_text_and_help_keep_static_evidence_boundary_visible() {
    let case = OutsideCheckout::new();
    let text = case.command().arg("support").output().expect("text support starts");
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).expect("UTF-8 text");
    assert!(text.contains("static support contract"));
    assert!(text.contains("not authenticated"));
    assert!(text.contains("immutable beta H1 remain excluded"));
    let help = case.command().args(["support", "--help"]).output().expect("support help starts");
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout).expect("help UTF-8").contains("--json"));
    case.assert_empty();
}
