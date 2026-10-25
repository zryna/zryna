//! This finite Node fixture's host deadline/stream owner is separate from source resource quotas.
use std::{
    io::Read,
    process::{Child, Command, Output, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
const STREAM_BYTES: usize = 65_536;
const DEADLINE_SECONDS: u64 = 60;
// Schedule this fixture's heavy children without changing any child deadline or source quota.
static PROCESS_OWNER: Mutex<()> = Mutex::new(());
struct Running {
    child: Child,
    done: bool,
}
impl Drop for Running {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
pub(in crate::generic_execution_v1) struct Outcome {
    pub(in crate::generic_execution_v1) output: Output,
    pub(in crate::generic_execution_v1) containment: Option<&'static str>,
}
fn reader(
    stream: impl Read + Send + 'static,
    overflow: Arc<AtomicBool>,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        stream
            .take(u64::try_from(STREAM_BYTES + 1).expect("fixed stream bound"))
            .read_to_end(&mut bytes)
            .expect("bounded fixture output");
        if bytes.len() > STREAM_BYTES {
            overflow.store(true, Ordering::Release);
        }
        bytes
    })
}
pub(in crate::generic_execution_v1) fn run(mut command: Command) -> Outcome {
    let _owner = PROCESS_OWNER.lock().expect("finite accounting fixture process owner");
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = command.spawn().expect("finite ordinary Node fixture process");
    let mut running = Running { child, done: false };
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout = reader(running.child.stdout.take().expect("stdout pipe"), Arc::clone(&overflow));
    let stderr = reader(running.child.stderr.take().expect("stderr pipe"), Arc::clone(&overflow));
    let deadline = Instant::now() + Duration::from_secs(DEADLINE_SECONDS);
    let mut containment = None;
    let status = loop {
        if overflow.load(Ordering::Acquire) || Instant::now() >= deadline {
            containment =
                Some(if overflow.load(Ordering::Acquire) { "stream" } else { "deadline" });
            let _ = running.child.kill();
            break running
                .child
                .wait()
                .expect("reap finite Node fixture after containment failure");
        }
        if let Some(status) = running.child.try_wait().expect("finite Node child status") {
            break status;
        }
        thread::sleep(Duration::from_millis(10));
    };
    running.done = true;
    let stdout = stdout.join().expect("bounded stdout reader");
    let stderr = stderr.join().expect("bounded stderr reader");
    if stdout.len() > STREAM_BYTES || stderr.len() > STREAM_BYTES {
        containment = Some("stream");
    }
    Outcome { output: Output { status, stdout, stderr }, containment }
}
