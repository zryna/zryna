//! Only separately typed source-owned carriers enter this finite Node host adapter.
use super::{NodeRuntimeCapability, Path};
use zryna_abi::{ScalarOutcome, ScalarTrapCode};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{invocation, raw::TrapIdentity};
mod scripts;

impl NodeRuntimeCapability {
    pub(crate) fn run_generic_javascript(
        &self,
        artifact: &zryna_backend_javascript::generic_execution_v1::ValidatedInvocationArtifact<
            '_,
            '_,
        >,
        directory: &Path,
    ) -> Result<ScalarOutcome, Diagnostic> {
        artifact.audit_source(artifact.as_source())?;
        let script = scripts::javascript(artifact.entry_export(), artifact.encode_export())?;
        let bytes = self.run_inline_module(
            &script,
            artifact.as_source().as_bytes(),
            directory,
            invocation::RECORD_BYTES,
        )?;
        outcome(&bytes, artifact.invocation().export().result())
    }

    pub(crate) fn run_generic_webassembly(
        &self,
        artifact: &zryna_backend_webassembly::generic_execution_v1::ValidatedInvocationArtifact<
            '_,
            '_,
        >,
        directory: &Path,
    ) -> Result<ScalarOutcome, Diagnostic> {
        artifact.audit_bytes(artifact.as_bytes())?;
        let script =
            scripts::webassembly(artifact.entry_export(), artifact.invocation().export().result())?;
        let bytes = self.run_inline_module(
            &script,
            artifact.as_bytes(),
            directory,
            invocation::RECORD_BYTES,
        )?;
        outcome(&bytes, artifact.invocation().export().result())
    }
}

fn outcome(bytes: &[u8], result: zryna_abi::ScalarType) -> Result<ScalarOutcome, Diagnostic> {
    match invocation::decode(bytes, result).map_err(|_| invalid())? {
        invocation::Outcome::Value(value) => Ok(ScalarOutcome::Returned { value }),
        invocation::Outcome::Trap(trap) => Ok(ScalarOutcome::Trapped {
            code: match trap {
                TrapIdentity::Bounds => ScalarTrapCode::Bounds,
                TrapIdentity::Allocation => ScalarTrapCode::Allocation,
                TrapIdentity::Capacity => ScalarTrapCode::Capacity,
                TrapIdentity::Refcount => ScalarTrapCode::Refcount,
                TrapIdentity::Utf8 => ScalarTrapCode::Utf8,
            },
        }),
    }
}

fn invalid() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C3412",
        None,
        "generic Node execution returned an invalid source outcome record",
        "retain the verified invocation and exact normally completed host response",
    )
}
