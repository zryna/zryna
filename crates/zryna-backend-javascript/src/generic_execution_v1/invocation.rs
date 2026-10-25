//! Source-bound ordinary invocation-only module; decoded results are never program authority.
use super::{Context, Mode, Writer, context::charge, emission_budget, internal, runtime};
use zryna_abi::{Invocation, VerifiedInvocation};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod render;
pub(super) mod traps;
const ENTRY: &str = "zryna$invoke$v1";
const ENCODE: &str = "zryna$encodeInvocation$v1";
/// Ordinary private invoker bound to its original NEW source execution and exact scalar request.
#[derive(Debug)]
pub struct ValidatedInvocationArtifact<'p, 'a> {
    source: &'p VerifiedExecution<'a>,
    invocation: VerifiedInvocation<'p>,
    text: String,
    entry: &'static str,
    encode: &'static str,
}
impl<'p, 'a> ValidatedInvocationArtifact<'p, 'a> {
    /// Returns the same NEW source authority used to emit this carrier.
    #[must_use]
    pub const fn source(&self) -> &'p VerifiedExecution<'a> {
        self.source
    }
    /// Returns the request independently verified against this exact source ABI.
    #[must_use]
    pub const fn invocation(&self) -> &VerifiedInvocation<'p> {
        &self.invocation
    }
    /// Returns the complete invocation-only module with no observer or injection exports.
    #[must_use]
    pub fn as_source(&self) -> &str {
        &self.text
    }
    /// Returns the sole invocation entry name; arguments are already bound internally.
    #[must_use]
    pub const fn entry_export(&self) -> &str {
        self.entry
    }
    /// Returns the private one-shot current-result encoder export name.
    #[must_use]
    pub const fn encode_export(&self) -> &str {
        self.encode
    }
    /// Audits actual complete text against this retained source and invocation.
    /// # Errors
    /// Rejects any code, trap identity, export, transport or bound-request drift.
    pub fn audit_source(&self, actual: &str) -> Result<(), Diagnostic> {
        if actual.len() > crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES {
            return Err(emission_budget());
        }
        let expected = build_checked(
            self.source,
            &self.invocation,
            crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES,
            actual.len(),
        )?;
        if actual != expected {
            return Err(internal());
        }
        Ok(())
    }
}
/// Emits a separately typed ordinary invoker from NEW source authority only.
/// ```compile_fail
/// fn raw(p: &zryna_ir::generic_v1::execution_v1::raw::Program, r: zryna_abi::Invocation) {
///     zryna_backend_javascript::generic_execution_v1::emit_invocation(p,r);
/// }
/// ```
/// ```compile_fail
/// fn old(p: &zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram<'_>, r: zryna_abi::Invocation) {
///     zryna_backend_javascript::generic_execution_v1::emit_invocation(p,r);
/// }
/// ```
/// # Errors
/// Rejects invalid same-source requests, unsupported rows and unchanged emission limits.
pub fn emit_invocation<'p, 'a>(
    source: &'p VerifiedExecution<'a>,
    request: Invocation,
) -> Result<ValidatedInvocationArtifact<'p, 'a>, Diagnostic> {
    let invocation = source.scalar_abi().prepare_invocation(request).map_err(|error| {
        Diagnostic::error(
            error.code(),
            None,
            "generic JavaScript invocation does not match this source ABI",
            "use the exact scalar export and typed arguments",
        )
    })?;
    let text = build(source, &invocation, crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)?;
    let artifact =
        ValidatedInvocationArtifact { source, invocation, text, entry: ENTRY, encode: ENCODE };
    artifact.audit_source(artifact.as_source())?;
    Ok(artifact)
}
fn build(
    source: &VerifiedExecution<'_>,
    invocation: &VerifiedInvocation<'_>,
    limit: usize,
) -> Result<String, Diagnostic> {
    build_checked(source, invocation, limit, 0)
}
fn build_checked(
    source: &VerifiedExecution<'_>,
    invocation: &VerifiedInvocation<'_>,
    limit: usize,
    comparison_bytes: usize,
) -> Result<String, Diagnostic> {
    let mut context = Context::new(source)?;
    charge(&context.work, comparison_bytes)?;
    context.accounting = runtime::accounting::Plan::derive(&context)?;
    let quota = if context.accounting.is_some() {
        runtime::accounting::quota()?
    } else {
        runtime::ordinary::quota::prove(&context)?
    };
    charge(
        &context.work,
        invocation
            .arguments()
            .len()
            .checked_mul(128)
            .and_then(|n| n.checked_add(4096))
            .ok_or_else(emission_budget)?,
    )?;
    let mut counter = Writer::counter(limit);
    counter.invocation();
    render::module(&context, &mut counter, Mode::Ordinary(quota), invocation)?;
    charge(&context.work, counter.bytes())?;
    let mut output = Writer::output(limit, counter.bytes())?;
    output.invocation();
    render::module(&context, &mut output, Mode::Ordinary(quota), invocation)?;
    if counter.bytes() != output.bytes() {
        return Err(internal());
    }
    Ok(output.finish())
}
#[cfg(test)]
mod tests;
