//! Source-bound ordinary invocation-only transport, disjoint from observer/injection artifacts.
use super::{Diagnostic, artifact, budget, code, memory, model::Model};
use zryna_abi::{Invocation, VerifiedInvocation};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) const ENTRY: &str = "zryna$invoke$v1";
/// Audited ordinary private invoker bound to one original NEW execution and exact request.
#[derive(Debug)]
pub struct ValidatedInvocationArtifact<'p, 'a> {
    source: &'p VerifiedExecution<'a>,
    invocation: VerifiedInvocation<'p>,
    bytes: Vec<u8>,
    entry: &'static str,
}
impl<'p, 'a> ValidatedInvocationArtifact<'p, 'a> {
    /// Returns the exact NEW source execution used for this carrier.
    #[must_use]
    pub const fn source(&self) -> &'p VerifiedExecution<'a> {
        self.source
    }
    /// Returns the request independently verified against that exact source ABI.
    #[must_use]
    pub const fn invocation(&self) -> &VerifiedInvocation<'p> {
        &self.invocation
    }
    /// Returns the complete core module with only the reserved invoker exported.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the fixed zero-argument packed-result entry export.
    #[must_use]
    pub const fn entry_export(&self) -> &str {
        self.entry
    }
    /// Audits actual bytes against this same source, request, memory and closed code.
    /// # Errors
    /// Rejects capability, section, body, source-call, cleanup or transport drift.
    pub fn audit_bytes(&self, actual: &[u8]) -> Result<(), Diagnostic> {
        with_model(self.source, &self.invocation, |m, code| {
            artifact::audit_invocation(actual, m, code, &self.invocation)
        })
    }
}
/// Emits an ordinary invocation-only module from NEW source authority only.
/// ```compile_fail
/// fn raw(p: &zryna_ir::generic_v1::execution_v1::raw::Program, r: zryna_abi::Invocation) {
///     zryna_backend_webassembly::generic_execution_v1::emit_invocation(p,r);
/// }
/// ```
/// ```compile_fail
/// fn old(p: &zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram<'_>, r: zryna_abi::Invocation) {
///     zryna_backend_webassembly::generic_execution_v1::emit_invocation(p,r);
/// }
/// ```
/// # Errors
/// Rejects invalid exact-source ABI requests and unchanged target resource limits.
pub fn emit_invocation<'p, 'a>(
    source: &'p VerifiedExecution<'a>,
    request: Invocation,
) -> Result<ValidatedInvocationArtifact<'p, 'a>, Diagnostic> {
    let invocation = source.scalar_abi().prepare_invocation(request).map_err(|error| {
        Diagnostic::error(
            error.code(),
            None,
            "generic Wasm invocation does not match this source ABI",
            "use the exact scalar export and typed arguments",
        )
    })?;
    let bytes = with_model(source, &invocation, |m, code| {
        let bytes = artifact::encode_invocation(m, code)?;
        artifact::audit_invocation(&bytes, m, code, &invocation)?;
        Ok(bytes)
    })?;
    Ok(ValidatedInvocationArtifact { source, invocation, bytes, entry: ENTRY })
}
fn with_model<R>(
    source: &VerifiedExecution<'_>,
    request: &VerifiedInvocation<'_>,
    inspect: impl FnOnce(&Model<'_, '_, '_>, &[code::FunctionCode]) -> Result<R, Diagnostic>,
) -> Result<R, Diagnostic> {
    let plan = memory::verify(memory::derive(source, false)?, source)?;
    let m = Model::new(source, &plan)?;
    let mut code = code::build(&m)?;
    m.take(size_of::<code::FunctionCode>())?;
    m.take(extra_work(request.arguments().len())?)?;
    code.try_reserve_exact(1).map_err(|_| budget())?;
    code.push(code::invocation::build(&m, request)?);
    inspect(&m, &code)
}
fn extra_work(arguments: usize) -> Result<usize, Diagnostic> {
    arguments.checked_mul(128).and_then(|n| n.checked_add(4096)).ok_or_else(budget)
}
#[cfg(test)]
mod tests;
