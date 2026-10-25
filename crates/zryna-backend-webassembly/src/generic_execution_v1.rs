//! New full-execution core Wasm checkpoint with independently checked owned memory.
use crate::ValidatedWebAssemblyArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod artifact;
mod bytes;
mod code;
mod invocation;
pub mod memory;
mod model;
pub use invocation::{ValidatedInvocationArtifact, emit_invocation};
mod recursive;
mod work;
const MAX_BYTES: usize = 32 * 1024 * 1024;
/// Explicitly instrumented test artifact, distinct from ordinary scalar-ABI artifacts.
#[derive(Debug)]
pub struct ObservedExecutionArtifact {
    bytes: Vec<u8>,
}
impl ObservedExecutionArtifact {
    /// Exact separately audited bytes, including only the reserved test observer export.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
/// Emits ordinary new-domain core Wasm with no imports or non-scalar exports.
/// # Errors
/// Rejects unsupported checkpoint rows, invalid memory geometry, budgets or actual-byte audit.
pub fn emit(program: &VerifiedExecution<'_>) -> Result<ValidatedWebAssemblyArtifact, Diagnostic> {
    let claim = memory::derive(program, false)?;
    let memory = memory::verify(claim, program)?;
    let model = model::Model::new(program, &memory)?;
    let code = code::build(&model)?;
    let bytes = artifact::encode(&model, &code)?;
    artifact::audit(&bytes, &model, &code)?;
    Ok(ValidatedWebAssemblyArtifact { bytes })
}
/// Emits the frozen explicit test-only observation policy; ordinary output is unchanged.
/// # Errors
/// Rejects unsupported rows, changed same-program memory policy and actual audit mismatches.
pub fn emit_observed(
    program: &VerifiedExecution<'_>,
) -> Result<ObservedExecutionArtifact, Diagnostic> {
    let claim = memory::derive(program, true)?;
    let memory = memory::verify(claim, program)?;
    let model = model::Model::new(program, &memory)?;
    let code = code::build(&model)?;
    let bytes = artifact::encode(&model, &code)?;
    artifact::audit(&bytes, &model, &code)?;
    Ok(ObservedExecutionArtifact { bytes })
}
fn budget() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4101",
        None,
        "full execution Wasm exceeds inherited memory, work or artifact budget",
        "retain the existing 256-page owned-memory and 32 MiB artifact ceilings",
    )
}
fn invariant() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4102",
        None,
        "full execution Wasm same-seal lowering or artifact invariant failed",
        "retain the exact new execution, runtime, layouts and checked target memory plan",
    )
}

#[cfg(test)]
mod tests;

fn checked_u32<T: TryInto<u32>>(value: T) -> Result<u32, Diagnostic> {
    value.try_into().map_err(|_| budget())
}
fn checked_u8<T: TryInto<u8>>(value: T) -> Result<u8, Diagnostic> {
    value.try_into().map_err(|_| budget())
}

fn checked_i64<T: TryInto<i64>>(value: T) -> Result<i64, Diagnostic> {
    value.try_into().map_err(|_| budget())
}

#[cfg(test)]
mod accounting_tests;
