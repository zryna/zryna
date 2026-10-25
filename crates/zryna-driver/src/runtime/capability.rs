//! Existing Node admission and retained executable identity checks.
use super::{
    Diagnostic, ExpectedRuntime, NodeRuntimeCapability, Path, admission, open_runtime_identity,
    runtime_error, same_file_state,
};

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
}
