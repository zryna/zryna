//! Direct JavaScript emission from verified Zryna IR.

#![forbid(unsafe_code)]

use std::fmt::Write;

use zryna_diagnostics::Diagnostic;
use zryna_ir::{ExprKind, Type, VerifiedFunction, VerifiedProgram};

/// JavaScript artifacts produced by one compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaScriptArtifact {
    /// ECMAScript module source.
    pub source: String,
}

/// Emits modern ECMAScript from verified Zryna IR.
///
/// # Errors
///
/// Returns a compiler diagnostic when a verified expression cannot be emitted.
pub fn emit(program: &VerifiedProgram) -> Result<JavaScriptArtifact, Diagnostic> {
    let mut output = String::new();
    for function in program.functions() {
        emit_function(function, &mut output)?;
    }
    Ok(JavaScriptArtifact { source: output })
}

fn emit_function(function: VerifiedFunction<'_>, output: &mut String) -> Result<(), Diagnostic> {
    write!(output, "export function {}(", function.export_name().as_str())
        .map_err(|error| formatting_error(function, error))?;
    for index in 0..function.parameters().len() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "p{index}").map_err(|error| formatting_error(function, error))?;
    }
    output.push_str(") {\n");
    for (index, expression) in function.expressions().iter().enumerate() {
        write!(output, "  const v{index} = ").map_err(|error| formatting_error(function, error))?;
        match &expression.kind {
            ExprKind::Parameter(parameter) => {
                write!(output, "p{parameter}")
                    .map_err(|error| formatting_error(function, error))?;
            }
            ExprKind::I32Add { lhs, rhs } => {
                write!(output, "(v{} + v{}) | 0", lhs.0, rhs.0)
                    .map_err(|error| formatting_error(function, error))?;
            }
            ExprKind::I32Literal(value) => {
                write!(output, "{value}").map_err(|error| formatting_error(function, error))?;
            }
        }
        output.push_str(";\n");
    }
    write!(output, "  return v{};\n}}\n", function.body().0)
        .map_err(|error| formatting_error(function, error))?;
    Ok(())
}

fn formatting_error(function: VerifiedFunction<'_>, error: std::fmt::Error) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-J1002",
        None,
        format!("could not emit JavaScript for '{}': {error}", function.export_name().as_str()),
        "report this compiler failure with the smallest reproducible Zryna source",
    )
}

/// Returns the JavaScript spelling associated with an IR type.
///
/// This representation helper does not admit a type into the current universal profile.
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
    use zryna_ir::{
        Expr, ExprId, ExprKind, Function, MAX_IR_EXPRESSION_DEPTH, Program, Type, verify,
    };
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};

    #[test]
    fn emits_wrapping_i32_addition() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/add.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/add.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
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
        let Ok(verified) = verify(program, &sources) else {
            panic!("test IR must be valid");
        };
        let Ok(artifact) = super::emit(&verified) else {
            panic!("JavaScript emission must succeed");
        };
        assert_eq!(
            artifact.source,
            "export function add(p0, p1) {\n  const v0 = p0;\n  const v1 = p1;\n  const v2 = (v0 + v1) | 0;\n  return v2;\n}\n"
        );
    }

    #[test]
    fn emits_the_max_depth_tree_iteratively_and_linearly() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/deep.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/deep.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
        let mut expressions = vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(1) }];
        let mut root = ExprId(0);
        for _ in 1..MAX_IR_EXPRESSION_DEPTH {
            let leaf = ExprId(u32::try_from(expressions.len()).expect("bounded fixture"));
            expressions.push(Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(1) });
            let parent = ExprId(u32::try_from(expressions.len()).expect("bounded fixture"));
            expressions.push(Expr {
                ty: Type::I32,
                span,
                kind: ExprKind::I32Add { lhs: root, rhs: leaf },
            });
            root = parent;
        }
        let expression_count = expressions.len();
        let program = Program {
            functions: vec![Function {
                name: "deepValue".to_owned(),
                parameters: Vec::new(),
                return_type: Type::I32,
                expressions,
                body: root,
            }],
        };
        let verified = verify(program, &sources).expect("maximum-depth IR must verify");
        let artifact = super::emit(&verified).expect("maximum-depth JavaScript must emit");
        assert_eq!(artifact.source.matches("  const v").count(), expression_count);
        assert!(artifact.source.len() < expression_count * 64);
    }
}
