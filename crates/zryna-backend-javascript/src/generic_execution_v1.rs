//! Additive JavaScript emission from the independently source-bound execution seal.
use crate::JavaScriptArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
#[cfg(test)]
mod container_tests;
mod context;
mod invocation;
pub use invocation::{ValidatedInvocationArtifact, emit_invocation};
mod flow;
#[cfg(test)]
mod loan_tests;
mod operations;
#[cfg(test)]
mod ordinary_tests;
#[cfg(test)]
mod push_upgrade_tests;
mod runtime;
mod shapes;
mod storage;
#[cfg(test)]
mod tests;
mod writer;
use context::Context;
use writer::Writer;

/// Emits only the new independently source/type/state/cleanup/runtime-bound execution seal.
///
/// ```compile_fail
/// fn raw(program: &zryna_ir::generic_v1::execution_v1::raw::Program) {
///     zryna_backend_javascript::generic_execution_v1::emit(program);
/// }
/// ```
/// ```compile_fail
/// fn old(program: &zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram<'_>) {
///     zryna_backend_javascript::generic_execution_v1::emit(program);
/// }
/// ```
/// # Errors
/// Rejects resource exhaustion and operations outside the implemented execution checkpoint.
pub fn emit(program: &VerifiedExecution<'_>) -> Result<JavaScriptArtifact, Diagnostic> {
    emit_with_budget(program, crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
}

fn emit_with_budget(
    program: &VerifiedExecution<'_>,
    limit: usize,
) -> Result<JavaScriptArtifact, Diagnostic> {
    let mut context = Context::new(program)?;
    context.accounting = runtime::accounting::Plan::derive(&context)?;
    let quota = if context.accounting.is_some() {
        runtime::accounting::quota()?
    } else {
        runtime::ordinary::quota::prove(&context)?
    };
    let mode = Mode::Ordinary(quota);
    emit_context(&context, limit, mode)
}
#[derive(Clone, Copy)]
enum Mode {
    Ordinary(runtime::ordinary::quota::Quota),
    Observed,
}
/// Explicit private-injection observation artifact; never ordinary driver authority.
#[derive(Debug, Eq, PartialEq)]
pub struct ObservedExecutionArtifact {
    source: String,
}
impl ObservedExecutionArtifact {
    /// Complete test-only module with its explicit private runtime injection binding.
    #[must_use]
    pub fn as_source(&self) -> &str {
        &self.source
    }
}
/// Emits only the separately identified private observation checkpoint.
/// # Errors
/// Rejects unsupported operations and unchanged compiler resource limits.
pub fn emit_observed(
    program: &VerifiedExecution<'_>,
) -> Result<ObservedExecutionArtifact, Diagnostic> {
    let context = Context::new(program)?;
    let artifact =
        emit_context(&context, crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES, Mode::Observed)?;
    Ok(ObservedExecutionArtifact { source: artifact.source })
}
fn emit_context(
    context: &Context<'_, '_>,
    limit: usize,
    mode: Mode,
) -> Result<JavaScriptArtifact, Diagnostic> {
    let mut counter = Writer::counter(limit);
    render(context, &mut counter, mode)?;
    let mut output = Writer::output(limit, counter.bytes())?;
    render(context, &mut output, mode)?;
    debug_assert_eq!(counter.bytes(), output.bytes());
    let source = output.finish();
    if matches!(mode, Mode::Ordinary(_)) {
        runtime::ordinary::audit(context, &source, mode)?;
    }
    Ok(JavaScriptArtifact { source })
}

fn render(context: &Context<'_, '_>, writer: &mut Writer, mode: Mode) -> Result<(), Diagnostic> {
    writer.text(crate::prelude::JAVASCRIPT_PRELUDE)?;
    runtime::declarations(context, writer, mode)?;
    shapes::declarations(context, writer)?;
    for function in &context.functions {
        flow::function(context, function, writer)?;
    }
    flow::exports(context, writer, matches!(mode, Mode::Ordinary(_)))
}

fn internal() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-I7001",
        None,
        "execution JavaScript checkpoint invariant failed",
        "retain the exact new execution seal and implement the remaining operation before emission",
    )
}
fn emission_budget() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-J2003",
        None,
        "execution JavaScript emission resource budget exhausted",
        "reduce the complete sealed program below the existing JavaScript emission budget",
    )
}

#[cfg(test)]
mod while_tests;

#[cfg(test)]
mod erased_array_tests;

#[cfg(test)]
mod borrowed_match_tests;

#[cfg(test)]
mod accounting_tests;

#[cfg(test)]
mod vec_accounting_tests;

#[cfg(test)]
mod indexed_tests;

#[cfg(test)]
mod control_accounting_tests;

#[cfg(test)]
mod aggregate_accounting_tests;
