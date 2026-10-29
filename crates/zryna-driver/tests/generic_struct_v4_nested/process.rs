//! Bounded child streams and process groups, created through the shared spawn gate.
use process_wrap::std::{ChildWrapper, CommandWrap, ProcessGroup};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
const STREAM_BYTES: usize = 65_536;
struct Running {
    child: Box<dyn ChildWrapper>,
    done: bool,
}
impl Drop for Running {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
        }
    }
}
fn reader(
    stream: impl Read + Send + 'static,
    overflow: Arc<AtomicBool>,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut input = stream.take((STREAM_BYTES + 1) as u64);
        input.read_to_end(&mut bytes).expect("bounded child stream");
        if bytes.len() > STREAM_BYTES {
            overflow.store(true, Ordering::Release);
        }
        bytes
    })
}
pub(super) fn run(mut native: Command, seconds: u64) -> Output {
    native.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).env("LC_ALL", "C");
    let program = native.get_program().to_owned();
    let argv = std::iter::once(native.get_program())
        .chain(native.get_args())
        .map(|argument| argument.as_encoded_bytes().to_vec())
        .collect::<Vec<_>>();
    let cwd =
        native.get_current_dir().map(Path::to_path_buf).or_else(|| std::env::current_dir().ok());
    let mut command = CommandWrap::from(native);
    command.wrap(ProcessGroup::leader());
    let child = zryna_process::spawn(|| command.spawn()).expect("shared-gate child creation");
    let mut running = Running { child, done: false };
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout = reader(running.child.stdout().take().expect("stdout pipe"), Arc::clone(&overflow));
    let stderr = reader(running.child.stderr().take().expect("stderr pipe"), Arc::clone(&overflow));
    let started = Instant::now();
    let deadline = started + Duration::from_secs(seconds);
    let status = loop {
        let stream_overflow = overflow.load(Ordering::Acquire);
        if stream_overflow || Instant::now() >= deadline {
            let elapsed = started.elapsed();
            running.child.kill().expect("terminate and reap complete test process group");
            let stdout = stdout.join().ok();
            let stderr = stderr.join().ok();
            let stdout_bytes = stdout.as_ref().map(Vec::len);
            let stderr_bytes = stderr.as_ref().map(Vec::len);
            failure_evidence(&argv, cwd.as_deref(), stdout.as_deref(), stderr.as_deref());
            panic!(
                "child exceeded its existing stream/deadline budget: program={}, \
                 seconds={seconds}, elapsed={elapsed:?}, overflow={stream_overflow}, \
                 stdout_bytes={stdout_bytes:?}, stderr_bytes={stderr_bytes:?}",
                program.display()
            );
        }
        if let Some(status) = running.child.try_wait().expect("bounded child status") {
            break status;
        }
        thread::sleep(Duration::from_millis(10));
    };
    running.done = true;
    let stdout = stdout.join().expect("stdout reader");
    let stderr = stderr.join().expect("stderr reader");
    assert!(stdout.len() <= STREAM_BYTES && stderr.len() <= STREAM_BYTES, "child output ceiling");
    Output { status, stdout, stderr }
}

fn failure_evidence(
    argv: &[Vec<u8>],
    cwd: Option<&Path>,
    stdout: Option<&[u8]>,
    stderr: Option<&[u8]>,
) {
    let cwd = cwd.map(|path| path.as_os_str().as_encoded_bytes());
    let mut log = std::io::stderr().lock();
    let _ = writeln!(log, "child actual argv_encoded_bytes={argv:?}, cwd_encoded_bytes={cwd:?}");
    for (name, bytes) in [("stdout", stdout), ("stderr", stderr)] {
        if let Some(bytes) = bytes {
            for (index, chunk) in bytes.chunks(1024).enumerate() {
                let _ = writeln!(log, "child original {name} raw bytes chunk={index}: {chunk:?}");
            }
        } else {
            let _ = writeln!(log, "child original {name} reader panicked; no joined bytes");
        }
    }
}
