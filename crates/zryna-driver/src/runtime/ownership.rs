use super::*;

impl NodeRuntimeCapability {
    pub(crate) fn run_ownership_javascript(
        &self,
        harness: &Path,
        working_directory: &Path,
    ) -> Result<[u8; 8], Diagnostic> {
        self.run_module_frame(harness, working_directory, 8)?
            .try_into()
            .map_err(|_| invalid_result_frame())
    }

    pub(crate) fn run_ownership_webassembly(
        &self,
        script: &[u8],
        module: &[u8],
        working_directory: &Path,
    ) -> Result<[u8; 8], Diagnostic> {
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
            8,
            MAX_STDERR,
        )?;
        self.revalidate()?;
        if !output.status.success() || !output.stderr.is_empty() || output.stdout.len() != 8 {
            return Err(invalid_result_frame());
        }
        output.stdout.try_into().map_err(|_| invalid_result_frame())
    }
}
