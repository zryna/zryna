//! Native physical frames issued only from the independent full execution seal.
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, VerifiedFunction, raw};
/// Physical category of the retained same-seal layout view.
pub use zryna_layout::TypeCategory;
use zryna_layout::{StorageTarget, generic_v1::TypeView};
mod containers;
mod planning;
#[cfg(test)]
mod resource_tests;
mod shapes;
pub use containers::ContainerPlan;

pub use crate::generic_owned_v2::{
    MAX_CODEGEN_UNITS, MAX_PARAMETERS, MAX_TYPE_LANES, MAX_VALUE_LANES,
};

/// Same-program physical representation. Raw claims and older seals cannot construct it.
/// ```compile_fail
/// fn fake(raw: zryna_ir::generic_v1::execution_v1::raw::Program) {
///     let _: zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_> = raw;
/// }
/// ```
pub struct VerifiedProgram<'p, 'a> {
    program: &'p VerifiedExecution<'a>,
    types: Vec<shapes::Shape<'a>>,
    functions: Vec<FunctionPlan<'p, 'a>>,
    containers: Option<ContainerPlan<'p, 'a>>,
    concat: bool,
}
/// A checked physical frame interval, meaningful only in the retaining function plan.
#[derive(Clone, Copy, Debug)]
pub struct Record {
    offset: u32,
    ty: raw::ValueType,
}
impl Record {
    /// Byte displacement within this invocation's own frame.
    #[must_use]
    pub const fn offset(self) -> u32 {
        self.offset
    }
    /// Independently checked logical type, never an older layout ID.
    #[must_use]
    pub const fn ty(self) -> raw::ValueType {
        self.ty
    }
}
/// Nonstored referent address in the private calling convention.
#[derive(Clone, Copy, Debug)]
pub enum LoanAddress {
    /// Caller-owned argument pointer plus checked static projection displacement.
    Formal {
        /// Source-order private parameter index.
        parameter: u32,
        /// Checked byte displacement within its exact referent.
        offset: u32,
    },
    /// Exact complete/projection storage in this invocation's frame.
    Frame(Record),
}
/// One exact function and its checked physical frame inventory.
pub struct FunctionPlan<'p, 'a> {
    source: VerifiedFunction<'p, 'a>,
    symbol: String,
    frame_bytes: u32,
    values: Vec<Record>,
    places: Vec<Record>,
    loans: Vec<LoanAddress>,
    loan_types: Vec<u32>,
}
impl<'p, 'a> FunctionPlan<'p, 'a> {
    /// Retained same-program source/type/state-verified function.
    #[must_use]
    pub const fn source(&self) -> VerifiedFunction<'p, 'a> {
        self.source
    }
    /// Compiler-private symbol in canonical function order.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
    /// Entire per-invocation physical frame, including static roots and ordinary values.
    #[must_use]
    pub const fn frame_bytes(&self) -> u32 {
        self.frame_bytes
    }
    /// Same-function ordinary value storage.
    #[must_use]
    pub fn value(&self, id: u32) -> Option<Record> {
        self.values.get(id as usize).copied()
    }
    /// Same-function checked static place projection.
    #[must_use]
    pub fn place(&self, id: u32) -> Option<Record> {
        self.places.get(id as usize).copied()
    }
    /// Exact referent type for a same-function loan.
    #[must_use]
    pub fn loan_type(&self, id: u32) -> Option<u32> {
        self.loan_types.get(id as usize).copied()
    }
    /// Same-function checked nonstored loan origin.
    #[must_use]
    pub fn loan(&self, id: u32) -> Option<LoanAddress> {
        self.loans.get(id as usize).copied()
    }
}
impl<'p, 'a> VerifiedProgram<'p, 'a> {
    /// The exact new execution owner, layouts, runtime and scalar ABI.
    #[must_use]
    pub const fn program(&self) -> &'p VerifiedExecution<'a> {
        self.program
    }
    /// Canonical function plans. No older MIR authority is installed.
    #[must_use]
    pub fn functions(&self) -> &[FunctionPlan<'p, 'a>] {
        &self.functions
    }
    /// Exact same-seal Linux record correspondence authenticated by the IR issuer.
    #[must_use]
    pub fn stored_type(&self, ty: u32) -> Option<TypeView<'a>> {
        self.types.get(ty as usize).map(|shape| shape.view)
    }
    /// All-variant static String acquisition sites in this checked finite physical shape.
    #[must_use]
    pub fn string_occurrences(&self, ty: u32) -> Option<u64> {
        self.types.get(ty as usize).map(|shape| shape.strings)
    }
    /// Checked finite container/helper authority, absent on the unchanged first-five path.
    #[must_use]
    pub const fn containers(&self) -> Option<&ContainerPlan<'p, 'a>> {
        self.containers.as_ref()
    }
    /// Whether this exact program requires the inherited String concatenation runtime operation.
    #[must_use]
    pub const fn uses_string_concat(&self) -> bool {
        self.concat
    }
    /// Physical byte count for a same-seal logical value type.
    #[must_use]
    pub fn size(&self, ty: raw::ValueType) -> Option<u64> {
        match ty {
            raw::ValueType::Unit => Some(0),
            raw::ValueType::Stored(id) => self.stored_type(id).map(TypeView::size),
        }
    }
}
/// Prepay complete physical projection, expansion, emission and cleanup before code generation.
/// # Errors
/// Returns inherited N7001 budgets or an unexpected sealed/unsupported invariant N7002.
pub fn lower<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
) -> Result<VerifiedProgram<'p, 'a>, Diagnostic> {
    let mut work = 0;
    lower_from(program, &mut work)
}
fn lower_from<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
    work: &mut u64,
) -> Result<VerifiedProgram<'p, 'a>, Diagnostic> {
    let layouts = program.layouts(StorageTarget::LinuxX8664V1);
    let types = shapes::derive(layouts, work)?;
    planning::take(work, (program.functions().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    let mut functions = planning::reserve(program.functions().len())?;
    let mut extensions = containers::Extensions::default();
    for f in program.functions() {
        let (function, required) = planning::function(f, &types, work)?;
        extensions.push |= required.push;
        extensions.upgrade |= required.upgrade;
        extensions.concat |= required.concat;
        functions.push(function);
    }
    let containers = containers::derive(program, &types, extensions, work)?;
    Ok(VerifiedProgram { program, types, functions, containers, concat: extensions.concat })
}
fn budget() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N7001",
        None,
        "full execution native representation exceeds its checked budget",
        "reduce physical type, parameter or code amplification",
    )
}
fn invariant() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N7002",
        None,
        "full execution native MIR rejected a sealed or unfinished invariant",
        "report the smallest reproducible source",
    )
}
