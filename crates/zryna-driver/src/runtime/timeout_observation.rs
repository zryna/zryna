//! Test-only bounded observation of an already-cleaned-up runtime timeout.

use std::{io, process::ExitStatus};

use super::{Captured, Pending};

const MAX_RECORD_BYTES: usize = 2048;
const PREFIX_BYTES: usize = 64;

pub(super) fn record_failure(
    pending: &Pending,
    input_result: &Option<io::Result<()>>,
    status: &ExitStatus,
    stdout: &Captured,
    stderr: &Captured,
) {
    if let Some(record) = failure_record(pending, input_result, status, stdout, stderr) {
        // libtest retains this diagnostic for failed tests; expected-timeout tests keep it captured.
        eprintln!("{record}");
    }
}

fn failure_record(
    pending: &Pending,
    input_result: &Option<io::Result<()>>,
    status: &ExitStatus,
    stdout: &Captured,
    stderr: &Captured,
) -> Option<String> {
    let (input_outcome, input_error_kind) = match input_result {
        None => ("none", "null".to_owned()),
        Some(Ok(())) => ("ok", "null".to_owned()),
        // ErrorKind is a fixed standard-library classification, never the unbounded error text.
        Some(Err(error)) => ("error", format!("\"{:?}\"", error.kind())),
    };
    let exit_code = status.code().map_or_else(|| "null".to_owned(), |code| code.to_string());
    #[cfg(windows)]
    let windows_raw_status = status.code().map_or_else(
        || "null".to_owned(),
        |code| u32::from_ne_bytes(code.to_ne_bytes()).to_string(),
    );
    #[cfg(not(windows))]
    let windows_raw_status = "null";
    let record = format!(
        "runtimeTimeoutCapture={{\"format\":\"zryna.test-runtime-timeout.v1\",\
         \"diagnosticCode\":\"ZRYNA-R3003\",\"timedOut\":{},\"outputExceeded\":{},\
         \"inputResult\":\"{input_outcome}\",\"inputErrorKind\":{input_error_kind},\
         \"statusContext\":\"actual-post-cleanup-returned-status-not-evidence-of-natural-or-pre-timeout-exit\",\
         \"success\":{},\"exitCode\":{exit_code},\"windowsRawStatus\":{windows_raw_status},\
         \"stdout\":{{\"bytes\":{},\"exceeded\":{},\"prefixBytes\":{},\"prefixTruncated\":{},\"prefixHex\":\"{}\"}},\
         \"stderr\":{{\"bytes\":{},\"exceeded\":{},\"prefixBytes\":{},\"prefixTruncated\":{},\"prefixHex\":\"{}\"}}}}",
        pending.timed_out,
        pending.output_exceeded,
        status.success(),
        stdout.bytes.len(),
        stdout.exceeded,
        stdout.bytes.len().min(PREFIX_BYTES),
        stdout.bytes.len() > PREFIX_BYTES,
        prefix_hex(&stdout.bytes),
        stderr.bytes.len(),
        stderr.exceeded,
        stderr.bytes.len().min(PREFIX_BYTES),
        stderr.bytes.len() > PREFIX_BYTES,
        prefix_hex(&stderr.bytes),
    );
    // Include the diagnostic's final LF in the complete record ceiling.
    (record.len() < MAX_RECORD_BYTES).then_some(record)
}

fn prefix_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().min(PREFIX_BYTES) * 2);
    for byte in bytes.iter().take(PREFIX_BYTES) {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn timeout_record_bounds_capture_and_error_text_and_labels_cleanup_status() {
        let (_, input_receiver) = mpsc::sync_channel(1);
        let (_, stdout_receiver) = mpsc::sync_channel(1);
        let (_, stderr_receiver) = mpsc::sync_channel(1);
        let pending = Pending {
            input: Some(input_receiver),
            stdout: stdout_receiver,
            stderr: stderr_receiver,
            timed_out: true,
            output_exceeded: true,
        };
        #[cfg(unix)]
        let status = {
            use std::os::unix::process::ExitStatusExt as _;
            ExitStatus::from_raw(1 << 8)
        };
        #[cfg(windows)]
        let status = {
            use std::os::windows::process::ExitStatusExt as _;
            ExitStatus::from_raw(0xc000_0005)
        };
        let stdout = Captured { bytes: vec![b'o'; 1024 * 1024], exceeded: true };
        let stderr = Captured { bytes: vec![b'e'; 1024 * 1024], exceeded: false };
        let input_result = Some(Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "UNBOUNDED_PRIVATE_ERROR_TEXT".repeat(65536),
        )));
        let record = failure_record(&pending, &input_result, &status, &stdout, &stderr)
            .expect("bounded timeout record");
        assert!(record.len() < MAX_RECORD_BYTES);
        assert!(!record.contains("UNBOUNDED_PRIVATE_ERROR_TEXT"));
        assert!(record.contains("\"inputResult\":\"error\",\"inputErrorKind\":\"BrokenPipe\""));
        assert!(record.contains("\"timedOut\":true,\"outputExceeded\":true"));
        assert!(record.contains(
            "actual-post-cleanup-returned-status-not-evidence-of-natural-or-pre-timeout-exit"
        ));
        assert!(record.contains("\"stdout\":{\"bytes\":1048576,\"exceeded\":true,\"prefixBytes\":64,\"prefixTruncated\":true"));
        assert!(record.contains("\"stderr\":{\"bytes\":1048576,\"exceeded\":false,\"prefixBytes\":64,\"prefixTruncated\":true"));
        assert!(record.contains(&format!("\"prefixHex\":\"{}\"", "6f".repeat(PREFIX_BYTES))));
        assert!(record.contains(&format!("\"prefixHex\":\"{}\"", "65".repeat(PREFIX_BYTES))));
        for (input, expected) in [(None, "none"), (Some(Ok(())), "ok")] {
            let record = failure_record(&pending, &input, &status, &stdout, &stderr)
                .expect("bounded absent/successful input record");
            assert!(
                record.contains(&format!("\"inputResult\":\"{expected}\",\"inputErrorKind\":null"))
            );
        }
    }
}
