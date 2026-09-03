use std::{
    fmt, io,
    io::Write as _,
    sync::atomic::{AtomicU64, Ordering},
};

use super::ProcessPhase;

static NEXT_INVOCATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub(super) enum Operation {
    Deadline,
    Spawn,
    StdoutPipe,
    StderrPipe,
    TryWait,
    ReadStdout,
    ReadStderr,
}

#[derive(Clone, Copy)]
pub(super) struct Context {
    invocation: u64,
    phase: ProcessPhase,
}

impl Context {
    pub(super) fn new(phase: ProcessPhase) -> Self {
        Self { invocation: NEXT_INVOCATION.fetch_add(1, Ordering::Relaxed), phase }
    }

    fn failure(self, operation: Operation, error: Option<&io::Error>) -> Failure {
        Failure {
            context: self,
            operation,
            kind: error.map(io::Error::kind),
            errno: error.and_then(io::Error::raw_os_error),
        }
    }

    pub(super) fn record(self, operation: Operation, error: Option<&io::Error>) {
        // A fixed record ties capture-thread failures to this invocation without retaining input.
        let _ = writeln!(io::stderr().lock(), "{}", self.failure(operation, error));
    }
}

struct Failure {
    context: Context,
    operation: Operation,
    kind: Option<io::ErrorKind>,
    errno: Option<i32>,
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "native-process-test invocation={} phase={:?} operation={:?} kind={:?} errno={:?}",
            self.context.invocation, self.context.phase, self.operation, self.kind, self.errno
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{capture_stream, process_io_error, run_bounded_process};
    use std::{io::Read, path::Path, sync::mpsc, time::Duration};

    #[test]
    fn context_is_sanitized_bounded_and_distinguishes_operations() {
        let context = Context { invocation: u64::MAX, phase: ProcessPhase::Run };
        let error = io::Error::other("/private/path\nTOKEN=secret executable argv");
        for operation in [
            Operation::Deadline,
            Operation::Spawn,
            Operation::StdoutPipe,
            Operation::StderrPipe,
            Operation::TryWait,
            Operation::ReadStdout,
            Operation::ReadStderr,
        ] {
            let text = context.failure(operation, Some(&error)).to_string();
            assert!(text.len() < 192);
            assert!(!text.contains(['\n', '/']));
            assert!(!text.contains("secret"));
            assert!(text.contains(&format!("operation={operation:?}")));
            assert!(text.ends_with("kind=Some(Other) errno=None"));
        }
        let error = io::Error::from_raw_os_error(libc::ENOENT);
        assert_eq!(
            context.failure(Operation::Spawn, Some(&error)).to_string(),
            "native-process-test invocation=18446744073709551615 phase=Run operation=Spawn kind=Some(NotFound) errno=Some(2)"
        );
        assert!(
            context
                .failure(Operation::StdoutPipe, None)
                .to_string()
                .ends_with("kind=None errno=None")
        );
    }

    #[test]
    fn capture_threads_retain_one_context_without_cross_invocation_state() {
        let first = Context::new(ProcessPhase::Probe);
        let second = Context::new(ProcessPhase::Link);
        assert_ne!(first.invocation, second.invocation);
        let stdout = std::thread::spawn(move || {
            first.failure(Operation::ReadStdout, None).context.invocation
        });
        let stderr = std::thread::spawn(move || {
            first.failure(Operation::ReadStderr, None).context.invocation
        });
        assert_eq!(stdout.join().expect("stdout context"), first.invocation);
        assert_eq!(stderr.join().expect("stderr context"), first.invocation);
        assert_ne!(first.invocation, second.invocation);
    }

    #[test]
    fn spawn_failure_keeps_the_complete_stable_diagnostic() {
        let diagnostic = run_bounded_process(
            Path::new("/dev/null/not-an-executable"),
            &[],
            Path::new("/tmp"),
            Duration::from_secs(1),
            16,
            16,
            ProcessPhase::Run,
            None,
        )
        .expect_err("non-directory executable path must fail at spawn");
        assert_eq!(diagnostic, process_io_error());
        assert_eq!(
            diagnostic,
            zryna_diagnostics::Diagnostic::error(
                "ZRYNA-N4006",
                None,
                "native system process could not be started or observed safely",
                "verify the documented system toolchain and retry on Linux x86-64",
            )
        );
    }

    struct FailedRead {
        calls: usize,
        interrupted: bool,
    }

    impl Read for FailedRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.interrupted {
                Err(io::Error::from_raw_os_error(libc::EINTR))
            } else {
                Err(io::Error::other("/private/path TOKEN=secret"))
            }
        }
    }

    #[test]
    fn read_failure_keeps_diagnostic_and_does_not_add_retries() {
        for interrupted in [false, true] {
            for operation in [Operation::ReadStdout, Operation::ReadStderr] {
                let mut reader = FailedRead { calls: 0, interrupted };
                let (sender, receiver) = mpsc::sync_channel(2);
                let result = capture_stream(
                    &mut reader,
                    16,
                    &sender,
                    (Context::new(ProcessPhase::Run), operation),
                );
                assert!(matches!(result, Err(ref diagnostic) if *diagnostic == process_io_error()));
                assert_eq!(reader.calls, 1);
                assert!(matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)));
            }
        }
    }
}
