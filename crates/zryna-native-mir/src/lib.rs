//! Native machine-independent representation.

#![forbid(unsafe_code)]

use zryna_diagnostics::Diagnostic;
use zryna_ir::{ExprKind, VerifiedFunction, VerifiedProgram};

/// Native value reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueId(u32);

impl ValueId {
    /// Returns the dense value index assigned during verified lowering.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Opaque native MIR operation constructed only by verified lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation(OperationKind);

#[derive(Clone, Debug, Eq, PartialEq)]
enum OperationKind {
    Parameter { index: u32 },
    I32Literal { value: i32 },
    I32Add { lhs: ValueId, rhs: ValueId },
}

/// Read-only view of one sealed native MIR operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationView {
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

impl Operation {
    /// Returns a copyable read-only view for code generation.
    #[must_use]
    pub const fn view(&self) -> OperationView {
        match self.0 {
            OperationKind::Parameter { index } => OperationView::Parameter { index },
            OperationKind::I32Literal { value } => OperationView::I32Literal { value },
            OperationKind::I32Add { lhs, rhs } => OperationView::I32Add { lhs, rhs },
        }
    }
}

/// Native MIR function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirFunction {
    name: String,
    parameter_count: u32,
    operations: Vec<Operation>,
    result: ValueId,
}

impl MirFunction {
    /// Returns the sealed backend-safe native symbol input.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of `i32` parameters in the current native slice.
    #[must_use]
    pub const fn parameter_count(&self) -> u32 {
        self.parameter_count
    }

    /// Returns the deterministic lowered operation sequence.
    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// Returns the verified function result value.
    #[must_use]
    pub const fn result(&self) -> ValueId {
        self.result
    }
}

/// Native MIR module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirModule {
    functions: Vec<MirFunction>,
}

impl MirModule {
    /// Returns the lowered functions in verified program order.
    #[must_use]
    pub fn functions(&self) -> &[MirFunction] {
        &self.functions
    }
}

/// Lowers verified target-neutral IR to native MIR.
///
/// # Errors
///
/// Returns a diagnostic when the program cannot fit the current native ABI limits.
pub fn lower(program: &VerifiedProgram) -> Result<MirModule, Diagnostic> {
    let mut functions = Vec::new();
    for function in program.functions() {
        functions.push(lower_function(function)?);
    }
    Ok(MirModule { functions })
}

fn lower_function(function: VerifiedFunction<'_>) -> Result<MirFunction, Diagnostic> {
    let mut operations = Vec::with_capacity(function.expressions().len());
    for expression in function.expressions() {
        let operation = match &expression.kind {
            ExprKind::Parameter(index) => Operation(OperationKind::Parameter { index: *index }),
            ExprKind::I32Literal(value) => Operation(OperationKind::I32Literal { value: *value }),
            ExprKind::I32Add { lhs, rhs } => {
                Operation(OperationKind::I32Add { lhs: ValueId(lhs.0), rhs: ValueId(rhs.0) })
            }
        };
        operations.push(operation);
    }
    let parameter_count = u32::try_from(function.parameters().len()).map_err(|error| {
        Diagnostic::error(
            "ZRYNA-N1001",
            None,
            format!(
                "function '{}' has too many native parameters: {error}",
                function.export_name().as_str()
            ),
            "reduce the parameter count or wait for an expanded native ABI",
        )
    })?;
    Ok(MirFunction {
        name: function.export_name().as_str().to_owned(),
        parameter_count,
        operations,
        result: ValueId(function.body().0),
    })
}
