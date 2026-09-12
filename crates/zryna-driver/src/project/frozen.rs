//! Compile-side source requests consume resolver-captured root bytes, never host paths.

use sha2::{Digest as _, Sha256};
use zryna_diagnostics::Diagnostic;
use zryna_source::{NormalizedSourcePath, SourceMap};

use crate::{
    CommandFailure,
    source_session::{ModuleSourceRoot, ModuleSourceSession},
    workspace_source::StableSource,
};

use super::ProjectAdmission;

pub(super) enum RootPackageSources {
    Snapshot(Vec<zryna_package::PackageFile>),
    Retained(Box<crate::package_resolution::CapturedProject>),
}

impl RootPackageSources {
    pub(super) fn snapshot(
        request: &crate::PackageResolutionRequest,
    ) -> Result<(crate::PackageResolutionSuccess, Self), zryna_package::ResolveError> {
        crate::package_resolution::resolve_project_package(request)
            .map(|(success, files)| (success, Self::Snapshot(files)))
    }

    pub(super) fn retained(
        request: &crate::PackageResolutionRequest,
    ) -> Result<(crate::PackageResolutionSuccess, Self), zryna_package::ResolveError> {
        crate::package_resolution::capture_project_package(request)
            .map(|(success, capture)| (success, Self::Retained(Box::new(capture))))
    }

    pub(super) fn root_file(&self, path: &str) -> Option<&[u8]> {
        match self {
            Self::Snapshot(files) => {
                files.iter().find(|file| file.path == path).map(|file| file.bytes.as_slice())
            }
            Self::Retained(capture) => capture.root_file(path),
        }
    }

    pub(super) fn revalidate_retained(&self) -> Result<(), zryna_package::ResolveError> {
        match self {
            // Checkout M1 retains its existing graph/byte replay check in ProjectAdmission.
            Self::Snapshot(_) => Ok(()),
            Self::Retained(capture) => capture.revalidate(),
        }
    }
}

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
            Diagnostic::error(
                "ZRYNA-P4004",
                None,
                "imported source is absent from the authenticated root package",
                "declare the exact relative .zry source in the root package manifest",
            )
        })?;
        let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
            Diagnostic::error(
                "ZRYNA-P4004",
                None,
                "captured root package source is not UTF-8",
                "store canonical UTF-8 source in the root package",
            )
        })?;
        Ok(StableSource { text, sha256: Sha256::digest(bytes).into() })
    }

    fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        self.project.revalidate().map_err(diagnostic)
    }

    fn validate_provider_batch(&mut self, sources: &SourceMap) -> Result<(), Diagnostic> {
        let reject = || {
            Diagnostic::error(
                "ZRYNA-P4004",
                None,
                "provider batch differs from the captured root package",
                "use only exact resolver-admitted source paths and bytes",
            )
        };
        for index in 0..sources.len() {
            let raw = u32::try_from(index).map_err(|_| reject())?;
            let id = sources.verify_file_id(raw).map_err(|_| reject())?;
            let source = sources.source(id).ok_or_else(reject)?;
            if self.project.sources.root_file(source.path().as_str())
                != Some(source.text().as_bytes())
            {
                return Err(reject());
            }
        }
        Ok(())
    }
}

fn diagnostic(failure: CommandFailure) -> Diagnostic {
    failure.diagnostics.into_iter().next().unwrap_or_else(|| {
        Diagnostic::error(
            "ZRYNA-P4010",
            None,
            "project source admission changed during compilation",
            "restore the authenticated root package and retry",
        )
    })
}
