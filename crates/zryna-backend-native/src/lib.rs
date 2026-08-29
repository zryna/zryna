//! Native code-generation boundary.

#![forbid(unsafe_code)]

use std::fmt::Write;

use zryna_diagnostics::Diagnostic;
use zryna_native_mir::{MirFunction, MirModule, OperationView, ValueId};

/// Textual LLVM IR artifact used to validate the initial native boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlvmIrArtifact {
    /// LLVM IR module text.
    pub source: String,
}

/// Emits the supported scalar MIR slice as LLVM IR text.
///
/// Object emission is intentionally a later backend implementation.
///
/// # Errors
///
/// Returns a compiler diagnostic when MIR references are invalid or formatting fails.
pub fn emit_llvm_ir(module: &MirModule) -> Result<LlvmIrArtifact, Diagnostic> {
    let mut output = String::new();
    for function in module.functions() {
        emit_function(function, &mut output)?;
    }
    Ok(LlvmIrArtifact { source: output })
}

fn emit_function(function: &MirFunction, output: &mut String) -> Result<(), Diagnostic> {
    write!(output, "define i32 @{}(", function.name()).map_err(native_format_error)?;
    for index in 0..function.parameter_count() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "i32 %p{index}").map_err(native_format_error)?;
    }
    output.push_str(") {\nentry:\n");
    for (index, operation) in function.operations().iter().enumerate() {
        let id = u32::try_from(index).map_err(|error| {
            Diagnostic::error(
                "ZRYNA-N2001",
                None,
                format!("native MIR contains too many values: {error}"),
                "split the function before native lowering",
            )
        })?;
        match operation.view() {
            OperationView::Parameter { .. } => {}
            OperationView::I32Literal { value } => {
                writeln!(output, "  %v{id} = add i32 0, {value}").map_err(native_format_error)?;
            }
            OperationView::I32Add { lhs, rhs } => {
                let left = llvm_value(function, lhs)?;
                let right = llvm_value(function, rhs)?;
                writeln!(output, "  %v{id} = add i32 {left}, {right}")
                    .map_err(native_format_error)?;
            }
        }
    }
    let result = llvm_value(function, function.result())?;
    write!(output, "  ret i32 {result}\n}}\n").map_err(native_format_error)?;
    Ok(())
}

fn llvm_value(function: &MirFunction, id: ValueId) -> Result<String, Diagnostic> {
    let operation = usize::try_from(id.index())
        .ok()
        .and_then(|index| function.operations().get(index))
        .ok_or_else(|| {
            Diagnostic::error(
                "ZRYNA-N2002",
                None,
                format!("function '{}' references a missing MIR value", function.name()),
                "run native MIR validation before code generation",
            )
        })?;
    match operation.view() {
        OperationView::Parameter { index } => Ok(format!("%p{index}")),
        OperationView::I32Literal { .. } | OperationView::I32Add { .. } => {
            Ok(format!("%v{}", id.index()))
        }
    }
}

fn native_format_error(error: std::fmt::Error) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N2003",
        None,
        format!("native IR formatting failed: {error}"),
        "report this compiler failure with the smallest reproducible Zryna source",
    )
}
