//! Windows Job containment mirrors the existing host boundary; execution20s, cleanup5s, streams64KiB.
use std::{
    io::Read,
    path::Path,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use windows_spawn::{Child, Command, Job, SpawnOptions, Stdio};
const STREAM_BYTES: usize = 65_536;
const EXECUTION: Duration = Duration::from_secs(20);
const CLEANUP: Duration = Duration::from_secs(5);
pub(super) struct Observed {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub failure: Option<String>,
}
struct Running {
    child: Child,
    job: Job,
}
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.job.terminate(1);
    }
}
struct Capture {
    bytes: Vec<u8>,
    failure: Option<String>,
}
fn reader(
    stream: impl Read + Send + 'static,
    overflow: Arc<AtomicBool>,
) -> mpsc::Receiver<Capture> {
    let (send, receive) = mpsc::channel();
    let _ = thread::spawn(move || {
        let mut bytes = Vec::new();
        let failure = if bytes.try_reserve_exact(STREAM_BYTES + 1).is_err() {
            Some("bounded stream reservation failed".into())
        } else {
            stream
                .take((STREAM_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .err()
                .map(|error| error.to_string())
        };
        if bytes.len() > STREAM_BYTES {
            overflow.store(true, Ordering::Release);
        }
        let _ = send.send(Capture { bytes, failure });
    });
    receive
}
fn start(node: &Path, args: &[std::ffi::OsString], directory: &Path) -> Result<Running, String> {
    let job = Job::create().map_err(|error| format!("create job: {error}"))?;
    job.set_kill_on_close(true).map_err(|error| format!("configure job: {error}"))?;
    let mut command = Command::new(node);
    command
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("LC_ALL", "C");
    let child = zryna_process::spawn(|| command.spawn_with(SpawnOptions::new().job(&job)))
        .map_err(|error| format!("spawn: {error}"))?;
    Ok(Running { child, job })
}
fn capture(
    receive: &mpsc::Receiver<Capture>,
    deadline: Instant,
    failure: &mut Option<String>,
) -> Vec<u8> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    match receive.recv_timeout(remaining) {
        Ok(captured) => {
            if failure.is_none() {
                *failure = captured.failure;
            }
            captured.bytes
        }
        Err(error) => {
            failure.get_or_insert_with(|| format!("bounded stream cleanup: {error}"));
            Vec::new()
        }
    }
}
pub(super) fn run(node: &Path, args: &[std::ffi::OsString], directory: &Path) -> Observed {
    let mut running = match start(node, args, directory) {
        Ok(child) => child,
        Err(error) => {
            return Observed {
                status: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                failure: Some(error),
            };
        }
    };
    let mut result =
        Observed { status: None, stdout: Vec::new(), stderr: Vec::new(), failure: None };
    let overflow = Arc::new(AtomicBool::new(false));
    let (stdout, stderr) = if let (Some(stdout), Some(stderr)) =
        (running.child.stdout.take(), running.child.stderr.take())
    {
        (reader(stdout, Arc::clone(&overflow)), reader(stderr, Arc::clone(&overflow)))
    } else {
        result.failure = Some("configured child stream disappeared".into());
        return result;
    };
    let deadline = Instant::now() + EXECUTION;
    loop {
        if overflow.load(Ordering::Acquire) || Instant::now() >= deadline {
            result.failure = Some("existing child stream or20s execution bound exceeded".into());
            break;
        }
        match running.child.try_wait() {
            Ok(Some(status)) => {
                result.status = Some(status);
                break;
            }
            Ok(None) => {}
            Err(error) => {
                result.failure = Some(format!("child status: {error}"));
                break;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    let cleanup = Instant::now() + CLEANUP;
    if let Err(error) = running.job.terminate(1) {
        result.failure.get_or_insert_with(|| format!("job cleanup: {error}"));
    }
    while result.status.is_none() && Instant::now() < cleanup {
        match running.child.try_wait() {
            Ok(Some(status)) => result.status = Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                result.failure.get_or_insert_with(|| format!("reap: {error}"));
                break;
            }
        }
    }
    if result.status.is_none() {
        result.failure.get_or_insert_with(|| "child cleanup exceeded existing5s reserve".into());
    }
    result.stdout = capture(&stdout, cleanup, &mut result.failure);
    result.stderr = capture(&stderr, cleanup, &mut result.failure);
    if result.stdout.len() > STREAM_BYTES || result.stderr.len() > STREAM_BYTES {
        result.failure.get_or_insert_with(|| "child exceeded existing stream bound".into());
    }
    result
}
