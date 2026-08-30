//! Native code-generation boundary.

#![forbid(unsafe_code)]

use std::fmt::Write;

use zryna_diagnostics::Diagnostic;
use zryna_native_mir::{
    MirType, OperationView, ValueId, VerifiedCallingConvention, VerifiedMirFunction,
    VerifiedMirModule,
};

/// Textual LLVM IR artifact used to validate the initial native boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlvmIrArtifact {
    /// LLVM IR module text.
    pub source: String,
}

/// Emits the supported verified MIR slice as LLVM IR text.
///
/// Object emission is intentionally a later backend implementation.
/// Raw MIR is not accepted by this boundary:
///
/// ```compile_fail
/// let raw = zryna_native_mir::raw::Module::new(Vec::new());
/// let _ = zryna_backend_native::emit_llvm_ir(&raw);
/// ```
///
/// # Errors
///
/// Returns a compiler diagnostic when an internal verified-MIR invariant or formatting fails.
pub fn emit_llvm_ir(module: &VerifiedMirModule) -> Result<LlvmIrArtifact, Diagnostic> {
    let mut output = String::new();
    for function in module.functions() {
        emit_function(function, &mut output)?;
    }
    Ok(LlvmIrArtifact { source: output })
}

fn emit_function(function: VerifiedMirFunction<'_>, output: &mut String) -> Result<(), Diagnostic> {
    match function.calling_convention() {
        VerifiedCallingConvention::ZrynaInternalI32V1 => {}
    }
    verify_codegen_type(function.result_type())?;
    for ty in function.parameter_types() {
        verify_codegen_type(*ty)?;
    }
    write!(output, "define i32 @{}(", function.symbol()).map_err(native_format_error)?;
    for index in 0..function.parameter_types().len() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "i32 %p{index}").map_err(native_format_error)?;
    }
    output.push_str(") {\nentry:\n");
    for value in function.values() {
        verify_codegen_type(value.ty())?;
        let id = value.id().index();
        match value.operation() {
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

fn llvm_value(function: VerifiedMirFunction<'_>, id: ValueId) -> Result<String, Diagnostic> {
    let value = function.value(id).ok_or_else(|| {
        Diagnostic::error(
            "ZRYNA-N2002",
            None,
            format!("verified native function '{}' references a missing value", function.symbol()),
            "report this compiler invariant failure with the smallest reproducible source",
        )
    })?;
    match value.operation() {
        OperationView::Parameter { index } => Ok(format!("%p{index}")),
        OperationView::I32Literal { .. } | OperationView::I32Add { .. } => {
            Ok(format!("%v{}", id.index()))
        }
    }
}

fn verify_codegen_type(ty: MirType) -> Result<(), Diagnostic> {
    match ty {
        MirType::I32 => Ok(()),
        MirType::Unit | MirType::Bool => Err(Diagnostic::error(
            "ZRYNA-N2001",
            None,
            "verified native MIR contains a type outside the LLVM proof profile",
            "report this compiler invariant failure with the smallest reproducible source",
        )),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_entry_accepts_only_verified_mir() {
        let _: fn(&VerifiedMirModule) -> Result<LlvmIrArtifact, Diagnostic> = emit_llvm_ir;
        let verified = zryna_native_mir::verify(zryna_native_mir::raw::Module::new(Vec::new()))
            .expect("empty raw MIR must verify");
        assert_eq!(emit_llvm_ir(&verified).expect("empty module must emit").source, "");
    }
}
