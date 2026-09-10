//! Direct runtime admission, with distribution bytes checked before any execution.

use std::{
    ffi::OsString,
    io::{Read as _, Seek as _, SeekFrom},
    path::Path,
};

use same_file::Handle;
use sha2::{Digest as _, Sha256};
use zryna_diagnostics::Diagnostic;

use super::{
    MAX_STDERR, MAX_VERSION_STDOUT, NodeRuntimeCapability, is_pinned_node_version,
    node_compatible_path, open_runtime_identity, run_bounded, runtime_error,
    stable_invocation_path,
};

#[derive(Clone, Debug)]
pub(crate) struct ExpectedRuntime {
    pub(crate) sha256: String,
    pub(crate) size: u64,
}

pub(super) fn discover(
    executable: &Path,
    working_directory: &Path,
    expected: Option<ExpectedRuntime>,
) -> Result<NodeRuntimeCapability, Diagnostic> {
    if !executable.is_absolute() || !working_directory.is_absolute() {
        return Err(runtime_error(
            "ZRYNA-R3001",
            "Node.js runtime and working directory must be absolute paths",
            "pass the absolute path of the documented Node.js 22.22.1 executable",
        ));
    }
    let (identity, state) = open_runtime_identity(executable)?;
    if let Some(expected) = &expected {
        authenticate(&identity, expected)?;
    }
    let invocation_path = stable_invocation_path(executable, &identity)?;
    let node_working_directory = node_compatible_path(working_directory);
    let output = run_bounded(
        &invocation_path,
        &[OsString::from("--version")],
        &node_working_directory,
        None,
        MAX_VERSION_STDOUT,
        MAX_STDERR,
    )?;
    if !output.status.success()
        || !output.stderr.is_empty()
        || !is_pinned_node_version(&output.stdout)
    {
        return Err(runtime_error(
            "ZRYNA-R3002",
            "Node.js runtime identity does not match the pinned Zryna runtime",
            "install Node.js 22.22.1 and pass its absolute executable path",
        ));
    }
    let capability = NodeRuntimeCapability {
        executable: executable.to_path_buf(),
        invocation_path,
        identity,
        state,
        expected,
    };
    capability.revalidate()?;
    Ok(capability)
}

pub(super) fn authenticate(
    identity: &Handle,
    expected: &ExpectedRuntime,
) -> Result<(), Diagnostic> {
    if expected.size == 0
        || expected.size > 256 * 1024 * 1024
        || expected.sha256.len() != 64
        || !expected
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(authentication_error());
    }
    let mut file = identity.as_file().try_clone().map_err(|_| authentication_error())?;
    if file.metadata().map_err(|_| authentication_error())?.len() != expected.size {
        return Err(authentication_error());
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| authentication_error())?;
    let mut reader = file.take(expected.size + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = reader.read(&mut buffer).map_err(|_| authentication_error())?;
        if count == 0 {
            break;
        }
        total += u64::try_from(count).map_err(|_| authentication_error())?;
        digest.update(&buffer[..count]);
    }
    if total != expected.size || format!("{:x}", digest.finalize()) != expected.sha256 {
        return Err(authentication_error());
    }
    Ok(())
}

fn authentication_error() -> Diagnostic {
    runtime_error(
        "ZRYNA-R3010",
        "installed Node.js bytes differ from the authenticated distribution",
        "restore the independently verified archive before running the compiler",
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{ExpectedRuntime, authenticate, discover, open_runtime_identity};
    use sha2::{Digest as _, Sha256};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "zryna-runtime-admission-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create unique runtime fixture");
            Self(path)
        }

        fn executable(&self, bytes: &[u8]) -> PathBuf {
            let path = self.0.join("runtime");
            fs::write(&path, bytes).expect("write runtime fixture");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                    .expect("fixture mode");
            }
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0.join("runtime"));
            let _ = fs::remove_file(self.0.join("invoked"));
            let _ = fs::remove_dir(&self.0);
        }
    }

    #[test]
    fn mismatched_runtime_bytes_fail_before_any_version_execution() {
        let fixture = Fixture::new();
        let script = b"#!/bin/sh\nprintf invoked > invoked\nprintf 'v22.22.1\\n'\n";
        let executable = fixture.executable(script);
        let expected = ExpectedRuntime { size: script.len() as u64, sha256: "a".repeat(64) };
        let error =
            discover(&executable, &fixture.0, Some(expected)).expect_err("reject before execution");
        assert_eq!(error.code(), "ZRYNA-R3010");
        assert!(!fixture.0.join("invoked").exists());
    }

    #[test]
    fn retained_runtime_bytes_reject_same_length_mutation_or_deny_the_write() {
        let fixture = Fixture::new();
        let original = b"original";
        let executable = fixture.executable(original);
        let (identity, _) = open_runtime_identity(&executable).expect("open runtime fixture");
        let expected = ExpectedRuntime {
            size: original.len() as u64,
            sha256: format!("{:x}", Sha256::digest(original)),
        };
        authenticate(&identity, &expected).expect("exact bytes match");
        if fs::write(&executable, b"modified").is_ok() {
            assert_eq!(
                authenticate(&identity, &expected).expect_err("changed bytes").code(),
                "ZRYNA-R3010"
            );
        }
    }

    #[test]
    fn invalid_runtime_size_or_digest_is_rejected_before_reading() {
        let fixture = Fixture::new();
        let executable = fixture.executable(b"x");
        let (identity, _) = open_runtime_identity(&executable).expect("open runtime fixture");
        for expected in [
            ExpectedRuntime { size: 0, sha256: "a".repeat(64) },
            ExpectedRuntime { size: 256 * 1024 * 1024 + 1, sha256: "a".repeat(64) },
            ExpectedRuntime { size: 1, sha256: "A".repeat(64) },
        ] {
            assert_eq!(
                authenticate(&identity, &expected).expect_err("invalid expectation").code(),
                "ZRYNA-R3010"
            );
        }
    }
}
