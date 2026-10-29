//! Retain this portable invocation before any child execution or oracle projection.
use super::{emit::Artifacts, process};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::{ffi::OsStrExt, fs::OpenOptionsExt, process::ExitStatusExt},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    path::Path,
    process::{Command, Output},
};
const PROGRESS_BYTES: usize = 65_536;

fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .expect("new portable witness file in exclusive case directory");
    file.write_all(bytes).expect("retain exact portable witness bytes");
}

pub(super) fn run(node: Command, target: &str, case: &str, artifacts: &Artifacts) -> Output {
    let cwd = node
        .get_current_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().expect("actual portable child cwd"));
    let argv = std::iter::once(node.get_program())
        .chain(node.get_args())
        .map(|argument| argument.as_bytes())
        .collect::<Vec<_>>();
    let request = serde_json::json!({"target":target,"case":case,
        "argvUnixBytes":argv,"cwdUnixBytes":cwd.as_os_str().as_bytes(),
        "deadlineSeconds":10,"deadlineOrigin":"existing process::run timer after spawn"});
    write_new(
        &artifacts.directory.join(format!("{target}-process-request.json")),
        &serde_json::to_vec_pretty(&request).expect("portable request JSON"),
    );
    let output = match catch_unwind(AssertUnwindSafe(|| process::run(node, 10))) {
        Ok(output) => output,
        Err(original) => {
            report_progress(target, case, &artifacts.directory);
            resume_unwind(original);
        }
    };
    write_new(&artifacts.directory.join(format!("{target}-child.stdout")), &output.stdout);
    write_new(&artifacts.directory.join(format!("{target}-child.stderr")), &output.stderr);
    let status = serde_json::json!({"raw":output.status.into_raw(),
        "code":output.status.code(),"signal":output.status.signal()});
    write_new(
        &artifacts.directory.join(format!("{target}-child-status.json")),
        &serde_json::to_vec_pretty(&status).expect("portable child status JSON"),
    );
    output
}

fn report_progress(target: &str, case: &str, directory: &Path) {
    let path = directory.join(format!("{target}-progress.jsonl"));
    let mut bytes = Vec::new();
    let read = File::open(&path)
        .and_then(|file| file.take((PROGRESS_BYTES + 1) as u64).read_to_end(&mut bytes));
    // Best effort only: failure reporting must not replace the original unwind payload.
    let mut log = std::io::stderr().lock();
    let _ = writeln!(
        log,
        "portable original failure witness: target={target}, case={case}, \
         directory={directory:?}, deadline_seconds=10, journal_read={read:?}, first_extra={}",
        bytes.len() > PROGRESS_BYTES
    );
    for (index, chunk) in bytes[..bytes.len().min(PROGRESS_BYTES)].chunks(1024).enumerate() {
        let _ = writeln!(log, "portable progress raw bytes chunk={index}: {chunk:?}");
    }
}
