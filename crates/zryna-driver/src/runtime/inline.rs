//! Retained-byte transport shared by scalar ESM and core WebAssembly consumers.

use super::{
    Diagnostic, Duration, MAX_INLINE_MODULE_BYTES, MAX_STDERR, MAX_WEBASSEMBLY_INPUT_BYTES,
    NodeRuntimeCapability, OsString, PROCESS_TIMEOUT, Path, invalid_result_frame,
    node_compatible_path, run_bounded_with_timeout, runtime_error,
};

impl NodeRuntimeCapability {
    pub(crate) fn run_webassembly_module(
        &self,
        script: &[u8],
        module: &[u8],
        working_directory: &Path,
    ) -> Result<[u8; 4], Diagnostic> {
        let frame = self.run_inline_module(script, module, working_directory, 4)?;
        if frame.len() != 4 {
            return Err(invalid_result_frame());
        }
        frame.try_into().map_err(|_| invalid_result_frame())
    }

    pub(crate) fn run_inline_module(
        &self,
        script: &[u8],
        module: &[u8],
        working_directory: &Path,
        stdout_limit: usize,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.run_inline_with_timeout(
            script,
            module,
            working_directory,
            stdout_limit,
            PROCESS_TIMEOUT,
        )
    }

    #[cfg(test)]
    pub(crate) fn run_browser_fixture(
        &self,
        script: &[u8],
        input: &[u8],
        working_directory: &Path,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.run_inline_with_timeout(
            script,
            input,
            working_directory,
            64 * 1024,
            Duration::from_secs(30),
        )
    }

    fn run_inline_with_timeout(
        &self,
        script: &[u8],
        module: &[u8],
        working_directory: &Path,
        stdout_limit: usize,
        timeout: Duration,
    ) -> Result<Vec<u8>, Diagnostic> {
        if script.len() > MAX_INLINE_MODULE_BYTES || module.len() > MAX_WEBASSEMBLY_INPUT_BYTES {
            return Err(runtime_error(
                "ZRYNA-R3004",
                "target runtime input exceeded its hard byte budget",
                "reduce the sealed artifact and report a reproducible boundary failure",
            ));
        }
        let script = std::str::from_utf8(script).map_err(|_| {
            runtime_error(
                "ZRYNA-R3006",
                "target runtime received an invalid inline module",
                "report the smallest reproducible source and verified invocation",
            )
        })?;
        self.revalidate()?;
        let output = run_bounded_with_timeout(
            &self.invocation_path,
            &[
                OsString::from("--input-type=module"),
                OsString::from("--eval"),
                OsString::from(script),
            ],
            &node_compatible_path(working_directory),
            Some(module),
            stdout_limit,
            MAX_STDERR,
            timeout,
        )?;
        self.revalidate()?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err(invalid_result_frame());
        }
        Ok(output.stdout)
    }
}
