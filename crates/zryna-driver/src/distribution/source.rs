//! Source session whose provider checkpoints retain both installation and project proofs.

use zryna_diagnostics::Diagnostic;
use zryna_source::{NormalizedSourcePath, SourceMap};

use crate::{
    project::ProjectAdmission,
    source_session::{ModuleSourceRoot, ModuleSourceSession},
    workspace_source::StableSource,
};

use super::{InstalledExecution, admission_error};

pub(crate) struct InstalledProjectSources<'proof, 'installation> {
    project: &'proof ProjectAdmission,
    execution: &'proof InstalledExecution<'installation>,
}

impl<'proof, 'installation> InstalledProjectSources<'proof, 'installation> {
    pub(crate) fn new(
        project: &'proof ProjectAdmission,
        execution: &'proof InstalledExecution<'installation>,
    ) -> Self {
        Self { project, execution }
    }
}

pub(crate) struct InstalledSourceSession<'proof, 'installation> {
    project: <ProjectAdmission as ModuleSourceRoot>::Session<'proof>,
    execution: &'proof InstalledExecution<'installation>,
}

impl<'installation> ModuleSourceRoot for InstalledProjectSources<'_, 'installation> {
    type Session<'proof>
        = InstalledSourceSession<'proof, 'installation>
    where
        Self: 'proof;

    fn begin(&self) -> Result<Self::Session<'_>, Diagnostic> {
        let mut session =
            InstalledSourceSession { project: self.project.begin()?, execution: self.execution };
        session.revalidate_all()?;
        Ok(session)
    }
}

impl ModuleSourceSession for InstalledSourceSession<'_, '_> {
    fn read_source(&mut self, path: &NormalizedSourcePath) -> Result<StableSource, Diagnostic> {
        self.project.read_source(path)
    }

    fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        self.project.revalidate_all()?;
        self.revalidate_execution()
    }

    fn validate_provider_batch(&mut self, sources: &SourceMap) -> Result<(), Diagnostic> {
        self.project.validate_provider_batch(sources)?;
        self.revalidate_execution()
    }
}

impl InstalledSourceSession<'_, '_> {
    fn revalidate_execution(&self) -> Result<(), Diagnostic> {
        self.execution.revalidate().map_err(|failure| {
            failure
                .diagnostics
                .into_iter()
                .next()
                .unwrap_or_else(|| admission_error("installed provider admission changed"))
        })
    }
}
