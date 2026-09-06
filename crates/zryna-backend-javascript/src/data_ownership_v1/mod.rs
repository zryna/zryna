use std::fmt::Write;

use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{VerifiedFunction, VerifiedProgram};
use zryna_ownership_runtime_abi::VerifiedOwnershipRuntimeAbi;

use crate::JavaScriptArtifact;

mod instructions;
mod runtime;

const MAX_BYTES: usize = 32 * 1024 * 1024;

/// Emits bounded deterministic ECMAScript for sealed `DataOwnershipV1` authority.
///
/// The ownership ABI must be bound to the program's exact type universe and both layouts.
/// No raw IR, ambient package, browser capability, or engine-GC timing enters this boundary.
///
/// # Errors
/// Returns a stable diagnostic for mismatched authority or bounded formatting failure.
pub fn emit_data_ownership(
    program: &VerifiedProgram,
    runtime_abi: &VerifiedOwnershipRuntimeAbi,
) -> Result<JavaScriptArtifact, Diagnostic> {
    validate_authority(program, runtime_abi)?;
    let mut count = Counter::default();
    render(program, &mut count)?;
    if count.0 > MAX_BYTES {
        return Err(error(
            "ZRYNA-J3003",
            format!("DataOwnershipV1 JavaScript exceeds {MAX_BYTES} bytes"),
        ));
    }
    let mut source = String::new();
    source.try_reserve(count.0).map_err(|_| {
        error("ZRYNA-J3003", "could not reserve bounded DataOwnershipV1 JavaScript")
    })?;
    render(program, &mut source)?;
    debug_assert_eq!(source.len(), count.0);
    Ok(JavaScriptArtifact { source })
}

fn validate_authority(
    program: &VerifiedProgram,
    runtime_abi: &VerifiedOwnershipRuntimeAbi,
) -> Result<(), Diagnostic> {
    if program.type_universe_identity() != runtime_abi.type_universe_identity()
        || program.linear32_layouts().fingerprint() != &runtime_abi.linear32_fingerprint()
        || program.linux_x86_64_layouts().fingerprint() != &runtime_abi.linux_x86_64_fingerprint()
    {
        return Err(error(
            "ZRYNA-J3001",
            "ownership runtime ABI is not bound to the verified program",
        ));
    }
    if runtime_abi.javascript_helpers().len() != runtime_abi.operations().len() {
        return Err(error(
            "ZRYNA-J3001",
            "ownership runtime ABI has an incomplete JavaScript helper map",
        ));
    }
    Ok(())
}

fn render(program: &VerifiedProgram, out: &mut impl Write) -> Result<(), Diagnostic> {
    out.write_str(super::JAVASCRIPT_PRELUDE).map_err(format_error)?;
    out.write_str(runtime::PRELUDE).map_err(format_error)?;
    let mut exports = Vec::new();
    for module in program.modules() {
        for function in module.functions() {
            instructions::emit_function(function, program.linear32_layouts(), out)?;
            if let Some(export) = function.public_export() {
                exports.push((function, export.javascript_name().as_str().to_owned()));
            }
        }
    }
    for (index, (function, name)) in exports.into_iter().enumerate() {
        instructions::emit_wrapper(function, index, &name, program.linear32_layouts(), out)?;
    }
    if program
        .modules()
        .all(|module| module.functions().all(|function| function.public_export().is_none()))
    {
        out.write_str("export {};\n").map_err(format_error)?;
    }
    Ok(())
}

#[derive(Default)]
struct Counter(usize);
impl Write for Counter {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.0 = self.0.checked_add(value.len()).ok_or(std::fmt::Error)?;
        Ok(())
    }
}

fn private_name(function: VerifiedFunction<'_>) -> String {
    let id = function.id();
    format!("$zryna$d{}f{}", id.module(), id.declaration())
}

fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(code, None, message, "report the smallest reproducible verified M3 program")
}

fn format_error(_: std::fmt::Error) -> Diagnostic {
    error("ZRYNA-J3002", "could not format deterministic DataOwnershipV1 JavaScript")
}
