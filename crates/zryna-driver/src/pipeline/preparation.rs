//! Target preparation from one default-profile verified program.

use zryna_frontend::VerifiedFrontendProvider;
use zryna_source::SourceMap;

use super::{
    CommandFailure, CommandFailureKind, NATIVE_TARGET, TargetSelection, preparation_failure,
    source_failure,
};

pub(super) struct PreparedArtifacts {
    pub(super) javascript: Option<zryna_backend_javascript::JavaScriptArtifact>,
    pub(super) webassembly: Option<zryna_backend_webassembly::ValidatedWebAssemblyArtifact>,
    pub(super) native_object: Option<zryna_backend_native::ValidatedNativeObjectArtifact>,
    pub(super) native_executable: Option<crate::native::PreparedNativeExecutable>,
    pub(super) component: Option<zryna_backend_webassembly::ValidatedScalarComponent>,
}

#[cfg(test)]
pub(super) fn compile_selected<Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
    targets: TargetSelection,
) -> Result<(crate::SourceToIrSuccess, PreparedArtifacts), CommandFailure> {
    let compiled = analyze(frontend, sources)?;
    let prepared = prepare_selected(&compiled, targets)?;
    Ok((compiled, prepared))
}

pub(super) fn analyze<Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
) -> Result<crate::SourceToIrSuccess, CommandFailure> {
    crate::compile_to_verified_ir(frontend, sources).map_err(|error| source_failure(&error))
}

pub(super) fn prepare_selected(
    compiled: &crate::SourceToIrSuccess,
    targets: TargetSelection,
) -> Result<PreparedArtifacts, CommandFailure> {
    let program = compiled.program();
    let javascript = if targets.javascript() {
        Some(zryna_backend_javascript::emit(program).map_err(preparation_failure)?)
    } else {
        None
    };
    let webassembly = if targets.webassembly() {
        Some(zryna_backend_webassembly::emit(program).map_err(preparation_failure)?)
    } else {
        None
    };
    let native_object = if targets.native() {
        let target =
            crate::select_native_object_target(NATIVE_TARGET).map_err(preparation_failure)?;
        let mir = zryna_native_mir::lower(program).map_err(|diagnostics| CommandFailure {
            kind: CommandFailureKind::Preparation,
            diagnostics,
        })?;
        Some(zryna_backend_native::emit_object(&mir, target).map_err(preparation_failure)?)
    } else {
        None
    };
    let component = if targets.component() {
        Some(
            zryna_backend_webassembly::emit_scalar_component(
                program,
                &zryna_backend_webassembly::pinned_wit_sources(),
            )
            .map_err(preparation_failure)?,
        )
    } else {
        None
    };
    Ok(PreparedArtifacts {
        javascript,
        webassembly,
        native_object,
        native_executable: None,
        component,
    })
}
