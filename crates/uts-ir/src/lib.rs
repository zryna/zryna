//! Verified target-neutral UTS intermediate representation.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use uts_diagnostics::Diagnostic;
use uts_source::Span;

/// Exact scalar types in the first UTS language slice.
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

/// UTS compilation unit before verification.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
pub fn verify(program: Program) -> Result<VerifiedProgram, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    for function in &program.functions {
        verify_function(function, &mut diagnostics);
    }
    if diagnostics.is_empty() { Ok(VerifiedProgram(program)) } else { Err(diagnostics) }
}

fn verify_function(function: &Function, diagnostics: &mut Vec<Diagnostic>) {
    let Some(body) = expression(function, function.body) else {
        diagnostics.push(Diagnostic::error(
            "UTS-I1001",
            None,
            format!("function '{}' has an invalid body expression", function.name),
            "produce a body expression that belongs to the same function arena",
        ));
        return;
    };
    if body.ty != function.return_type {
        diagnostics.push(Diagnostic::error(
            "UTS-I1002",
            None,
            format!("function '{}' returns the wrong IR type", function.name),
            "make the body expression type equal the declared return type",
        ));
    }
    for expr in &function.expressions {
        match &expr.kind {
            ExprKind::Parameter(index) => {
                let parameter =
                    usize::try_from(*index).ok().and_then(|value| function.parameters.get(value));
                if parameter != Some(&expr.ty) {
                    diagnostics.push(Diagnostic::error(
                        "UTS-I1003",
                        None,
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
                    diagnostics.push(Diagnostic::error(
                        "UTS-I1004",
                        None,
                        format!("function '{}' contains an invalid i32 addition", function.name),
                        "i32 addition requires two valid i32 operands and an i32 result",
                    ));
                }
            }
            ExprKind::I32Literal(_) => {
                if expr.ty != Type::I32 {
                    diagnostics.push(Diagnostic::error(
                        "UTS-I1005",
                        None,
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
