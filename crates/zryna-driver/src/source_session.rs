//! Private source-session interface for checkout handles and resolver-owned root-package bytes.

use zryna_diagnostics::Diagnostic;
use zryna_source::NormalizedSourcePath;

use crate::workspace_source::{StableSource, WorkspaceSourceRoot, WorkspaceSourceSession};

pub(crate) trait ModuleSourceRoot {
    type Session<'root>: ModuleSourceSession
    where
        Self: 'root;

    fn begin(&self) -> Result<Self::Session<'_>, Diagnostic>;
}

pub(crate) trait ModuleSourceSession {
    fn read_source(&mut self, path: &NormalizedSourcePath) -> Result<StableSource, Diagnostic>;
    fn revalidate_all(&mut self) -> Result<(), Diagnostic>;
}

impl ModuleSourceRoot for WorkspaceSourceRoot {
    type Session<'root> = WorkspaceSourceSession<'root>;

    fn begin(&self) -> Result<Self::Session<'_>, Diagnostic> {
        self.begin_discovery()
    }
}

impl ModuleSourceSession for WorkspaceSourceSession<'_> {
    fn read_source(&mut self, path: &NormalizedSourcePath) -> Result<StableSource, Diagnostic> {
        Self::read_source(self, path)
    }

    fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        Self::revalidate_all(self)
    }
}
