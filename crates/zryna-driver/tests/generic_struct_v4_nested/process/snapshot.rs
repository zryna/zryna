//! Best-effort original Linux child evidence after the existing failure decision.
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};

const FILE_BYTES: usize = 4096;
const TOTAL_BYTES: usize = 131_072;
const REPORT_BYTES: usize = 1_048_576;
const TASKS: usize = 64;

pub(super) struct Snapshot {
    pid: u32,
    deadline_seconds: u64,
    decision_elapsed: Duration,
    stream_overflow: bool,
    started: Instant,
    deadline: Instant,
    capture_elapsed: Duration,
    remaining: usize,
    time_exhausted: bool,
    tasks_seen: usize,
    tasks_first_extra: bool,
    task_error: Option<std::io::ErrorKind>,
    entries: Vec<serde_json::Value>,
}

impl Snapshot {
    fn read(&mut self, path: &Path) -> bool {
        self.time_exhausted |= Instant::now() >= self.deadline;
        if self.time_exhausted || self.remaining == 0 {
            return false;
        }
        let mut bytes = Vec::new();
        let result = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)
            .and_then(|file| file.take(4097).read_to_end(&mut bytes));
        let first_extra = bytes.len() > FILE_BYTES || bytes.len() > self.remaining;
        bytes.truncate(FILE_BYTES.min(self.remaining));
        self.remaining -= bytes.len();
        self.entries.push(serde_json::json!({"path":path.display().to_string(),"bytes":bytes,
            "error":result.err().map(|error| format!("{:?}", error.kind())),
            "first_extra":first_extra}));
        true
    }

    fn retain(&self) {
        let reporting = Instant::now();
        let document = serde_json::json!({"format":"zryna.original-child-proc.v1",
            "observer_pid":std::process::id(),"original_child_pid":self.pid,
            "capture_before_budget_kill":true,"failure_already_decided":true,"non_atomic":true,
            "original_deadline_seconds":self.deadline_seconds,
            "original_decision_elapsed":format!("{:?}",self.decision_elapsed),
            "original_stream_overflow":self.stream_overflow,
            "capture_elapsed":format!("{:?}",self.capture_elapsed),
            "cleanup_since_capture":format!("{:?}",self.started.elapsed().saturating_sub(self.capture_elapsed)),
            "read_budget_ms":25,"raw_file_limit":FILE_BYTES,"raw_total_limit":TOTAL_BYTES,
            "raw_bytes":TOTAL_BYTES-self.remaining,"time_exhausted":self.time_exhausted,
            "bytes_exhausted":self.remaining==0,"task_limit":TASKS,
            "tasks_seen":self.tasks_seen,"tasks_first_extra":self.tasks_first_extra,
            "task_error":self.task_error.map(|error|format!("{error:?}")),"entries":self.entries});
        let mut log = std::io::stderr().lock();
        let bytes = match serde_json::to_vec(&document) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(log, "original child proc serialization error: {error}");
                return;
            }
        };
        let path = std::env::temp_dir().join(format!(
            "m416-process-deadline-{}-{}.json",
            std::process::id(),
            self.pid
        ));
        let retained = if bytes.len() <= REPORT_BYTES {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&path)
                .and_then(|mut file| file.write_all(&bytes))
        } else {
            Err(std::io::Error::other("original proc report byte limit"))
        };
        let _ = writeln!(
            log,
            "original child proc snapshot: child_pid={}, path={}, bytes={}, capture_elapsed={:?}, \
             reporting_after_failure_elapsed={:?}, write_error={:?}; deadline decision unchanged",
            self.pid,
            path.display(),
            bytes.len(),
            self.capture_elapsed,
            reporting.elapsed(),
            retained.as_ref().err().map(std::io::Error::kind)
        );
        if retained.is_err() {
            for (index, chunk) in bytes[..bytes.len().min(REPORT_BYTES)].chunks(1024).enumerate() {
                let _ = writeln!(log, "original proc fallback raw bytes chunk={index}: {chunk:?}");
            }
        }
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        // Diagnostic serialization/IO must never replace the original timeout unwind.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.retain()));
    }
}

pub(super) fn capture(pid: u32, seconds: u64, elapsed: Duration, overflow: bool) -> Snapshot {
    let started = Instant::now();
    let mut snapshot = Snapshot {
        pid,
        deadline_seconds: seconds,
        decision_elapsed: elapsed,
        stream_overflow: overflow,
        started,
        deadline: started + Duration::from_millis(25),
        capture_elapsed: Duration::ZERO,
        remaining: TOTAL_BYTES,
        time_exhausted: false,
        tasks_seen: 0,
        tasks_first_extra: false,
        task_error: None,
        entries: Vec::new(),
    };
    let child = std::path::PathBuf::from(format!("/proc/{pid}"));
    for name in ["stat", "status", "wchan", "syscall"] {
        if !snapshot.read(&child.join(name)) {
            break;
        }
    }
    match std::fs::read_dir(child.join("task")) {
        Ok(tasks) => {
            for (index, entry) in tasks.take(TASKS + 1).enumerate() {
                snapshot.tasks_seen = index + 1;
                if index == TASKS {
                    snapshot.tasks_first_extra = true;
                    break;
                }
                match entry {
                    Ok(entry) => {
                        if let Ok(tid) = entry.file_name().to_string_lossy().parse::<u32>() {
                            let task = child.join("task").join(tid.to_string());
                            if !snapshot.read(&task.join("stat"))
                                || !snapshot.read(&task.join("wchan"))
                            {
                                break;
                            }
                        } else {
                            snapshot.task_error = Some(std::io::ErrorKind::InvalidData);
                        }
                    }
                    Err(error) => {
                        snapshot.task_error = Some(error.kind());
                    }
                }
            }
        }
        Err(error) => {
            snapshot.task_error = Some(error.kind());
        }
    }
    for path in [
        "/proc/sys/kernel/core_pattern",
        "/proc/pressure/cpu",
        "/proc/pressure/io",
        "/proc/pressure/memory",
        "/proc/loadavg",
    ] {
        if !snapshot.read(Path::new(path)) {
            break;
        }
    }
    let _ = snapshot.read(&child.join("stat"));
    snapshot.time_exhausted |= Instant::now() >= snapshot.deadline;
    snapshot.capture_elapsed = started.elapsed();
    snapshot
}
