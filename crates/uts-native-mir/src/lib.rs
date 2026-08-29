//! Native machine-independent representation.

#![forbid(unsafe_code)]

use uts_diagnostics::Diagnostic;
use uts_ir::{ExprId, ExprKind, Function, VerifiedProgram};

/// Native value reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueId(pub u32);

/// First native MIR operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    /// Read a function argument.
    Parameter {
        /// Zero-based parameter index.
        index: u32,
    },
    /// Create a signed 32-bit literal.
    I32Literal {
        /// Literal value.
        value: i32,
    },
    /// Add two signed 32-bit values with wrapping semantics.
    I32Add {
        /// Left operand.
        lhs: ValueId,
        /// Right operand.
        rhs: ValueId,
    },
}

/// Native MIR function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirFunction {
    /// Function name.
    pub name: String,
    /// Number of i32 parameters in the first language slice.
    pub parameter_count: u32,
    /// Deterministic operation list.
    pub operations: Vec<Operation>,
    /// Returned value.
    pub result: ValueId,
}

/// Native MIR module.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MirModule {
    /// Functions in source order.
    pub functions: Vec<MirFunction>,
}

/// Lowers verified target-neutral IR to native MIR.
///
/// # Errors
///
/// Returns a diagnostic when the program cannot fit the current native ABI limits.
pub fn lower(program: &VerifiedProgram) -> Result<MirModule, Diagnostic> {
    let mut functions = Vec::new();
    for function in &program.as_program().functions {
        functions.push(lower_function(function)?);
    }
    Ok(MirModule { functions })
}

fn lower_function(function: &Function) -> Result<MirFunction, Diagnostic> {
    let mut operations = Vec::with_capacity(function.expressions.len());
    for expression in &function.expressions {
        let operation = match &expression.kind {
            ExprKind::Parameter(index) => Operation::Parameter { index: *index },
            ExprKind::I32Literal(value) => Operation::I32Literal { value: *value },
            ExprKind::I32Add { lhs, rhs } => {
                Operation::I32Add { lhs: ValueId(lhs.0), rhs: ValueId(rhs.0) }
            }
        };
        operations.push(operation);
    }
    let parameter_count = u32::try_from(function.parameters.len()).map_err(|error| {
        Diagnostic::error(
            "UTS-N1001",
            None,
            format!("function '{}' has too many native parameters: {error}", function.name),
            "reduce the parameter count or wait for an expanded native ABI",
        )
    })?;
    Ok(MirFunction {
        name: function.name.clone(),
        parameter_count,
        operations,
        result: ValueId(function.body.0),
    })
}

/// Converts an IR expression identifier into its native value identifier.
#[must_use]
pub const fn value_id(id: ExprId) -> ValueId {
    ValueId(id.0)
}
