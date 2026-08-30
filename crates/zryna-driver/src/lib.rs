//! Zryna compiler-phase orchestration.

#![forbid(unsafe_code)]

use std::path::Path;

use zryna_architecture::ValidationReport;
use zryna_backend_javascript::JavaScriptArtifact;
use zryna_backend_native::LlvmIrArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_ir::VerifiedProgram;
use zryna_source::SourceMap;

/// Artifacts emitted by the first verified dual-target slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DualTargetArtifacts {
    /// Direct ECMAScript output.
    pub javascript: JavaScriptArtifact,
    /// Textual LLVM IR validating the native backend boundary.
    pub llvm_ir: LlvmIrArtifact,
}

/// Runs the mandatory workspace gate.
#[must_use]
pub fn check_workspace(root: &Path) -> ValidationReport {
    zryna_architecture::validate_workspace(root)
}

/// Runs one authenticated frontend worker and returns only source-map-verified syntax.
///
/// # Errors
///
/// Returns a deterministic worker failure before untrusted syntax can enter later phases.
pub fn analyze_sources<Provider: zryna_frontend::VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
) -> Result<zryna_frontend::syntax_v2::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
    frontend.analyze_verified(sources)
}

/// Emits both current backend artifacts from one verified program.
///
/// # Errors
///
/// Returns bounded diagnostics when native MIR verification or either target emission fails.
pub fn emit_verified(program: &VerifiedProgram) -> Result<DualTargetArtifacts, Vec<Diagnostic>> {
    let javascript = zryna_backend_javascript::emit(program).map_err(|error| vec![error])?;
    let mir = zryna_native_mir::lower(program)?;
    let llvm_ir = zryna_backend_native::emit_llvm_ir(&mir).map_err(|error| vec![error])?;
    Ok(DualTargetArtifacts { javascript, llvm_ir })
}

#[cfg(test)]
mod tests {
    use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type, verify};
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};

    #[test]
    fn one_verified_program_drives_both_backend_boundaries() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/add.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/add.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
        let program = Program {
            functions: vec![
                Function {
                    name: "add".to_owned(),
                    parameters: vec![Type::I32, Type::I32],
                    return_type: Type::I32,
                    expressions: vec![
                        Expr { ty: Type::I32, span, kind: ExprKind::Parameter(0) },
                        Expr { ty: Type::I32, span, kind: ExprKind::Parameter(1) },
                        Expr {
                            ty: Type::I32,
                            span,
                            kind: ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) },
                        },
                    ],
                    body: ExprId(2),
                },
                Function {
                    name: "answer".to_owned(),
                    parameters: Vec::new(),
                    return_type: Type::I32,
                    expressions: vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(42) }],
                    body: ExprId(0),
                },
            ],
        };
        let verified = verify(program, &sources).expect("fixture IR must verify");

        let artifacts = super::emit_verified(&verified).expect("both backends must emit");

        assert_eq!(
            artifacts.javascript.source,
            "export function add(p0, p1) {\n  const v0 = p0;\n  const v1 = p1;\n  const v2 = (v0 + v1) | 0;\n  return v2;\n}\nexport function answer() {\n  const v0 = 42;\n  return v0;\n}\n"
        );
        assert_eq!(
            artifacts.llvm_ir.source,
            "define i32 @add(i32 %p0, i32 %p1) {\nentry:\n  %v2 = add i32 %p0, %p1\n  ret i32 %v2\n}\ndefine i32 @answer() {\nentry:\n  %v0 = add i32 0, 42\n  ret i32 %v0\n}\n"
        );
    }
}
