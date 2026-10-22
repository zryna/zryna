//! Actual host-process preflight failures retain a closed non-secret cleanup acknowledgement.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn execute(path: &Path, pin: &str, operation: &str, payload: &[u8]) -> serde_json::Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_zryna-playground-compiler"))
        .args([
            "--daemon-approval",
            path.to_str().expect("fixture path"),
            "--approval-sha256",
            pin,
            "--operation",
            operation,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("actual app host");
    let mut control = child.stdin.take().expect("owned host input");
    control
        .write_all(&u32::try_from(payload.len()).expect("finite input").to_le_bytes())
        .expect("framing");
    control.write_all(payload).expect("payload");
    // Keep the cancellation pipe open through the actual process terminal acknowledgement.
    let output = child.wait_with_output().expect("actual host terminal");
    drop(control);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "no fabricated compiler bytes");
    assert!(output.stderr.len() <= 4096);
    let record: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("one terminal JSON");
    assert_eq!(record.as_object().expect("closed terminal").len(), 7);
    assert_eq!(record["format"], "zryna.playground-host-error.v1");
    assert_eq!(record["cleanupConfirmed"], true);
    assert!(record["owner"].is_null());
    record
}

#[test]
fn malformed_frames_and_closed_requests_reject_before_approval_discovery() {
    let missing = Path::new("/nonexistent-zryna-daemon-approval.json");
    let pin = "a".repeat(64);
    for (operation, payload, reason) in [
        ("compile", &b"{}"[..], "PLAYGROUND-REQUEST-SHAPE"),
        (
            "compile",
            &b"{\"version\":1,\"revision\":1,\"source\":\"\",\"argv\":[]}"[..],
            "PLAYGROUND-REQUEST-SHAPE",
        ),
        ("evaluate", &b"{\"version\":1}"[..], "PLAYGROUND-EVALUATION-REQUEST"),
    ] {
        assert_eq!(execute(missing, &pin, operation, payload)["reason"], reason);
    }
}

#[test]
fn independent_approval_pin_rejects_without_leaking_record_or_contacting_daemon() {
    let root =
        std::env::temp_dir().join(format!("playground-host-terminal-{}", std::process::id()));
    std::fs::create_dir(&root).expect("owned fixture");
    let path = root.join("approval.json");
    let secret = b"{\"fixtureSecret\":\"must-not-appear-in-terminal\"}";
    std::fs::write(&path, secret).expect("inert operator fixture");
    let payload = b"{\"version\":1,\"revision\":1,\"source\":\"fn main() -> i32 { 1 }\"}";
    let wrong = execute(&path, &"0".repeat(64), "compile", payload);
    assert_eq!(wrong["reason"], "PLAYGROUND-HOST-APPROVAL");
    let exact = execute(&path, &format!("{:x}", Sha256::digest(secret)), "compile", payload);
    assert_eq!(exact["reason"], "PLAYGROUND-HOST-APPROVAL");
    assert!(!exact.to_string().contains("fixtureSecret"));
    assert!(!exact.to_string().contains("must-not-appear"));
    std::fs::remove_dir_all(root).expect("owned fixture cleanup");
}
