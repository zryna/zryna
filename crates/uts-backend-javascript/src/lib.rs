//! Direct JavaScript emission from verified UTS IR.

#![forbid(unsafe_code)]

use std::fmt::Write;

use uts_diagnostics::Diagnostic;
use uts_ir::{Expr, ExprId, ExprKind, Function, Type, VerifiedProgram};

/// JavaScript artifacts produced by one compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaScriptArtifact {
    /// ECMAScript module source.
    pub source: String,
}

/// Emits modern ECMAScript from verified UTS IR.
///
/// # Errors
///
/// Returns a compiler diagnostic when a verified expression cannot be emitted.
pub fn emit(program: &VerifiedProgram) -> Result<JavaScriptArtifact, Diagnostic> {
    let mut output = String::new();
    for function in &program.as_program().functions {
        emit_function(function, &mut output)?;
    }
    Ok(JavaScriptArtifact { source: output })
}

fn emit_function(function: &Function, output: &mut String) -> Result<(), Diagnostic> {
    write!(output, "export function {}(", function.name)
        .map_err(|error| formatting_error(function, error))?;
    for index in 0..function.parameters.len() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "p{index}").map_err(|error| formatting_error(function, error))?;
    }
    output.push_str(") {\n  return ");
    emit_expression(function, function.body, output)?;
    output.push_str(";\n}\n");
    Ok(())
}

fn emit_expression(function: &Function, id: ExprId, output: &mut String) -> Result<(), Diagnostic> {
    let expression = get_expression(function, id)?;
    match &expression.kind {
        ExprKind::Parameter(index) => {
            write!(output, "p{index}").map_err(|error| formatting_error(function, error))?;
        }
        ExprKind::I32Add { lhs, rhs } => {
            output.push('(');
            emit_expression(function, *lhs, output)?;
            output.push_str(" + ");
            emit_expression(function, *rhs, output)?;
            output.push_str(") | 0");
        }
        ExprKind::I32Literal(value) => {
            write!(output, "{value}").map_err(|error| formatting_error(function, error))?;
        }
    }
    Ok(())
}

fn get_expression(function: &Function, id: ExprId) -> Result<&Expr, Diagnostic> {
    usize::try_from(id.0).ok().and_then(|index| function.expressions.get(index)).ok_or_else(|| {
        Diagnostic::error(
            "UTS-J1001",
            None,
            format!("function '{}' references missing verified IR", function.name),
            "run the mandatory IR verifier before JavaScript emission",
        )
    })
}

fn formatting_error(function: &Function, error: std::fmt::Error) -> Diagnostic {
    Diagnostic::error(
        "UTS-J1002",
        None,
        format!("could not emit JavaScript for '{}': {error}", function.name),
        "report this compiler failure with the smallest reproducible UTS source",
    )
}

/// Returns the JavaScript representation used by the first language slice.
#[must_use]
pub const fn javascript_type(ty: Type) -> &'static str {
    match ty {
        Type::Unit => "undefined",
        Type::Bool => "boolean",
        Type::I32 => "number",
    }
}

#[cfg(test)]
mod tests {
    use uts_ir::{Expr, ExprId, ExprKind, Function, Program, Type, verify};
    use uts_source::{FileId, Span};

    #[test]
    fn emits_wrapping_i32_addition() {
        let span = Span { file: FileId(0), start: 0, end: 1 };
        let program = Program {
            functions: vec![Function {
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
            }],
        };
        let Ok(verified) = verify(program) else {
            panic!("test IR must be valid");
        };
        let Ok(artifact) = super::emit(&verified) else {
            panic!("JavaScript emission must succeed");
        };
        assert_eq!(artifact.source, "export function add(p0, p1) {\n  return (p0 + p1) | 0;\n}\n");
    }
}
