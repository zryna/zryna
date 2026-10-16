//! Fixed failure capture for the ignored pinned-browser fixture only.
//! This module is compiled only through inline.rs's cfg(test) declaration.

use super::BoundedOutput;
use sha2::{Digest as _, Sha256};

const STDOUT_LIMIT: usize = 64 * 1024;
const STDERR_LIMIT: usize = 16 * 1024;

pub(super) fn record_failure(output: &BoundedOutput) {
    use std::io::Write as _;

    let Some(record) = failure_record(output) else {
        return;
    };
    // The original R3006 is returned even if this diagnostic write fails.
    let _ = writeln!(std::io::stderr().lock(), "browserFixtureRuntimeCapture={record}");
}

fn failure_record(output: &BoundedOutput) -> Option<serde_json::Value> {
    // Existing browser capture owns these same limits. Refuse a malformed
    // internal value rather than create a larger diagnostic or alter its error.
    if output.stdout.len() > STDOUT_LIMIT || output.stderr.len() > STDERR_LIMIT {
        return None;
    }
    #[cfg(windows)]
    let windows_raw_status =
        output.status.code().map(|code| u32::from_ne_bytes(code.to_ne_bytes()));
    #[cfg(not(windows))]
    let windows_raw_status: Option<u32> = None;

    Some(serde_json::json!({
        "format": "zryna.test-browser-runtime-failure.v1",
        "diagnosticCode": "ZRYNA-R3006",
        "capture": {
            "statusContext": "bounded-runner-final-returned-status-after-cleanup",
            "success": output.status.success(),
            "exitCode": output.status.code(),
            "windowsRawStatus": windows_raw_status,
            "stdoutBytes": output.stdout.len(),
            "stderrBytes": output.stderr.len(),
            "stdoutSha256": format!("{:x}", Sha256::digest(&output.stdout)),
            "stderrSha256": format!("{:x}", Sha256::digest(&output.stderr)),
            "stdoutHex": hex(&output.stdout),
            "stderrHex": hex(&output.stderr),
        },
    }))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{BoundedOutput, STDERR_LIMIT, STDOUT_LIMIT, failure_record};
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn status(code: u32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt as _;
        ExitStatus::from_raw(i32::try_from(code).expect("bounded Unix test exit code") << 8)
    }

    #[cfg(windows)]
    fn status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt as _;
        ExitStatus::from_raw(code)
    }

    fn output(code: u32, stdout: Vec<u8>, stderr: Vec<u8>) -> BoundedOutput {
        BoundedOutput { status: status(code), stdout, stderr }
    }

    #[test]
    fn nonzero_final_status_is_recorded_without_stderr() {
        let record = failure_record(&output(7, b"partial".to_vec(), Vec::new()))
            .expect("bounded failed output");
        assert_eq!(record["diagnosticCode"], "ZRYNA-R3006");
        assert_eq!(record["capture"]["success"], false);
        assert_eq!(record["capture"]["exitCode"], 7);
        assert_eq!(record["capture"]["stderrBytes"], 0);
        assert_eq!(record["capture"]["stdoutHex"], "7061727469616c");
        assert_eq!(
            record["capture"]["statusContext"],
            "bounded-runner-final-returned-status-after-cleanup"
        );
    }

    #[test]
    fn stderr_only_rejection_keeps_status_zero_and_lossless_binary() {
        let record = failure_record(&output(0, vec![0, 255], vec![0, 128, 255, 10]))
            .expect("bounded stderr rejection");
        assert_eq!(record["capture"]["success"], true);
        assert_eq!(record["capture"]["exitCode"], 0);
        assert_eq!(record["capture"]["stdoutBytes"], 2);
        assert_eq!(record["capture"]["stderrBytes"], 4);
        assert_eq!(record["capture"]["stdoutHex"], "00ff");
        assert_eq!(record["capture"]["stderrHex"], "0080ff0a");
        assert_eq!(
            record["capture"]["stdoutSha256"],
            "06eb7d6a69ee19e5fbdf749018d3d2abfa04bcbd1365db312eb86dc7169389b8"
        );
    }

    #[test]
    fn exact_capture_ceilings_remain_bounded_and_complete() {
        let record = failure_record(&output(7, vec![b'o'; STDOUT_LIMIT], vec![b'e'; STDERR_LIMIT]))
            .expect("inclusive capture limits");
        assert_eq!(record["capture"]["stdoutBytes"], STDOUT_LIMIT);
        assert_eq!(record["capture"]["stderrBytes"], STDERR_LIMIT);
        assert_eq!(record["capture"]["stdoutHex"].as_str().expect("hex").len(), STDOUT_LIMIT * 2);
        assert_eq!(record["capture"]["stderrHex"].as_str().expect("hex").len(), STDERR_LIMIT * 2);
        assert!(serde_json::to_vec(&record).expect("record").len() < 166 * 1024);
    }

    #[test]
    fn first_extra_stdout_or_stderr_refuses_serialization() {
        assert!(failure_record(&output(7, vec![0; STDOUT_LIMIT + 1], Vec::new())).is_none());
        assert!(failure_record(&output(7, Vec::new(), vec![0; STDERR_LIMIT + 1])).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn high_bit_windows_status_is_kept_as_unsigned_raw_status() {
        let record = failure_record(&output(0xc000_0005, Vec::new(), Vec::new()))
            .expect("bounded failed output");
        assert_eq!(record["capture"]["success"], false);
        assert_eq!(record["capture"]["windowsRawStatus"], 0xc000_0005_u32);
        assert_eq!(record["capture"]["exitCode"], -1_073_741_819_i32);
    }
}
