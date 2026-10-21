//! Bounded, direct Node.js execution for sealed JavaScript and WebAssembly artifacts.
mod admission;
mod identity;
pub(crate) use identity::{open_runtime_identity, same_file_state, stable_invocation_path};
mod inline;
pub(crate) use admission::ExpectedRuntime;
mod ownership;
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use same_file::Handle;

#[cfg(unix)]
use std::process::Stdio;

#[cfg(unix)]
use nix::{
    errno::Errno,
    sys::signal::{self, Signal},
    unistd::Pid,
};
#[cfg(unix)]
use process_wrap::std::{ChildWrapper, CommandWrap, ProcessGroup};

use zryna_abi::{RawHostScalar, ScalarType};
use zryna_diagnostics::Diagnostic;

pub(crate) const NODE_VERSION: &str = "v22.22.1";
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_RESERVE: Duration = Duration::from_secs(5);
const MAX_VERSION_STDOUT: usize = 64;
const MAX_STDERR: usize = 16 * 1_024;
const MAX_INLINE_MODULE_BYTES: usize = 16 * 1_024;
const MAX_WEBASSEMBLY_INPUT_BYTES: usize = 32 * 1_024 * 1_024;

#[derive(Debug)]
pub(crate) struct NodeRuntimeCapability {
    executable: PathBuf,
    invocation_path: PathBuf,
    identity: Handle,
    state: std::fs::Metadata,
    expected: Option<ExpectedRuntime>,
}

impl NodeRuntimeCapability {
    pub(crate) fn discover(
        executable: &Path,
        working_directory: &Path,
    ) -> Result<Self, Diagnostic> {
        admission::discover(executable, working_directory, None)
    }

    pub(crate) fn discover_authenticated(
        executable: &Path,
        working_directory: &Path,
        expected: ExpectedRuntime,
    ) -> Result<Self, Diagnostic> {
        admission::discover(executable, working_directory, Some(expected))
    }

    pub(crate) fn executable(&self) -> Result<&Path, Diagnostic> {
        self.revalidate()?;
        Ok(&self.invocation_path)
    }

    pub(crate) fn revalidate(&self) -> Result<(), Diagnostic> {
        let (current, state) = open_runtime_identity(&self.executable)?;
        if let Some(expected) = &self.expected {
            admission::authenticate(&current, expected)?;
        }
        if current != self.identity || !same_file_state(&self.state, &state) {
            return Err(runtime_error(
                "ZRYNA-R3001",
                "Node.js runtime identity changed after validation",
                "stop concurrent replacement and retry with the pinned Node.js executable",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn run_module(
        &self,
        harness: &Path,
        working_directory: &Path,
    ) -> Result<[u8; 4], Diagnostic> {
        self.run_module_frame(harness, working_directory, 4)?
            .try_into()
            .map_err(|_| invalid_result_frame())
    }

    pub(crate) fn run_javascript_module(
        &self,
        harness: &Path,
        working_directory: &Path,
        result_type: ScalarType,
    ) -> Result<RawHostScalar, Diagnostic> {
        let expected_bytes = match result_type {
            ScalarType::Bool => 1,
            ScalarType::I32 => 4,
        };
        let frame = self.run_module_frame(harness, working_directory, expected_bytes)?;
        match result_type {
            ScalarType::Bool => match frame.as_slice() {
                [0] => Ok(RawHostScalar::JavaScriptBool(false)),
                [1] => Ok(RawHostScalar::JavaScriptBool(true)),
                _ => Err(invalid_result_frame()),
            },
            ScalarType::I32 => {
                let bytes: [u8; 4] = frame.try_into().map_err(|_| invalid_result_frame())?;
                Ok(RawHostScalar::JavaScriptNumber(f64::from(i32::from_le_bytes(bytes))))
            }
        }
    }

    fn run_module_frame(
        &self,
        harness: &Path,
        working_directory: &Path,
        expected_bytes: usize,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.revalidate()?;
        let node_harness = node_compatible_path(harness);
        let node_working_directory = node_compatible_path(working_directory);
        let output = run_bounded(
            &self.invocation_path,
            &[node_harness.into_os_string()],
            &node_working_directory,
            None,
            expected_bytes,
            MAX_STDERR,
        )?;
        self.revalidate()?;
        if !output.status.success()
            || !output.stderr.is_empty()
            || output.stdout.len() != expected_bytes
        {
            return Err(invalid_result_frame());
        }
        Ok(output.stdout)
    }
}

fn invalid_result_frame() -> Diagnostic {
    runtime_error(
        "ZRYNA-R3006",
        "target runtime returned an invalid scalar result frame",
        "report the smallest reproducible source and verified invocation",
    )
}

fn is_pinned_node_version(stdout: &[u8]) -> bool {
    stdout == format!("{NODE_VERSION}\n").as_bytes()
        || stdout == format!("{NODE_VERSION}\r\n").as_bytes()
}

#[cfg(windows)]
pub(crate) fn node_compatible_path(path: &Path) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    const VERBATIM_PREFIX: &[u16] = &[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    const UNC_PREFIX: &[u16] = &[b'U' as u16, b'N' as u16, b'C' as u16, b'\\' as u16];

    let encoded = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let Some(remainder) = encoded.strip_prefix(VERBATIM_PREFIX) else {
        return path.to_path_buf();
    };
    let normalized = if remainder.starts_with(UNC_PREFIX) {
        [u16::from(b'\\'), u16::from(b'\\')]
            .into_iter()
            .chain(remainder[UNC_PREFIX.len()..].iter().copied())
            .collect()
    } else {
        remainder.to_vec()
    };
    PathBuf::from(OsString::from_wide(&normalized))
}

#[cfg(not(windows))]
pub(crate) fn node_compatible_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct Captured {
    bytes: Vec<u8>,
    exceeded: bool,
}

struct Pending {
    input: Option<Receiver<io::Result<()>>>,
    stdout: Receiver<io::Result<Captured>>,
    stderr: Receiver<io::Result<Captured>>,
    timed_out: bool,
    output_exceeded: bool,
}

fn capture_stream(
    mut stream: impl Read,
    limit: usize,
    overflow: &SyncSender<()>,
) -> io::Result<Captured> {
    let mut bytes = Vec::with_capacity(limit.min(8 * 1_024));
    let mut buffer = [0_u8; 8 * 1_024];
    let mut exceeded = false;
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let available = limit.saturating_sub(bytes.len());
        let retained = available.min(count);
        bytes.extend_from_slice(&buffer[..retained]);
        if retained != count && !exceeded {
            exceeded = true;
            let _ = overflow.send(());
        }
    }
    Ok(Captured { bytes, exceeded })
}

fn run_bounded(
    program: &Path,
    arguments: &[OsString],
    working_directory: &Path,
    input: Option<&[u8]>,
    stdout_limit: usize,
    stderr_limit: usize,
) -> Result<BoundedOutput, Diagnostic> {
    run_bounded_with_timeout(
        program,
        arguments,
        working_directory,
        input,
        stdout_limit,
        stderr_limit,
        PROCESS_TIMEOUT,
    )
}

#[cfg(unix)]
fn run_bounded_with_timeout(
    program: &Path,
    arguments: &[OsString],
    working_directory: &Path,
    input: Option<&[u8]>,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
) -> Result<BoundedOutput, Diagnostic> {
    let mut native = std::process::Command::new(program);
    native
        .args(arguments)
        .current_dir(working_directory)
        .env_clear()
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut command = CommandWrap::from(native);
    command.wrap(ProcessGroup::leader());
    let mut child = crate::process_spawn::spawn(|| command.spawn())
        .map_err(|_| process_error("could not start"))?;
    let group_id = child.id().cast_signed();
    let operation = (|| {
        let input = match input {
            Some(bytes) => {
                let stdin = child.stdin().take().ok_or_else(|| process_error("lost stdin"))?;
                Some(spawn_input_writer(stdin, bytes.to_vec()))
            }
            None => None,
        };
        let stdout = child.stdout().take().ok_or_else(|| process_error("lost stdout"))?;
        let stderr = child.stderr().take().ok_or_else(|| process_error("lost stderr"))?;
        monitor_process(child.as_mut(), input, stdout, stderr, stdout_limit, stderr_limit, timeout)
    })();
    let cleanup_deadline = Instant::now() + CLEANUP_RESERVE;
    let cleanup = cleanup_unix(child.as_mut(), group_id, cleanup_deadline);
    let status = cleanup?;
    let pending = operation?;
    finish_capture(&pending, status, cleanup_deadline)
}

#[cfg(windows)]
fn run_bounded_with_timeout(
    program: &Path,
    arguments: &[OsString],
    working_directory: &Path,
    input: Option<&[u8]>,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
) -> Result<BoundedOutput, Diagnostic> {
    use windows_spawn::{Command, Job, SpawnOptions, Stdio as WindowsStdio};

    let job = Job::create().map_err(|_| process_error("could not create process job"))?;
    job.set_kill_on_close(true).map_err(|_| process_error("could not configure process job"))?;
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(working_directory)
        .env_clear()
        .stdin(if input.is_some() { WindowsStdio::piped() } else { WindowsStdio::null() })
        .stdout(WindowsStdio::piped())
        .stderr(WindowsStdio::piped());
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command
        .spawn_with(SpawnOptions::new().job(&job))
        .map_err(|_| process_error("could not start"))?;
    let operation = (|| {
        let input = match input {
            Some(bytes) => {
                let stdin = child.stdin.take().ok_or_else(|| process_error("lost stdin"))?;
                Some(spawn_input_writer(stdin, bytes.to_vec()))
            }
            None => None,
        };
        let stdout = child.stdout.take().ok_or_else(|| process_error("lost stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| process_error("lost stderr"))?;
        monitor_process(&mut child, input, stdout, stderr, stdout_limit, stderr_limit, timeout)
    })();
    let cleanup_deadline = Instant::now() + CLEANUP_RESERVE;
    let cleanup = cleanup_windows(&mut child, &job, cleanup_deadline);
    let status = cleanup?;
    let pending = operation?;
    finish_capture(&pending, status, cleanup_deadline)
}

#[cfg(windows)]
fn cleanup_windows(
    child: &mut windows_spawn::Child,
    job: &windows_spawn::Job,
    deadline: Instant,
) -> Result<ExitStatus, Diagnostic> {
    job.terminate(1).map_err(|_| cleanup_error())?;
    loop {
        if let Some(status) = child.try_wait().map_err(|_| cleanup_error())? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err(cleanup_error());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn monitor_process<Child: RuntimeChild + ?Sized>(
    child: &mut Child,
    input: Option<Receiver<io::Result<()>>>,
    stdout: impl Read + Send + 'static,
    stderr: impl Read + Send + 'static,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
) -> Result<Pending, Diagnostic> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| process_error("could not establish execution deadline"))?;
    let (stdout_sender, stdout_receiver) = mpsc::sync_channel(1);
    let (stderr_sender, stderr_receiver) = mpsc::sync_channel(1);
    let (overflow_sender, overflow_receiver) = mpsc::sync_channel(2);
    let stdout_overflow = overflow_sender.clone();
    thread::spawn(move || {
        let _ = stdout_sender.send(capture_stream(stdout, stdout_limit, &stdout_overflow));
    });
    thread::spawn(move || {
        let _ = stderr_sender.send(capture_stream(stderr, stderr_limit, &overflow_sender));
    });
    let mut timed_out = false;
    let mut output_exceeded = false;
    loop {
        match overflow_receiver.try_recv() {
            Ok(()) => {
                output_exceeded = true;
                break;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        match child.try_wait().map_err(|_| process_error("could not observe"))? {
            Some(_) => break,
            None if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            None => {
                timed_out = true;
                break;
            }
        }
    }
    Ok(Pending {
        input,
        stdout: stdout_receiver,
        stderr: stderr_receiver,
        timed_out,
        output_exceeded,
    })
}

fn spawn_input_writer(
    mut stdin: impl Write + Send + 'static,
    input: Vec<u8>,
) -> Receiver<io::Result<()>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = stdin.write_all(&input).and_then(|()| stdin.flush());
        drop(stdin);
        let _ = sender.send(result);
    });
    receiver
}

trait RuntimeChild {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;
}

#[cfg(unix)]
impl RuntimeChild for dyn ChildWrapper + '_ {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        ChildWrapper::try_wait(self)
    }
}

#[cfg(windows)]
impl RuntimeChild for windows_spawn::Child {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.try_wait()
    }
}

#[cfg(unix)]
fn cleanup_unix(
    child: &mut dyn ChildWrapper,
    raw_group_id: i32,
    deadline: Instant,
) -> Result<ExitStatus, Diagnostic> {
    let group = Pid::from_raw(raw_group_id);
    let mut status = None;
    loop {
        let _ = child.start_kill();
        if status.is_none()
            && let Ok(Some(value)) = child.try_wait()
        {
            status = Some(value);
        }
        let group_gone = match signal::killpg(group, None) {
            Err(Errno::ESRCH) => true,
            Ok(()) | Err(Errno::EPERM) => {
                let _ = signal::killpg(group, Signal::SIGKILL);
                false
            }
            Err(_) => return Err(cleanup_error()),
        };
        if group_gone && let Some(status) = status {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err(cleanup_error());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn finish_capture(
    pending: &Pending,
    status: ExitStatus,
    deadline: Instant,
) -> Result<BoundedOutput, Diagnostic> {
    let input_result = if let Some(input) = &pending.input {
        let remaining =
            deadline.checked_duration_since(Instant::now()).ok_or_else(cleanup_error)?;
        match input.recv_timeout(remaining) {
            Ok(result) => Some(result),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                return Err(cleanup_error());
            }
        }
    } else {
        None
    };
    let stdout = receive_capture(&pending.stdout, deadline)?;
    let stderr = receive_capture(&pending.stderr, deadline)?;
    if pending.timed_out {
        return Err(runtime_error(
            "ZRYNA-R3003",
            "target runtime exceeded its hard deadline",
            "reduce the invocation and report a reproducible timeout",
        ));
    }
    if pending.output_exceeded || stdout.exceeded || stderr.exceeded {
        return Err(runtime_error(
            "ZRYNA-R3004",
            "target runtime exceeded its output budget",
            "report the smallest reproducible source and invocation",
        ));
    }
    if matches!(input_result, Some(Err(_))) {
        return Err(process_error("could not provide bounded input"));
    }
    Ok(BoundedOutput { status, stdout: stdout.bytes, stderr: stderr.bytes })
}

fn receive_capture(
    receiver: &Receiver<io::Result<Captured>>,
    deadline: Instant,
) -> Result<Captured, Diagnostic> {
    let remaining = deadline.checked_duration_since(Instant::now()).ok_or_else(cleanup_error)?;
    match receiver.recv_timeout(remaining) {
        Ok(Ok(captured)) => Ok(captured),
        Ok(Err(_)) => Err(process_error("could not capture output")),
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => Err(cleanup_error()),
    }
}

fn process_error(action: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-R3005",
        None,
        format!("target runtime process {action}"),
        "use the documented direct Node.js runtime and report repeated failures",
    )
}

fn cleanup_error() -> Diagnostic {
    runtime_error(
        "ZRYNA-R3007",
        "target runtime process-tree cleanup could not be confirmed",
        "stop the exact runtime process tree before retrying",
    )
}

fn runtime_error(code: &'static str, message: &'static str, guidance: &'static str) -> Diagnostic {
    Diagnostic::error(code, None, message, guidance)
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };

    #[cfg(windows)]
    use super::node_compatible_path;
    use super::{
        MAX_INLINE_MODULE_BYTES, MAX_WEBASSEMBLY_INPUT_BYTES, NodeRuntimeCapability,
        is_pinned_node_version,
    };
    use zryna_abi::{RawHostScalar, ScalarType};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[cfg(windows)]
    #[test]
    fn node_paths_remove_only_windows_verbatim_prefixes() {
        assert_eq!(
            node_compatible_path(Path::new(r"\\?\C:\workspace\adapter\src\worker.mjs")),
            PathBuf::from(r"C:\workspace\adapter\src\worker.mjs")
        );
        assert_eq!(
            node_compatible_path(Path::new(r"\\?\UNC\server\share\adapter\src\worker.mjs")),
            PathBuf::from(r"\\server\share\adapter\src\worker.mjs")
        );
        assert_eq!(
            node_compatible_path(Path::new(r"C:\workspace\adapter")),
            PathBuf::from(r"C:\workspace\adapter")
        );
    }

    struct RuntimeRoot {
        path: PathBuf,
    }

    impl RuntimeRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "zryna runtime {label} & literal $HOME ; %PATH% {}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("runtime fixture root must be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn harness(&self, label: &str, source: &str) -> PathBuf {
            let path = self.path.join(format!("{label} & literal $HOME ; %PATH%.mjs"));
            fs::write(&path, source).expect("runtime harness must be written");
            path
        }
    }

    impl Drop for RuntimeRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn node_executable() -> PathBuf {
        let executable = if cfg!(windows) { "node.exe" } else { "node" };
        let candidate = ["ZRYNA_TEST_NODE", "NODE"]
            .into_iter()
            .filter_map(env::var_os)
            .map(PathBuf::from)
            .chain(
                env::var_os("PATH")
                    .into_iter()
                    .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
                    .map(move |directory| directory.join(executable)),
            )
            .find(|path| path.is_file())
            .expect("Node.js must be installed for runtime tests")
            .canonicalize()
            .expect("Node.js executable must canonicalize");
        let version = crate::process_spawn::output(Command::new(&candidate).arg("--version"))
            .expect("Node.js version probe must start");
        assert!(version.status.success());
        assert!(is_pinned_node_version(&version.stdout));
        assert!(version.stderr.is_empty());
        candidate
    }

    fn capability(root: &RuntimeRoot) -> NodeRuntimeCapability {
        NodeRuntimeCapability::discover(&node_executable(), root.path())
            .expect("documented Node.js runtime must validate")
    }

    fn assert_code(error: &zryna_diagnostics::Diagnostic, expected: &str) {
        assert_eq!(error.code(), expected);
    }

    #[test]
    fn pinned_node_version_accepts_native_line_endings_only() {
        assert!(is_pinned_node_version(b"v22.22.1\n"));
        assert!(is_pinned_node_version(b"v22.22.1\r\n"));
        assert!(!is_pinned_node_version(b"v22.22.1"));
        assert!(!is_pinned_node_version(b"v22.22.1\nextra\n"));
        assert!(!is_pinned_node_version(b"v22.22.0\r\n"));
    }

    #[test]
    fn webassembly_inline_inputs_fail_at_the_first_extra_byte() {
        let root = RuntimeRoot::new("webassembly-input-bounds");
        let runtime = capability(&root);
        let oversized_script = vec![b'a'; MAX_INLINE_MODULE_BYTES + 1];
        assert_code(
            &runtime
                .run_webassembly_module(&oversized_script, &[], root.path())
                .expect_err("oversized inline script must fail"),
            "ZRYNA-R3004",
        );

        let oversized_module = vec![0_u8; MAX_WEBASSEMBLY_INPUT_BYTES + 1];
        assert_code(
            &runtime
                .run_webassembly_module(b"", &oversized_module, root.path())
                .expect_err("oversized module input must fail"),
            "ZRYNA-R3004",
        );
        assert_code(
            &runtime
                .run_webassembly_module(&[0xff], &[], root.path())
                .expect_err("non-UTF-8 inline script must fail"),
            "ZRYNA-R3006",
        );
    }

    #[test]
    fn blocked_webassembly_input_preserves_timeout_and_output_diagnostics() {
        let root = RuntimeRoot::new("blocked-webassembly-input");
        let runtime = capability(&root);
        let input = vec![0_u8; 1024 * 1024];
        let timeout = runtime
            .run_webassembly_module(b"setInterval(() => {}, 60_000);", &input, root.path())
            .expect_err("blocked input must preserve the timeout diagnostic");
        assert_code(&timeout, "ZRYNA-R3003");

        let overflow = runtime
            .run_webassembly_module(
                b"for (;;) process.stdout.write(Buffer.alloc(8192, 65));",
                &input,
                root.path(),
            )
            .expect_err("blocked input must preserve the output diagnostic");
        assert_code(&overflow, "ZRYNA-R3004");
    }

    #[test]
    fn invalid_version_frame_stderr_and_exit_fail_closed() {
        let root = RuntimeRoot::new("invalid-results");
        let invalid_version = NodeRuntimeCapability::discover(
            &env::current_exe().expect("test executable path"),
            root.path(),
        )
        .expect_err("a non-Node executable must fail identity validation");
        assert_code(&invalid_version, "ZRYNA-R3002");

        let runtime = capability(&root);
        let short = root.harness("short-frame", "process.stdout.write(Buffer.from([1, 2, 3]));\n");
        assert_code(
            &runtime.run_module(&short, root.path()).expect_err("short frame must fail"),
            "ZRYNA-R3006",
        );

        let stderr = root.harness(
            "stderr-frame",
            "process.stdout.write(Buffer.from([1, 0, 0, 0])); process.stderr.write('unexpected');\n",
        );
        assert_code(
            &runtime.run_module(&stderr, root.path()).expect_err("stderr must fail framing"),
            "ZRYNA-R3006",
        );

        let abnormal = root.harness(
            "abnormal-exit",
            "process.stdout.write(Buffer.from([1, 0, 0, 0])); process.exit(23);\n",
        );
        assert_code(
            &runtime.run_module(&abnormal, root.path()).expect_err("abnormal exit must fail"),
            "ZRYNA-R3006",
        );
    }

    #[test]
    fn typed_javascript_frames_use_public_exports_and_reject_noncanonical_bool() {
        let root = RuntimeRoot::new("typed-javascript");
        let runtime = capability(&root);
        let module = root.path().join("public-module.mjs");
        fs::write(
            &module,
            concat!(
                "export function Number(value) {\n",
                "  if (typeof value !== 'boolean') throw new TypeError('bool');\n",
                "  return value;\n",
                "}\n",
                "export function Object() { return -2147483648; }\n",
            ),
        )
        .expect("public module must be written");
        let module_name = serde_json::to_string(&format!(
            "./{}",
            module.file_name().expect("module file name").to_string_lossy()
        ))
        .expect("module name JSON");
        let bool_harness = root.harness(
            "bool-public-import",
            &format!(
                "import {{ Number as invoke }} from {module_name};\nconst value = invoke(true);\nif (typeof value !== 'boolean') process.exit(70);\nprocess.stdout.write(Uint8Array.of(value ? 1 : 0));\n"
            ),
        );
        assert_eq!(
            runtime
                .run_javascript_module(&bool_harness, root.path(), ScalarType::Bool)
                .expect("primitive bool frame"),
            RawHostScalar::JavaScriptBool(true)
        );

        let i32_harness = root.harness(
            "i32-public-import",
            &format!(
                "import {{ Object as invoke }} from {module_name};\nconst frame = Buffer.allocUnsafe(4);\nframe.writeInt32LE(invoke(), 0);\nprocess.stdout.write(frame);\n"
            ),
        );
        assert_eq!(
            runtime
                .run_javascript_module(&i32_harness, root.path(), ScalarType::I32)
                .expect("canonical i32 frame"),
            RawHostScalar::JavaScriptNumber(f64::from(i32::MIN))
        );

        let invalid_bool =
            root.harness("invalid-bool-frame", "process.stdout.write(Uint8Array.of(2));\n");
        assert_code(
            &runtime
                .run_javascript_module(&invalid_bool, root.path(), ScalarType::Bool)
                .expect_err("noncanonical Boolean byte must fail"),
            "ZRYNA-R3006",
        );
    }

    #[test]
    fn live_output_overflow_fails_with_the_bounded_diagnostic() {
        let root = RuntimeRoot::new("overflow");
        let runtime = capability(&root);
        let harness =
            root.harness("overflow", "for (;;) process.stdout.write(Buffer.alloc(8192, 65));\n");

        let started = Instant::now();
        let error = runtime
            .run_module(&harness, root.path())
            .expect_err("unbounded target output must fail");

        assert_code(&error, "ZRYNA-R3004");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn timeout_terminates_the_target_within_the_total_deadline() {
        let root = RuntimeRoot::new("timeout");
        let runtime = capability(&root);
        let harness = root.harness("timeout", "setInterval(() => {}, 60_000);\n");

        let started = Instant::now();
        let error = runtime
            .run_module(&harness, root.path())
            .expect_err("nonterminating target must time out");

        assert_code(&error, "ZRYNA-R3003");
        assert!(started.elapsed() >= Duration::from_secs(4));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn environment_is_cleared_and_metacharacter_paths_are_literal() {
        let root = RuntimeRoot::new("environment");
        let runtime = capability(&root);
        let harness = root.harness(
            "environment",
            concat!(
                "const forbidden = ['PATH', 'HOME', 'USERPROFILE', 'CARGO_HOME', 'RUSTUP_HOME'];\n",
                "if (forbidden.some((name) => process.env[name] !== undefined)) {\n",
                "  process.stderr.write('ambient environment leaked'); process.exit(31);\n",
                "}\n",
                "process.stdout.write(Buffer.from([42, 0, 0, 0]));\n",
            ),
        );

        assert_eq!(
            runtime
                .run_module(&harness, root.path())
                .expect("literal metacharacter path and cleared environment must execute"),
            [42, 0, 0, 0]
        );
    }

    #[test]
    fn runtime_identity_replacement_is_detected_or_prevented() {
        let root = RuntimeRoot::new("identity-replacement");
        let extension = if cfg!(windows) { ".exe" } else { "" };
        let runtime_path = root.path().join(format!("private-node{extension}"));
        let runtime_source = root.path().join("private-node.rs");
        let compiled_runtime =
            env::temp_dir().join(format!("zryna-private-node-{}{}", std::process::id(), extension));
        fs::write(&runtime_source, "fn main() { print!(\"v22.22.1\\n\"); }\n")
            .expect("private runtime source must be written");
        let compiler = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let output = crate::process_spawn::output(
            Command::new(compiler)
                .args(["--edition=2024", "--crate-name", "zryna_private_node", "-o"])
                .arg(&compiled_runtime)
                .arg(&runtime_source),
        )
        .expect("private runtime compiler must start");
        assert!(
            output.status.success(),
            "private runtime compilation must succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        fs::rename(&compiled_runtime, &runtime_path)
            .expect("compiled private runtime must move into the literal path");
        let runtime = NodeRuntimeCapability::discover(&runtime_path, root.path())
            .expect("private Node.js copy must validate");
        let retained = root.path().join(format!("retained-node{extension}"));

        match fs::rename(&runtime_path, &retained) {
            Ok(()) => {
                fs::copy(env::current_exe().expect("replacement executable path"), &runtime_path)
                    .expect("replacement executable must be installed");
                assert_code(
                    &runtime.revalidate().expect_err("replacement must invalidate capability"),
                    "ZRYNA-R3001",
                );
            }
            Err(error) if cfg!(windows) => {
                assert!(
                    error.kind() == std::io::ErrorKind::PermissionDenied
                        || matches!(error.raw_os_error(), Some(5 | 32)),
                    "unexpected replacement failure: {error}"
                );
                runtime.revalidate().expect("denied replacement must preserve identity");
            }
            Err(error) => panic!("runtime replacement setup failed unexpectedly: {error}"),
        }
    }
}
