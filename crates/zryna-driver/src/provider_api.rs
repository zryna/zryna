//! Explicit compiler-owned provider selection, separate from target execution runtimes.

use std::path::{Path, PathBuf};

mod v5;

use zryna_abi::ScalarValue;
use zryna_frontend::{
    VerifiedFrontendProvider, VerifiedFrontendProviderV3, VerifiedFrontendProviderV4,
};

use crate::native_provider::{
    ComparisonFrontendV2, ComparisonFrontendV3, ComparisonFrontendV4, NativeProviderCapability,
};
use crate::runtime::NodeRuntimeCapability;
use crate::{
    CommandFailure, CommandFailureKind, CommandSuccess, PublishedOwnershipBundle, TargetSelection,
};

/// The driver admits exactly one provider mode; divergence never triggers fallback.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompilerProvider {
    /// The native lexer/parser in the authenticated compiler executable.
    #[default]
    Native,
    /// Explicit compatibility with the pinned TypeScript bootstrap.
    Bootstrap,
    /// Both verified providers must agree before semantic or backend work.
    Compare,
}

/// Existing source profiles; provider selection does not add language features.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompilationProfile {
    /// Default scalar syntax protocol v2.
    #[default]
    I32,
    /// Existing control-flow syntax protocol v3.
    ControlFlow,
    /// Existing data-ownership syntax protocol v4.
    DataOwnership,
    /// Explicit source-checkout bounded generics, Option and Result using syntax protocol v5.
    BoundedGenericsV1,
}

impl CompilationProfile {
    pub(crate) const fn package_profile(self) -> &'static str {
        match self {
            Self::I32 => "i32-v1",
            Self::ControlFlow => "control-flow-v1",
            Self::DataOwnership => "data-ownership-v1",
            Self::BoundedGenericsV1 => "bounded-generics-v1",
        }
    }
}

/// Source authority remains distinct from the compiler and runtime installations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationSource {
    /// A retained source-checkout root.
    Workspace(PathBuf),
    /// A frozen, package-authenticated standalone project.
    Project(PathBuf),
}

impl CompilationSource {
    pub(crate) fn root(&self) -> &Path {
        match self {
            Self::Workspace(root) | Self::Project(root) => root,
        }
    }
}

/// Provider-aware command inputs. Absence of Node is represented explicitly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderBuildRequest {
    /// Trusted compiler checkout; installed dispatch supplies its retained installation instead.
    pub compiler_root: PathBuf,
    /// Source root, with its exact independent admission policy.
    pub source: CompilationSource,
    /// Portable source-relative entrypoint.
    pub entrypoint: String,
    /// Portable output stem.
    pub artifact_stem: String,
    /// Existing target selection.
    pub targets: TargetSelection,
    /// Existing language profile.
    pub profile: CompilationProfile,
    /// Exact provider choice, defaulting to native in the CLI.
    pub provider: CompilerProvider,
    /// Optional explicit Node host for bootstrap comparison or JavaScript/WebAssembly execution.
    pub node_runtime: Option<PathBuf>,
}

/// The original profile-owned published result; no provider identity enters target manifests.
#[derive(Clone, Debug)]
pub enum ProviderCommandSuccess {
    /// Existing scalar or control-flow manifest.
    Scalar(CommandSuccess),
    /// Existing data-ownership manifest.
    Ownership(PublishedOwnershipBundle),
    /// Separately versioned bounded-generics build or completed run.
    Generic(crate::PublishedGenericBundle),
}

/// Compiles using one exact driver-selected provider without requiring a target runtime.
///
/// # Errors
/// Rejects missing authority, provider divergence, invalid source or failed atomic publication.
pub fn build_with_provider(
    request: &ProviderBuildRequest,
) -> Result<ProviderCommandSuccess, CommandFailure> {
    crate::pipeline::providers::execute(request, None, None)
}

/// Compiles and runs the selected existing target; Node is required only by a Node target host.
///
/// # Errors
/// Preserves all original source, invocation, runtime, cleanup and publication failures.
pub fn run_with_provider(
    request: &ProviderBuildRequest,
    export: String,
    arguments: Vec<ScalarValue>,
) -> Result<ProviderCommandSuccess, CommandFailure> {
    crate::pipeline::providers::execute(request, Some((export, arguments)), None)
}

pub(crate) struct ProviderSession<'request> {
    compiler_root: &'request Path,
    provider: CompilerProvider,
    native: Option<NativeProviderCapability>,
    node: Option<NodeRuntimeCapability>,
}

impl<'request> ProviderSession<'request> {
    pub(crate) fn prepare(
        request: &'request ProviderBuildRequest,
        running: bool,
    ) -> Result<Self, CommandFailure> {
        let needs_runtime =
            running && (request.targets.javascript() || request.targets.webassembly());
        Self::prepare_at(
            &request.compiler_root,
            request.provider,
            request.node_runtime.as_deref(),
            needs_runtime,
        )
    }

    pub(crate) fn prepare_syntax(
        compiler_root: &'request Path,
        provider: CompilerProvider,
        node_runtime: Option<&Path>,
    ) -> Result<Self, CommandFailure> {
        Self::prepare_at(compiler_root, provider, node_runtime, false)
    }

    fn prepare_at(
        compiler_root: &'request Path,
        provider: CompilerProvider,
        node_runtime: Option<&Path>,
        target_needs_node: bool,
    ) -> Result<Self, CommandFailure> {
        let native = if provider == CompilerProvider::Bootstrap {
            None
        } else {
            Some(NativeProviderCapability::capture_current().map_err(preparation_failure)?)
        };
        let node = if provider != CompilerProvider::Native || target_needs_node {
            let path = node_runtime.ok_or_else(|| {
                request_failure(
                    "the selected compatibility provider or target host requires --node",
                )
            })?;
            Some(
                NodeRuntimeCapability::discover(path, compiler_root)
                    .map_err(preparation_failure)?,
            )
        } else {
            None
        };
        let session = Self { compiler_root, provider, native, node };
        session.revalidate()?;
        Ok(session)
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandFailure> {
        if let Some(native) = &self.native {
            native.revalidate().map_err(preparation_failure)?;
        }
        if let Some(node) = &self.node {
            node.revalidate().map_err(preparation_failure)?;
        }
        Ok(())
    }

    pub(crate) fn node(&self) -> Option<&NodeRuntimeCapability> {
        self.node.as_ref()
    }

    fn native(&self) -> Result<&NativeProviderCapability, CommandFailure> {
        self.native.as_ref().ok_or_else(|| request_failure("native provider authority is absent"))
    }

    fn bootstrap_node(&self) -> Result<&NodeRuntimeCapability, CommandFailure> {
        self.node.as_ref().ok_or_else(|| request_failure("bootstrap runtime authority is absent"))
    }

    pub(crate) fn frontend_v2(&self) -> Result<Box<dyn VerifiedFrontendProvider>, CommandFailure> {
        self.revalidate()?;
        let native = || self.native()?.frontend_v2().map_err(preparation_failure);
        let bootstrap = || {
            crate::pipeline::preparation::configured_frontend(
                self.compiler_root,
                self.bootstrap_node()?,
            )
        };
        Ok(match self.provider {
            CompilerProvider::Native => Box::new(native()?),
            CompilerProvider::Bootstrap => Box::new(bootstrap()?),
            CompilerProvider::Compare => {
                Box::new(ComparisonFrontendV2::new(native()?, bootstrap()?))
            }
        })
    }

    pub(crate) fn frontend_v3(
        &self,
    ) -> Result<Box<dyn VerifiedFrontendProviderV3>, CommandFailure> {
        self.revalidate()?;
        let native = || self.native()?.frontend_v3().map_err(preparation_failure);
        let bootstrap = || {
            let node = self.bootstrap_node()?;
            crate::pipeline::configured_frontend_v3_at(self.compiler_root, node)
        };
        Ok(match self.provider {
            CompilerProvider::Native => Box::new(native()?),
            CompilerProvider::Bootstrap => Box::new(bootstrap()?),
            CompilerProvider::Compare => {
                Box::new(ComparisonFrontendV3::new(native()?, bootstrap()?))
            }
        })
    }

    pub(crate) fn frontend_v4(
        &self,
    ) -> Result<Box<dyn VerifiedFrontendProviderV4>, CommandFailure> {
        self.revalidate()?;
        let native = || self.native()?.frontend_v4().map_err(preparation_failure);
        let bootstrap = || {
            crate::ownership_pipeline::configured_frontend_at(
                &self.compiler_root.join("adapters/typescript-6"),
                self.bootstrap_node()?,
            )
        };
        Ok(match self.provider {
            CompilerProvider::Native => Box::new(native()?),
            CompilerProvider::Bootstrap => Box::new(bootstrap()?),
            CompilerProvider::Compare => {
                Box::new(ComparisonFrontendV4::new(native()?, bootstrap()?))
            }
        })
    }
}

pub(crate) fn preparation_failure(diagnostic: zryna_diagnostics::Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Preparation, diagnostics: vec![diagnostic] }
}

pub(crate) fn request_failure(message: impl Into<String>) -> CommandFailure {
    CommandFailure {
        kind: CommandFailureKind::Request,
        diagnostics: vec![zryna_diagnostics::Diagnostic::error(
            "ZRYNA-C1001",
            None,
            message,
            "select native, bootstrap or compare explicitly; provide a direct Node.js host only when required",
        )],
    }
}

#[cfg(test)]
mod tests;
