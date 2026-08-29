//! Verified target-neutral Zryna intermediate representation.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use zryna_diagnostics::Diagnostic;
use zryna_source::{SourceMap, Span};

/// Exact scalar types in the first Zryna language slice.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Type {
    /// No value.
    Unit,
    /// Boolean value.
    Bool,
    /// Signed 32-bit integer.
    I32,
}

/// Expression identifier within one function.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExprId(pub u32);

/// Typed expression.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Expr {
    /// Exact result type.
    pub ty: Type,
    /// Source range.
    pub span: Span,
    /// Operation with fully selected semantics.
    pub kind: ExprKind,
}

/// Exact target-neutral expression operations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExprKind {
    /// Function parameter by zero-based index.
    Parameter(u32),
    /// Signed wrapping 32-bit integer addition.
    I32Add {
        /// Left operand.
        lhs: ExprId,
        /// Right operand.
        rhs: ExprId,
    },
    /// Signed 32-bit integer literal.
    I32Literal(i32),
}

/// Typed function.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    /// Exported function name.
    pub name: String,
    /// Parameter types.
    pub parameters: Vec<Type>,
    /// Return type.
    pub return_type: Type,
    /// Function expression arena.
    pub expressions: Vec<Expr>,
    /// Returned root expression.
    pub body: ExprId,
}

/// Zryna compilation unit before verification.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    /// Functions in deterministic source order.
    pub functions: Vec<Function>,
}

/// Program proven to satisfy backend invariants.
#[derive(Clone, Debug)]
pub struct VerifiedProgram(Program);

impl VerifiedProgram {
    /// Borrows the verified program.
    #[must_use]
    pub const fn as_program(&self) -> &Program {
        &self.0
    }
}

/// Verifies all invariants required by every backend.
///
/// # Errors
///
/// Returns every detected IR invariant violation in deterministic phase order.
pub fn verify(program: Program, sources: &SourceMap) -> Result<VerifiedProgram, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    for function in &program.functions {
        verify_function(function, sources, &mut diagnostics);
    }
    if diagnostics.is_empty() { Ok(VerifiedProgram(program)) } else { Err(diagnostics) }
}

fn verify_function(function: &Function, sources: &SourceMap, diagnostics: &mut Vec<Diagnostic>) {
    let valid_spans = function
        .expressions
        .iter()
        .map(|expression| match sources.resolve(expression.span) {
            Ok(_) => true,
            Err(error) => {
                diagnostics.push(Diagnostic::from_source_error(&error));
                false
            }
        })
        .collect::<Vec<_>>();
    let Some(body) = expression(function, function.body) else {
        diagnostics.push(Diagnostic::error(
            "ZRYNA-I1001",
            None,
            format!("function '{}' has an invalid body expression", function.name),
            "produce a body expression that belongs to the same function arena",
        ));
        return;
    };
    let body_index = usize::try_from(function.body.0).expect("body index was already resolved");
    if valid_spans[body_index] && body.ty != function.return_type {
        diagnostics.push(Diagnostic::error_at(
            "ZRYNA-I1002",
            body.span,
            format!("function '{}' returns the wrong IR type", function.name),
            "make the body expression type equal the declared return type",
        ));
    }
    for (expr, valid_span) in function.expressions.iter().zip(valid_spans) {
        if !valid_span {
            continue;
        }
        match &expr.kind {
            ExprKind::Parameter(index) => {
                let parameter =
                    usize::try_from(*index).ok().and_then(|value| function.parameters.get(value));
                if parameter != Some(&expr.ty) {
                    diagnostics.push(Diagnostic::error_at(
                        "ZRYNA-I1003",
                        expr.span,
                        format!(
                            "function '{}' contains an invalid parameter expression",
                            function.name
                        ),
                        "ensure the parameter index exists and its IR type matches",
                    ));
                }
            }
            ExprKind::I32Add { lhs, rhs } => {
                let valid = [*lhs, *rhs].into_iter().all(|id| {
                    expression(function, id).is_some_and(|operand| operand.ty == Type::I32)
                });
                if expr.ty != Type::I32 || !valid {
                    diagnostics.push(Diagnostic::error_at(
                        "ZRYNA-I1004",
                        expr.span,
                        format!("function '{}' contains an invalid i32 addition", function.name),
                        "i32 addition requires two valid i32 operands and an i32 result",
                    ));
                }
            }
            ExprKind::I32Literal(_) => {
                if expr.ty != Type::I32 {
                    diagnostics.push(Diagnostic::error_at(
                        "ZRYNA-I1005",
                        expr.span,
                        format!("function '{}' contains a mistyped i32 literal", function.name),
                        "assign Type::I32 to every i32 literal",
                    ));
                }
            }
        }
    }
}

fn expression(function: &Function, id: ExprId) -> Option<&Expr> {
    usize::try_from(id.0).ok().and_then(|index| function.expressions.get(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zryna_diagnostics::render_text;
    use zryna_source::{NormalizedSourcePath, SourceFileInput};

    fn sources() -> SourceMap {
        SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "1".to_owned(),
        }])
        .expect("fixture source map must be valid")
    }

    fn program(sources: &SourceMap) -> Program {
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        Program {
            functions: vec![Function {
                name: "value".to_owned(),
                parameters: Vec::new(),
                return_type: Type::I32,
                expressions: vec![Expr {
                    ty: Type::I32,
                    span: sources.span(file, 0, 1).expect("fixture span must be valid"),
                    kind: ExprKind::I32Literal(1),
                }],
                body: ExprId(0),
            }],
        }
    }

    #[test]
    fn verified_program_rejects_a_span_from_another_source_map() {
        let first = sources();
        let second = sources();
        assert!(verify(program(&first), &first).is_ok());
        let diagnostics = verify(program(&first), &second)
            .expect_err("cross-map spans must not enter VerifiedProgram");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-S1006"));
    }

    #[test]
    fn invalid_body_span_is_rejected_before_type_diagnostics() {
        let first = sources();
        let second = sources();
        let mut invalid = program(&first);
        invalid.functions[0].return_type = Type::Bool;

        let diagnostics =
            verify(invalid, &second).expect_err("wrong-map body span must reject the program");
        assert_eq!(
            diagnostics.iter().map(Diagnostic::code).collect::<Vec<_>>(),
            vec!["ZRYNA-S1006"]
        );
        assert!(render_text(&diagnostics, &second).is_ok());
    }
}
