use super::*;

impl NodeRuntimeCapability {
    pub(crate) fn run_ownership_javascript(
        &self,
        harness: &Path,
        working_directory: &Path,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.revalidate()?;
        let output = run_bounded(
            &self.invocation_path,
            &[node_compatible_path(harness).into_os_string()],
            &node_compatible_path(working_directory),
            None,
            12 + 4096 * 4,
            MAX_STDERR,
        )?;
        self.revalidate()?;
        if !output.status.success() || !output.stderr.is_empty() || output.stdout.len() < 12 {
            return Err(invalid_result_frame());
        }
        Ok(output.stdout)
    }

    pub(crate) fn run_ownership_webassembly(
        &self,
        script: &[u8],
        module: &[u8],
        working_directory: &Path,
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
        let node_working_directory = node_compatible_path(working_directory);
        let output = run_bounded(
            &self.invocation_path,
            &[
                OsString::from("--input-type=module"),
                OsString::from("--eval"),
                OsString::from(script),
            ],
            &node_working_directory,
            Some(module),
            12 + 4096 * 4,
            MAX_STDERR,
        )?;
        self.revalidate()?;
        if !output.status.success() || !output.stderr.is_empty() || output.stdout.len() < 12 {
            return Err(invalid_result_frame());
        }
        Ok(output.stdout)
    }
}
