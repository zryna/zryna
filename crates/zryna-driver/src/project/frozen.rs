//! Compile-side source requests consume resolver-captured root bytes, never host paths.

use sha2::{Digest as _, Sha256};
use zryna_diagnostics::Diagnostic;
use zryna_source::NormalizedSourcePath;

use crate::{CommandFailure, source_session::{ModuleSourceRoot, ModuleSourceSession}, workspace_source::StableSource};

use super::ProjectAdmission;

pub(crate) struct ProjectSourceSession<'project> {
    project: &'project ProjectAdmission,
}

impl ModuleSourceRoot for ProjectAdmission {
    type Session<'project> = ProjectSourceSession<'project>;

    fn begin(&self) -> Result<Self::Session<'_>, Diagnostic> {
        self.revalidate().map_err(diagnostic)?;
        Ok(ProjectSourceSession { project: self })
    }
}

impl ModuleSourceSession for ProjectSourceSession<'_> {
    fn read_source(&mut self, path: &NormalizedSourcePath) -> Result<StableSource, Diagnostic> {
        let bytes = self.project.sources.root_file(path.as_str()).ok_or_else(|| {
            Diagnostic::error("ZRYNA-P4004", None,
                "imported source is absent from the authenticated root package",
                "declare the exact relative .zry source in the root package manifest")
        })?;
        let text = String::from_utf8(bytes.to_vec()).map_err(|_| Diagnostic::error(
            "ZRYNA-P4004", None, "captured root package source is not UTF-8",
            "store canonical UTF-8 source in the root package"))?;
        Ok(StableSource { text, sha256: Sha256::digest(bytes).into() })
    }

    fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        self.project.revalidate().map_err(diagnostic)
    }
}

fn diagnostic(failure: CommandFailure) -> Diagnostic {
    failure.diagnostics.into_iter().next().unwrap_or_else(|| Diagnostic::error(
        "ZRYNA-P4010", None, "project source admission changed during compilation",
        "restore the authenticated root package and retry"))
}
