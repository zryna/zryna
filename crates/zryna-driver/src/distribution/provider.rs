//! Profile-specific worker specifications over one retained installation and provider stage.

use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, ProviderExpectationV3, ProviderExpectationV4,
    WorkerFrontend, WorkerFrontendV3, WorkerFrontendV4, WorkerLimits, WorkerLimitsV3,
    WorkerLimitsV4, WorkerSpec, WorkerSpecV3, WorkerSpecV4, syntax_v2,
};

use crate::{
    CommandFailure, CommandFailureKind,
    runtime::{NodeRuntimeCapability, node_compatible_path},
};

use super::{InstalledCompiler, stage::ProviderStage};

pub(crate) struct InstalledExecution<'installation> {
    installation: &'installation InstalledCompiler,
    stage: ProviderStage,
    node: NodeRuntimeCapability,
}

impl<'installation> InstalledExecution<'installation> {
    pub(crate) fn prepare(
        installation: &'installation InstalledCompiler,
    ) -> Result<Self, CommandFailure> {
        let stage = ProviderStage::create(installation).map_err(failure)?;
        let node = installation.runtime().map_err(failure)?;
        let execution = Self { installation, stage, node };
        execution.revalidate()?;
        Ok(execution)
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandFailure> {
        self.installation.revalidate().map_err(failure)?;
        self.stage.revalidate().map_err(failure)?;
        self.node.revalidate().map_err(failure)
    }

    pub(crate) fn node(&self) -> Result<&NodeRuntimeCapability, CommandFailure> {
        self.revalidate()?;
        Ok(&self.node)
    }

    pub(crate) fn frontend_v2(&self) -> Result<WorkerFrontend, CommandFailure> {
        self.revalidate()?;
        let expected = ProviderExpectation::new(
            "typescript-6",
            "6.0.3",
            syntax_v2::PROTOCOL_VERSION,
            FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
        )
        .map_err(|error| worker_failure(error.diagnostics()))?;
        let worker = node_compatible_path(&self.stage.worker("worker.mjs").map_err(failure)?);
        let directory = node_compatible_path(&self.stage.working_directory().map_err(failure)?);
        let spec = WorkerSpec::new(
            self.node.executable().map_err(failure)?,
            vec![worker.into_os_string()],
            directory,
            expected,
            WorkerLimits::default(),
        )
        .map_err(|error| worker_failure(error.diagnostics()))?;
        Ok(WorkerFrontend::new(spec))
    }

    pub(crate) fn frontend_v3(&self) -> Result<WorkerFrontendV3, CommandFailure> {
        self.revalidate()?;
        let expected = ProviderExpectationV3::new("typescript-6", "6.0.3")
            .map_err(|error| worker_failure(error.diagnostics()))?;
        let worker = node_compatible_path(&self.stage.worker("worker-v3.mjs").map_err(failure)?);
        let directory = node_compatible_path(&self.stage.working_directory().map_err(failure)?);
        let spec = WorkerSpecV3::new(
            self.node.executable().map_err(failure)?,
            vec![worker.into_os_string()],
            directory,
            expected,
            WorkerLimitsV3::default(),
        )
        .map_err(|error| worker_failure(error.diagnostics()))?;
        Ok(WorkerFrontendV3::new(spec))
    }

    pub(crate) fn frontend_v4(&self) -> Result<WorkerFrontendV4, CommandFailure> {
        self.revalidate()?;
        let expected = ProviderExpectationV4::new("typescript-6", "6.0.3")
            .map_err(|error| worker_failure(error.diagnostics()))?;
        let worker = node_compatible_path(&self.stage.worker("worker-v4.mjs").map_err(failure)?);
        let directory = node_compatible_path(&self.stage.working_directory().map_err(failure)?);
        let spec = WorkerSpecV4::new(
            self.node.executable().map_err(failure)?,
            vec![worker.into_os_string()],
            directory,
            expected,
            WorkerLimitsV4::default(),
        )
        .map_err(|error| worker_failure(error.diagnostics()))?;
        Ok(WorkerFrontendV4::new(spec))
    }
}

fn worker_failure(diagnostics: &[zryna_diagnostics::Diagnostic]) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Preparation, diagnostics: diagnostics.to_vec() }
}

fn failure(diagnostic: zryna_diagnostics::Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Preparation, diagnostics: vec![diagnostic] }
}
