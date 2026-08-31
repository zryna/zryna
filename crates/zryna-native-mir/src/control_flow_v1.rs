//! Verified native MIR for the M2 structured control-flow profile.
//!
//! This module is deliberately separate from the crate-root M1 straight-line MIR. Raw claims are
//! independently checked before opaque views can be observed; native code generation does not yet
//! consume this profile.

use std::collections::{BTreeMap, VecDeque};

use zryna_abi::{
    AbiViolationKind, VerifiedScalarAbiModule, VerifiedScalarExport, raw as raw_abi, verify_v1,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::Type;

/// Maximum modules accepted by native MIR control-flow v1.
pub const MAX_MODULES: usize = zryna_ir::control_flow_v1::MAX_MODULES;
/// Maximum functions accepted in one module.
pub const MAX_FUNCTIONS_PER_MODULE: usize = zryna_ir::control_flow_v1::MAX_FUNCTIONS_PER_MODULE;
/// Maximum functions accepted in one program.
pub const MAX_FUNCTIONS_PER_PROGRAM: usize = zryna_ir::control_flow_v1::MAX_FUNCTIONS_PER_PROGRAM;
/// Maximum parameters accepted in one function.
pub const MAX_PARAMETERS_PER_FUNCTION: usize =
    zryna_ir::control_flow_v1::MAX_PARAMETERS_PER_FUNCTION;
/// Maximum parameters accepted in one program.
pub const MAX_PARAMETERS_PER_PROGRAM: usize = zryna_ir::control_flow_v1::MAX_PARAMETERS_PER_PROGRAM;
/// Maximum blocks accepted in one function.
pub const MAX_BLOCKS_PER_FUNCTION: usize = zryna_ir::control_flow_v1::MAX_BLOCKS_PER_FUNCTION;
/// Maximum blocks accepted in one program.
pub const MAX_BLOCKS_PER_PROGRAM: usize = zryna_ir::control_flow_v1::MAX_BLOCKS_PER_PROGRAM;
/// Maximum parameters accepted by one block.
pub const MAX_BLOCK_PARAMETERS: usize = zryna_ir::control_flow_v1::MAX_BLOCK_PARAMETERS;
/// Maximum values accepted in one function.
pub const MAX_VALUES_PER_FUNCTION: usize = zryna_ir::control_flow_v1::MAX_VALUES_PER_FUNCTION;
/// Maximum values accepted in one program.
pub const MAX_VALUES_PER_PROGRAM: usize = zryna_ir::control_flow_v1::MAX_VALUES_PER_PROGRAM;
/// Maximum CFG edges accepted in one function.
pub const MAX_CFG_EDGES_PER_FUNCTION: usize = zryna_ir::control_flow_v1::MAX_CFG_EDGES_PER_FUNCTION;
/// Maximum CFG edges accepted in one program.
pub const MAX_CFG_EDGES_PER_PROGRAM: usize = zryna_ir::control_flow_v1::MAX_CFG_EDGES_PER_PROGRAM;
/// Maximum raw terminator claims accepted in one function.
pub const MAX_TERMINATORS_PER_FUNCTION: usize = MAX_BLOCKS_PER_FUNCTION;
/// Maximum raw terminator claims accepted in one program.
pub const MAX_TERMINATORS_PER_PROGRAM: usize = MAX_BLOCKS_PER_PROGRAM;
/// Maximum direct-call sites accepted in one program.
pub const MAX_CALL_EDGES: usize = zryna_ir::control_flow_v1::MAX_CALL_EDGES;
/// Maximum direct-call arguments accepted in aggregate.
pub const MAX_CALL_ARGUMENTS_PER_PROGRAM: usize = MAX_CALL_EDGES * MAX_PARAMETERS_PER_FUNCTION;
/// Maximum CFG edge arguments accepted in aggregate.
pub const MAX_EDGE_ARGUMENTS_PER_PROGRAM: usize = MAX_CFG_EDGES_PER_PROGRAM * MAX_BLOCK_PARAMETERS;
/// Maximum static direct-call depth.
pub const MAX_STATIC_CALL_DEPTH: usize = zryna_ir::control_flow_v1::MAX_STATIC_CALL_DEPTH;
/// Maximum natural-loop nesting.
pub const MAX_LOOP_NESTING: usize = zryna_ir::control_flow_v1::MAX_LOOP_NESTING;
/// Maximum bytes accepted in one internal body symbol.
pub const MAX_INTERNAL_SYMBOL_BYTES: usize = 128;
/// Maximum internal body symbol bytes accepted across one program.
pub const MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM: usize =
    MAX_FUNCTIONS_PER_PROGRAM * MAX_INTERNAL_SYMBOL_BYTES;
/// Maximum bytes accepted in one provisional public export name.
pub const MAX_ENTRY_EXPORT_BYTES: usize = zryna_abi::MAX_LOGICAL_EXPORT_NAME_BYTES;
/// Maximum provisional public export-name bytes accepted across one program.
pub const MAX_ENTRY_EXPORT_BYTES_PER_PROGRAM: usize =
    zryna_abi::MAX_ABI_EXPORTS * MAX_ENTRY_EXPORT_BYTES;
/// Maximum retained diagnostics including the terminal diagnostic.
pub const MAX_DIAGNOSTICS: usize = 256;

/// Untrusted M2 native MIR claims.
pub mod raw {
    use zryna_ir::Type;

    /// Claimed dense module identifier.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct ModuleId(pub u32);
    /// Claimed canonical function identity.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct FunctionId {
        /// Containing module.
        pub module: ModuleId,
        /// Dense declaration index.
        pub declaration: u32,
    }
    /// Claimed function-local block identifier.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct BlockId(pub u32);
    /// Claimed function-local value identifier.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct ValueId(pub u32);

    /// Untrusted calling-convention claim.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct CallingConvention(u16);
    impl CallingConvention {
        /// Internal typed control-flow convention. This is not the public scalar ABI.
        pub const ZRYNA_INTERNAL_CONTROL_FLOW_V1: Self = Self(2);
        /// Constructs an arbitrary unverified convention code.
        #[must_use]
        pub const fn from_code(code: u16) -> Self {
            Self(code)
        }
        /// Returns the unverified convention code.
        #[must_use]
        pub const fn code(self) -> u16 {
            self.0
        }
    }

    /// One claimed typed value definition.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ValueDefinition {
        /// Dense value identity.
        pub id: ValueId,
        /// Exact scalar type claim.
        pub ty: Type,
    }

    /// One claimed instruction.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Instruction {
        /// Claimed result definition.
        pub result: ValueDefinition,
        /// Claimed operation.
        pub kind: InstructionKind,
    }

    /// Exhaustive M2 native MIR operations.
    #[allow(missing_docs)]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum InstructionKind {
        BoolLiteral(bool),
        I32Literal(i32),
        I32Add { lhs: ValueId, rhs: ValueId },
        I32Sub { lhs: ValueId, rhs: ValueId },
        I32Mul { lhs: ValueId, rhs: ValueId },
        I32Neg { operand: ValueId },
        Eq { lhs: ValueId, rhs: ValueId },
        Ne { lhs: ValueId, rhs: ValueId },
        I32LtS { lhs: ValueId, rhs: ValueId },
        I32LeS { lhs: ValueId, rhs: ValueId },
        I32GtS { lhs: ValueId, rhs: ValueId },
        I32GeS { lhs: ValueId, rhs: ValueId },
        DirectCall { callee: FunctionId, arguments: Vec<ValueId> },
    }

    /// Exhaustive M2 native MIR terminators.
    #[allow(missing_docs)]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum Terminator {
        Return(ValueId),
        Jump {
            target: BlockId,
            arguments: Vec<ValueId>,
        },
        Branch {
            condition: ValueId,
            true_target: BlockId,
            true_arguments: Vec<ValueId>,
            false_target: BlockId,
            false_arguments: Vec<ValueId>,
        },
    }

    /// One claimed dense basic block.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Block {
        /// Dense block identity.
        pub id: BlockId,
        /// Block parameters; entry block must have none.
        pub parameters: Vec<ValueDefinition>,
        /// Ordered instructions.
        pub instructions: Vec<Instruction>,
        /// Exactly one terminator is required; a vector retains malformed claims.
        pub terminators: Vec<Terminator>,
    }

    /// One claimed function.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Function {
        /// Canonical function identity.
        pub id: FunctionId,
        /// Exact deterministic internal body symbol claim.
        pub internal_symbol: String,
        /// Entry-module public logical name, if any.
        pub entry_export: Option<String>,
        /// Internal calling convention claim.
        pub convention: CallingConvention,
        /// Ordered function parameters.
        pub parameters: Vec<ValueDefinition>,
        /// Exact scalar result.
        pub result: Type,
        /// Dense block arena.
        pub blocks: Vec<Block>,
    }

    /// One claimed module.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Module {
        /// Dense module identity.
        pub id: ModuleId,
        /// Functions in declaration order.
        pub functions: Vec<Function>,
    }

    /// Complete raw M2 native MIR program.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Program {
        /// Claimed entry module.
        pub entry_module: ModuleId,
        /// Complete canonical module closure.
        pub modules: Vec<Module>,
    }
}

/// Opaque verified module identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleIdentity(u32);
impl ModuleIdentity {
    /// Returns the dense module index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}
/// Opaque verified function identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FunctionIdentity {
    module: ModuleIdentity,
    declaration: u32,
}
impl FunctionIdentity {
    /// Returns the containing module.
    #[must_use]
    pub const fn module(self) -> ModuleIdentity {
        self.module
    }
    /// Returns the dense declaration index.
    #[must_use]
    pub const fn declaration(self) -> u32 {
        self.declaration
    }
}
/// Opaque verified block identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockIdentity(u32);
impl BlockIdentity {
    /// Returns the dense block index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}
/// Opaque verified value identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValueIdentity(u32);
impl ValueIdentity {
    /// Returns the dense value index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Verified internal M2 calling convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedCallingConvention {
    /// Typed internal control-flow convention; distinct from public scalar ABI wrappers.
    ControlFlowV1,
}

/// Program sealed by the independent native MIR verifier.
///
/// Raw claims cannot construct the sealed program directly:
///
/// ```compile_fail
/// use zryna_native_mir::control_flow_v1::{VerifiedProgram, raw};
///
/// fn forge(program: raw::Program) -> VerifiedProgram {
///     VerifiedProgram {
///         program,
///         abi: todo!(),
///         abi_indices: Vec::new(),
///     }
/// }
/// ```
///
/// Nor can callers recover the retained raw program and mutate it after verification:
///
/// ```compile_fail
/// use zryna_native_mir::control_flow_v1::{VerifiedProgram, raw};
///
/// fn recover(verified: VerifiedProgram) -> raw::Program {
///     verified.program
/// }
/// ```
#[derive(Clone, Debug)]
pub struct VerifiedProgram {
    program: raw::Program,
    abi: VerifiedScalarAbiModule,
    abi_indices: Vec<Vec<Option<usize>>>,
}

impl VerifiedProgram {
    /// Returns the entry module.
    #[must_use]
    pub const fn entry_module(&self) -> ModuleIdentity {
        ModuleIdentity(self.program.entry_module.0)
    }
    /// Iterates modules in canonical order.
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = VerifiedModule<'_>> {
        self.program.modules.iter().enumerate().map(|(index, module)| VerifiedModule {
            owner: self,
            index,
            module,
        })
    }
    /// Returns the independently sealed public scalar ABI.
    #[must_use]
    pub const fn scalar_abi(&self) -> &VerifiedScalarAbiModule {
        &self.abi
    }
}

/// Immutable verified module view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedModule<'a> {
    owner: &'a VerifiedProgram,
    index: usize,
    module: &'a raw::Module,
}
impl<'a> VerifiedModule<'a> {
    /// Returns the module identity.
    #[must_use]
    pub const fn id(self) -> ModuleIdentity {
        ModuleIdentity(self.module.id.0)
    }
    /// Iterates functions in declaration order.
    #[must_use]
    pub fn functions(self) -> impl ExactSizeIterator<Item = VerifiedFunction<'a>> {
        self.module.functions.iter().enumerate().map(move |(function_index, function)| {
            VerifiedFunction {
                owner: self.owner,
                module_index: self.index,
                function_index,
                function,
            }
        })
    }
}

/// Immutable verified function view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedFunction<'a> {
    owner: &'a VerifiedProgram,
    module_index: usize,
    function_index: usize,
    function: &'a raw::Function,
}
impl<'a> VerifiedFunction<'a> {
    /// Returns the canonical function identity.
    #[must_use]
    pub const fn id(self) -> FunctionIdentity {
        FunctionIdentity {
            module: ModuleIdentity(self.function.id.module.0),
            declaration: self.function.id.declaration,
        }
    }
    /// Returns the deterministic internal body symbol.
    #[must_use]
    pub fn internal_symbol(self) -> &'a str {
        &self.function.internal_symbol
    }
    /// Returns public ABI metadata for entry exports.
    #[must_use]
    pub fn public_export(self) -> Option<VerifiedScalarExport<'a>> {
        self.owner.abi_indices[self.module_index][self.function_index]
            .and_then(|index| self.owner.abi.exports().nth(index))
    }
    /// Returns the verified internal convention.
    #[must_use]
    pub const fn calling_convention(self) -> VerifiedCallingConvention {
        VerifiedCallingConvention::ControlFlowV1
    }
    /// Iterates typed function parameters.
    #[must_use]
    pub fn parameters(self) -> impl ExactSizeIterator<Item = (ValueIdentity, Type)> + 'a {
        self.function.parameters.iter().map(|value| (ValueIdentity(value.id.0), value.ty))
    }
    /// Returns the result type.
    #[must_use]
    pub const fn result(self) -> Type {
        self.function.result
    }
    /// Iterates blocks.
    #[must_use]
    pub fn blocks(self) -> impl ExactSizeIterator<Item = VerifiedBlock<'a>> {
        self.function.blocks.iter().map(move |block| VerifiedBlock { function: self, block })
    }
}

/// Immutable verified block view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedBlock<'a> {
    function: VerifiedFunction<'a>,
    block: &'a raw::Block,
}
impl<'a> VerifiedBlock<'a> {
    /// Returns the block identity.
    #[must_use]
    pub const fn id(self) -> BlockIdentity {
        BlockIdentity(self.block.id.0)
    }
    /// Iterates block parameters.
    #[must_use]
    pub fn parameters(self) -> impl ExactSizeIterator<Item = (ValueIdentity, Type)> + 'a {
        self.block.parameters.iter().map(|value| (ValueIdentity(value.id.0), value.ty))
    }
    /// Iterates instructions.
    #[must_use]
    pub fn instructions(self) -> impl ExactSizeIterator<Item = VerifiedInstruction<'a>> {
        self.block
            .instructions
            .iter()
            .map(move |instruction| VerifiedInstruction { function: self.function, instruction })
    }
    /// Returns the exactly-one terminator.
    #[must_use]
    pub fn terminator(self) -> VerifiedTerminator<'a> {
        VerifiedTerminator { terminator: &self.block.terminators[0] }
    }
}

/// Immutable verified instruction view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedInstruction<'a> {
    function: VerifiedFunction<'a>,
    instruction: &'a raw::Instruction,
}
impl<'a> VerifiedInstruction<'a> {
    /// Returns the result identity.
    #[must_use]
    pub const fn result(self) -> ValueIdentity {
        ValueIdentity(self.instruction.result.id.0)
    }
    /// Returns the result type.
    #[must_use]
    pub const fn ty(self) -> Type {
        self.instruction.result.ty
    }
    /// Returns the operation view.
    #[must_use]
    pub fn kind(self) -> VerifiedInstructionKind<'a> {
        operation_view(self.function, &self.instruction.kind)
    }
}

/// Immutable list of verified value identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedValueList<'a>(&'a [raw::ValueId]);
impl VerifiedValueList<'_> {
    /// Iterates identities in evaluation order.
    #[must_use]
    pub fn iter(self) -> impl ExactSizeIterator<Item = ValueIdentity> {
        self.0.iter().map(|id| ValueIdentity(id.0))
    }
    /// Returns the list length.
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.len()
    }
    /// Returns whether the list is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

/// Exhaustive immutable verified operation view.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedInstructionKind<'a> {
    BoolLiteral(bool),
    I32Literal(i32),
    I32Add(ValueIdentity, ValueIdentity),
    I32Sub(ValueIdentity, ValueIdentity),
    I32Mul(ValueIdentity, ValueIdentity),
    I32Neg(ValueIdentity),
    Eq(ValueIdentity, ValueIdentity),
    Ne(ValueIdentity, ValueIdentity),
    I32LtS(ValueIdentity, ValueIdentity),
    I32LeS(ValueIdentity, ValueIdentity),
    I32GtS(ValueIdentity, ValueIdentity),
    I32GeS(ValueIdentity, ValueIdentity),
    DirectCall { callee: FunctionIdentity, arguments: VerifiedValueList<'a> },
}

/// Immutable verified terminator view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedTerminator<'a> {
    terminator: &'a raw::Terminator,
}
impl<'a> VerifiedTerminator<'a> {
    /// Returns the terminator operation.
    #[must_use]
    pub fn kind(self) -> VerifiedTerminatorKind<'a> {
        terminator_view(self.terminator)
    }
}
/// Exhaustive immutable verified terminator operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedTerminatorKind<'a> {
    Return(ValueIdentity),
    Jump {
        target: BlockIdentity,
        arguments: VerifiedValueList<'a>,
    },
    Branch {
        condition: ValueIdentity,
        true_target: BlockIdentity,
        true_arguments: VerifiedValueList<'a>,
        false_target: BlockIdentity,
        false_arguments: VerifiedValueList<'a>,
    },
}

fn operation_view<'a>(
    function: VerifiedFunction<'a>,
    kind: &'a raw::InstructionKind,
) -> VerifiedInstructionKind<'a> {
    use raw::InstructionKind as I;
    let id = |value: raw::ValueId| ValueIdentity(value.0);
    match kind {
        I::BoolLiteral(v) => VerifiedInstructionKind::BoolLiteral(*v),
        I::I32Literal(v) => VerifiedInstructionKind::I32Literal(*v),
        I::I32Add { lhs, rhs } => VerifiedInstructionKind::I32Add(id(*lhs), id(*rhs)),
        I::I32Sub { lhs, rhs } => VerifiedInstructionKind::I32Sub(id(*lhs), id(*rhs)),
        I::I32Mul { lhs, rhs } => VerifiedInstructionKind::I32Mul(id(*lhs), id(*rhs)),
        I::I32Neg { operand } => VerifiedInstructionKind::I32Neg(id(*operand)),
        I::Eq { lhs, rhs } => VerifiedInstructionKind::Eq(id(*lhs), id(*rhs)),
        I::Ne { lhs, rhs } => VerifiedInstructionKind::Ne(id(*lhs), id(*rhs)),
        I::I32LtS { lhs, rhs } => VerifiedInstructionKind::I32LtS(id(*lhs), id(*rhs)),
        I::I32LeS { lhs, rhs } => VerifiedInstructionKind::I32LeS(id(*lhs), id(*rhs)),
        I::I32GtS { lhs, rhs } => VerifiedInstructionKind::I32GtS(id(*lhs), id(*rhs)),
        I::I32GeS { lhs, rhs } => VerifiedInstructionKind::I32GeS(id(*lhs), id(*rhs)),
        I::DirectCall { callee, arguments } => {
            let _ = function;
            VerifiedInstructionKind::DirectCall {
                callee: FunctionIdentity {
                    module: ModuleIdentity(callee.module.0),
                    declaration: callee.declaration,
                },
                arguments: VerifiedValueList(arguments),
            }
        }
    }
}
fn terminator_view(kind: &raw::Terminator) -> VerifiedTerminatorKind<'_> {
    let id = |value: raw::ValueId| ValueIdentity(value.0);
    match kind {
        raw::Terminator::Return(value) => VerifiedTerminatorKind::Return(id(*value)),
        raw::Terminator::Jump { target, arguments } => VerifiedTerminatorKind::Jump {
            target: BlockIdentity(target.0),
            arguments: VerifiedValueList(arguments),
        },
        raw::Terminator::Branch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => VerifiedTerminatorKind::Branch {
            condition: id(*condition),
            true_target: BlockIdentity(true_target.0),
            true_arguments: VerifiedValueList(true_arguments),
            false_target: BlockIdentity(false_target.0),
            false_arguments: VerifiedValueList(false_arguments),
        },
    }
}

#[derive(Clone, Copy)]
struct FunctionKey {
    module: usize,
    function: usize,
}
#[derive(Clone, Copy)]
enum DefinitionLocation {
    Parameter,
    BlockParameter(usize),
    Instruction(usize, usize),
}
#[derive(Clone, Copy)]
struct ValueInfo {
    ty: Type,
    location: DefinitionLocation,
}
#[derive(Clone)]
struct Edge {
    source: usize,
    target: usize,
}

#[derive(Default)]
struct Errors {
    diagnostics: Vec<Diagnostic>,
    terminal: bool,
}
impl Errors {
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.terminal {
            return;
        }
        if self.diagnostics.len() < MAX_DIAGNOSTICS - 1 {
            self.diagnostics.push(diagnostic);
        } else {
            self.diagnostics.push(error(
                "ZRYNA-N2202",
                "native MIR diagnostic budget exceeded",
                "fix the first reported native MIR violation",
            ));
            self.terminal = true;
        }
    }
    fn limit(&mut self, label: &str, maximum: usize) {
        if self.terminal {
            return;
        }
        self.diagnostics.push(error(
            "ZRYNA-N2201",
            format!("native MIR {label} exceeds limit {maximum}"),
            "reduce the bounded M2 native MIR input",
        ));
        self.terminal = true;
    }
    fn exhausted(&self) -> bool {
        self.terminal
    }
    fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
    fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}
fn error(
    code: &'static str,
    message: impl Into<String>,
    guidance: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(code, None, message, guidance)
}

/// Returns the only canonical internal body symbol for one function identity.
#[must_use]
pub fn canonical_internal_symbol(id: raw::FunctionId) -> String {
    format!("zryna_m2_i_m{}_f{}", id.module.0, id.declaration)
}

/// Independently verifies one raw M2 native MIR program.
///
/// # Errors
///
/// Returns deterministic bounded diagnostics for resource, identity, symbol, type, CFG, call,
/// or ABI violations. No verified value is created after any violation.
pub fn verify(program: raw::Program) -> Result<VerifiedProgram, Vec<Diagnostic>> {
    let mut errors = Errors::default();
    preflight(&program, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    verify_inventory(&program, &mut errors);
    if errors.exhausted() {
        return Err(errors.finish());
    }
    let signatures = collect_signatures(&program);
    let mut calls = Vec::new();
    'outer: for (module_index, module) in program.modules.iter().enumerate() {
        for (function_index, function) in module.functions.iter().enumerate() {
            verify_function(
                FunctionKey { module: module_index, function: function_index },
                function,
                &signatures,
                &mut calls,
                &mut errors,
            );
            if errors.exhausted() {
                break 'outer;
            }
        }
    }
    if errors.exhausted() {
        return Err(errors.finish());
    }
    verify_call_graph(&program, &calls, &mut errors);
    if errors.exhausted() {
        return Err(errors.finish());
    }
    let (abi, abi_indices) = verify_public_abi(&program, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let Some(abi) = abi else {
        return Err(vec![error(
            "ZRYNA-N2202",
            "native MIR could not construct its bounded ABI table",
            "report this compiler invariant failure",
        )]);
    };
    Ok(VerifiedProgram { program, abi, abi_indices })
}

#[allow(clippy::too_many_lines)]
fn preflight(program: &raw::Program, errors: &mut Errors) {
    if program.modules.is_empty() {
        errors.push(error(
            "ZRYNA-N2101",
            "native MIR program has no entry module",
            "provide one complete nonempty module closure",
        ));
        return;
    }
    if program.modules.len() > MAX_MODULES {
        errors.limit("module count", MAX_MODULES);
        return;
    }
    let mut functions = 0usize;
    let mut parameters = 0usize;
    let mut blocks = 0usize;
    let mut values = 0usize;
    let mut edges = 0usize;
    let mut calls = 0usize;
    let mut call_arguments = 0usize;
    let mut edge_arguments = 0usize;
    let mut internal_symbol_bytes = 0usize;
    let mut entry_export_bytes = 0usize;
    let mut terminators = 0usize;
    for module in &program.modules {
        if module.functions.len() > MAX_FUNCTIONS_PER_MODULE {
            errors.limit("functions per module", MAX_FUNCTIONS_PER_MODULE);
            return;
        }
        if add_total(
            &mut functions,
            module.functions.len(),
            MAX_FUNCTIONS_PER_PROGRAM,
            "function count",
            errors,
        ) {
            return;
        }
        for function in &module.functions {
            if function.internal_symbol.len() > MAX_INTERNAL_SYMBOL_BYTES {
                errors.limit("internal symbol bytes", MAX_INTERNAL_SYMBOL_BYTES);
                return;
            }
            if add_total(
                &mut internal_symbol_bytes,
                function.internal_symbol.len(),
                MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM,
                "aggregate internal symbol bytes",
                errors,
            ) {
                return;
            }
            if let Some(export) = &function.entry_export {
                if export.len() > MAX_ENTRY_EXPORT_BYTES {
                    errors.limit("entry export bytes", MAX_ENTRY_EXPORT_BYTES);
                    return;
                }
                if add_total(
                    &mut entry_export_bytes,
                    export.len(),
                    MAX_ENTRY_EXPORT_BYTES_PER_PROGRAM,
                    "aggregate entry export bytes",
                    errors,
                ) {
                    return;
                }
            }
            if function.parameters.len() > MAX_PARAMETERS_PER_FUNCTION {
                errors.limit("parameters per function", MAX_PARAMETERS_PER_FUNCTION);
                return;
            }
            if add_total(
                &mut parameters,
                function.parameters.len(),
                MAX_PARAMETERS_PER_PROGRAM,
                "parameter count",
                errors,
            ) {
                return;
            }
            if function.blocks.len() > MAX_BLOCKS_PER_FUNCTION {
                errors.limit("blocks per function", MAX_BLOCKS_PER_FUNCTION);
                return;
            }
            if add_total(
                &mut blocks,
                function.blocks.len(),
                MAX_BLOCKS_PER_PROGRAM,
                "block count",
                errors,
            ) {
                return;
            }
            let mut function_values = function.parameters.len();
            let mut function_edges = 0usize;
            let mut function_terminators = 0usize;
            for block in &function.blocks {
                if block.parameters.len() > MAX_BLOCK_PARAMETERS {
                    errors.limit("block parameter count", MAX_BLOCK_PARAMETERS);
                    return;
                }
                function_values = if let Some(value) = function_values
                    .checked_add(block.parameters.len())
                    .and_then(|v| v.checked_add(block.instructions.len()))
                {
                    value
                } else {
                    errors.limit("value count", MAX_VALUES_PER_PROGRAM);
                    return;
                };
                if function_values > MAX_VALUES_PER_FUNCTION {
                    errors.limit("values per function", MAX_VALUES_PER_FUNCTION);
                    return;
                }
                for instruction in &block.instructions {
                    if let raw::InstructionKind::DirectCall { arguments, .. } = &instruction.kind {
                        calls = if let Some(value) = calls.checked_add(1) {
                            value
                        } else {
                            errors.limit("call count", MAX_CALL_EDGES);
                            return;
                        };
                        if calls > MAX_CALL_EDGES {
                            errors.limit("call count", MAX_CALL_EDGES);
                            return;
                        }
                        if arguments.len() > MAX_PARAMETERS_PER_FUNCTION {
                            errors.limit("call argument count", MAX_PARAMETERS_PER_FUNCTION);
                            return;
                        }
                        if add_total(
                            &mut call_arguments,
                            arguments.len(),
                            MAX_CALL_ARGUMENTS_PER_PROGRAM,
                            "aggregate call argument count",
                            errors,
                        ) {
                            return;
                        }
                    }
                }
                if add_total(
                    &mut function_terminators,
                    block.terminators.len(),
                    MAX_TERMINATORS_PER_FUNCTION,
                    "terminators per function",
                    errors,
                ) || add_total(
                    &mut terminators,
                    block.terminators.len(),
                    MAX_TERMINATORS_PER_PROGRAM,
                    "terminator count",
                    errors,
                ) {
                    return;
                }
                for terminator in &block.terminators {
                    let added = match terminator {
                        raw::Terminator::Return(_) => 0,
                        raw::Terminator::Jump { arguments, .. } => {
                            if arguments.len() > MAX_BLOCK_PARAMETERS {
                                errors.limit("edge argument count", MAX_BLOCK_PARAMETERS);
                                return;
                            }
                            if add_total(
                                &mut edge_arguments,
                                arguments.len(),
                                MAX_EDGE_ARGUMENTS_PER_PROGRAM,
                                "aggregate edge argument count",
                                errors,
                            ) {
                                return;
                            }
                            1
                        }
                        raw::Terminator::Branch { true_arguments, false_arguments, .. } => {
                            if true_arguments.len() > MAX_BLOCK_PARAMETERS
                                || false_arguments.len() > MAX_BLOCK_PARAMETERS
                            {
                                errors.limit("edge argument count", MAX_BLOCK_PARAMETERS);
                                return;
                            }
                            if add_total(
                                &mut edge_arguments,
                                true_arguments.len(),
                                MAX_EDGE_ARGUMENTS_PER_PROGRAM,
                                "aggregate edge argument count",
                                errors,
                            ) || add_total(
                                &mut edge_arguments,
                                false_arguments.len(),
                                MAX_EDGE_ARGUMENTS_PER_PROGRAM,
                                "aggregate edge argument count",
                                errors,
                            ) {
                                return;
                            }
                            2
                        }
                    };
                    function_edges = if let Some(value) = function_edges.checked_add(added) {
                        value
                    } else {
                        errors.limit("CFG edge count", MAX_CFG_EDGES_PER_FUNCTION);
                        return;
                    };
                    if function_edges > MAX_CFG_EDGES_PER_FUNCTION {
                        errors.limit("CFG edges per function", MAX_CFG_EDGES_PER_FUNCTION);
                        return;
                    }
                }
            }
            if add_total(
                &mut values,
                function_values,
                MAX_VALUES_PER_PROGRAM,
                "value count",
                errors,
            ) {
                return;
            }
            if add_total(
                &mut edges,
                function_edges,
                MAX_CFG_EDGES_PER_PROGRAM,
                "CFG edge count",
                errors,
            ) {
                return;
            }
        }
    }
}
fn add_total(
    current: &mut usize,
    extra: usize,
    maximum: usize,
    label: &str,
    errors: &mut Errors,
) -> bool {
    let Some(total) = current.checked_add(extra) else {
        errors.limit(label, maximum);
        return true;
    };
    *current = total;
    if total > maximum {
        errors.limit(label, maximum);
        true
    } else {
        false
    }
}

fn verify_inventory(program: &raw::Program, errors: &mut Errors) {
    let entry = usize::try_from(program.entry_module.0).ok();
    if entry.is_none_or(|index| index >= program.modules.len()) {
        errors.push(error(
            "ZRYNA-N2101",
            "entry module identity is unknown",
            "select one dense module identity",
        ));
    }
    let mut exact = BTreeMap::<String, (usize, usize)>::new();
    let mut portable = BTreeMap::<String, (usize, usize)>::new();
    for (module_index, module) in program.modules.iter().enumerate() {
        if module.id.0 != u32::try_from(module_index).expect("bounded module count fits u32") {
            errors.push(error(
                "ZRYNA-N2102",
                format!("module #{module_index} has a noncanonical identity"),
                "use dense modules in canonical order",
            ));
        }
        for (function_index, function) in module.functions.iter().enumerate() {
            let expected = raw::FunctionId {
                module: module.id,
                declaration: u32::try_from(function_index)
                    .expect("bounded function count fits u32"),
            };
            if function.id != expected {
                errors.push(error(
                    "ZRYNA-N2102",
                    format!(
                        "function #{module_index}:{function_index} has a noncanonical identity"
                    ),
                    "use its module and dense declaration index",
                ));
            }
            let symbol = canonical_internal_symbol(expected);
            if function.internal_symbol != symbol {
                errors.push(error("ZRYNA-N2103", format!("function #{module_index}:{function_index} has a noncanonical internal symbol"), "derive the symbol exactly from the sealed function identity"));
            }
            if exact
                .insert(function.internal_symbol.clone(), (module_index, function_index))
                .is_some()
                || portable
                    .insert(
                        function.internal_symbol.to_ascii_lowercase(),
                        (module_index, function_index),
                    )
                    .is_some()
            {
                errors.push(error(
                    "ZRYNA-N2103",
                    "internal body symbols collide",
                    "use unique canonical internal body symbols",
                ));
            }
            if function.convention != raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1 {
                errors.push(error(
                    "ZRYNA-N2104",
                    "function uses an unsupported internal convention",
                    "use only the versioned control-flow-v1 convention",
                ));
            }
            if module.id != program.entry_module && function.entry_export.is_some() {
                errors.push(error(
                    "ZRYNA-N2105",
                    "dependency function claims a public entry export",
                    "only entry-module functions may be exported",
                ));
            }
            if errors.exhausted() {
                return;
            }
        }
    }
}

fn collect_signatures(program: &raw::Program) -> Vec<Vec<(Vec<Type>, Type)>> {
    program
        .modules
        .iter()
        .map(|module| {
            module
                .functions
                .iter()
                .map(|function| {
                    (function.parameters.iter().map(|value| value.ty).collect(), function.result)
                })
                .collect()
        })
        .collect()
}

fn verify_function(
    key: FunctionKey,
    function: &raw::Function,
    signatures: &[Vec<(Vec<Type>, Type)>],
    calls: &mut Vec<(FunctionKey, FunctionKey)>,
    errors: &mut Errors,
) {
    validate_type(function.result, "function result", errors);
    if function.blocks.is_empty() {
        errors.push(error(
            "ZRYNA-N2107",
            format!("function #{}:{} has no entry block or terminator", key.module, key.function),
            "emit a nonempty block arena whose entry block has exactly one terminator",
        ));
        return;
    }
    let mut values = Vec::<ValueInfo>::new();
    for parameter in &function.parameters {
        define_value(parameter, DefinitionLocation::Parameter, &mut values, errors);
    }
    for (block_index, block) in function.blocks.iter().enumerate() {
        if block.id.0 != u32::try_from(block_index).expect("bounded block count fits u32") {
            errors.push(error(
                "ZRYNA-N2102",
                "block has a noncanonical dense identity",
                "number blocks densely in arena order",
            ));
        }
        if block_index == 0 && !block.parameters.is_empty() {
            errors.push(error(
                "ZRYNA-N2105",
                "entry block declares block parameters",
                "use function parameters for entry values",
            ));
        }
        for parameter in &block.parameters {
            define_value(
                parameter,
                DefinitionLocation::BlockParameter(block_index),
                &mut values,
                errors,
            );
        }
        for (position, instruction) in block.instructions.iter().enumerate() {
            define_value(
                &instruction.result,
                DefinitionLocation::Instruction(block_index, position),
                &mut values,
                errors,
            );
        }
        if errors.exhausted() {
            return;
        }
    }
    let mut edges = Vec::new();
    for (block_index, block) in function.blocks.iter().enumerate() {
        for (position, instruction) in block.instructions.iter().enumerate() {
            verify_instruction(
                key,
                block_index,
                position,
                instruction,
                &values,
                signatures,
                calls,
                errors,
            );
            if errors.exhausted() {
                return;
            }
        }
        if block.terminators.len() != 1 {
            errors.push(error(
                "ZRYNA-N2106",
                format!(
                    "block #{block_index} has {} terminators; expected exactly one",
                    block.terminators.len()
                ),
                "emit exactly one return, jump, or branch",
            ));
            continue;
        }
        verify_terminator(
            key,
            block_index,
            function,
            &block.terminators[0],
            &values,
            &mut edges,
            errors,
        );
        if errors.exhausted() {
            return;
        }
    }
    verify_cfg(key, function, &values, &edges, errors);
}

fn validate_type(ty: Type, label: &str, errors: &mut Errors) {
    if ty == Type::Unit {
        errors.push(error(
            "ZRYNA-N2104",
            format!("{label} uses unsupported unit type"),
            "native MIR control-flow v1 admits only bool and i32",
        ));
    }
}
fn define_value(
    definition: &raw::ValueDefinition,
    location: DefinitionLocation,
    values: &mut Vec<ValueInfo>,
    errors: &mut Errors,
) {
    let expected = u32::try_from(values.len()).expect("bounded value count fits u32");
    if definition.id.0 != expected {
        errors.push(error(
            "ZRYNA-N2102",
            format!("value #{} is noncanonical; expected #{expected}", definition.id.0),
            "allocate function parameters, block parameters, and instruction results in order",
        ));
    }
    validate_type(definition.ty, "value", errors);
    if !errors.exhausted() {
        values.push(ValueInfo { ty: definition.ty, location });
    }
}
fn value_info(id: raw::ValueId, values: &[ValueInfo]) -> Option<ValueInfo> {
    usize::try_from(id.0).ok().and_then(|index| values.get(index).copied())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_instruction(
    key: FunctionKey,
    block: usize,
    position: usize,
    instruction: &raw::Instruction,
    values: &[ValueInfo],
    signatures: &[Vec<(Vec<Type>, Type)>],
    calls: &mut Vec<(FunctionKey, FunctionKey)>,
    errors: &mut Errors,
) {
    use raw::InstructionKind as I;
    let result = instruction.result.ty;
    match &instruction.kind {
        I::BoolLiteral(_) => expect_result(result, Type::Bool, "bool literal", errors),
        I::I32Literal(_) => expect_result(result, Type::I32, "i32 literal", errors),
        I::I32Neg { operand } => expect_operands(
            &[*operand],
            &[Type::I32],
            result,
            Type::I32,
            block,
            position,
            values,
            errors,
        ),
        I::I32Add { lhs, rhs } | I::I32Sub { lhs, rhs } | I::I32Mul { lhs, rhs } => {
            expect_operands(
                &[*lhs, *rhs],
                &[Type::I32, Type::I32],
                result,
                Type::I32,
                block,
                position,
                values,
                errors,
            );
        }
        I::I32LtS { lhs, rhs }
        | I::I32LeS { lhs, rhs }
        | I::I32GtS { lhs, rhs }
        | I::I32GeS { lhs, rhs } => expect_operands(
            &[*lhs, *rhs],
            &[Type::I32, Type::I32],
            result,
            Type::Bool,
            block,
            position,
            values,
            errors,
        ),
        I::Eq { lhs, rhs } | I::Ne { lhs, rhs } => {
            let left = checked_use(*lhs, block, position, values, errors).map(|value| value.ty);
            let right = checked_use(*rhs, block, position, values, errors).map(|value| value.ty);
            if result != Type::Bool || left.is_none() || left != right {
                errors.push(error(
                    "ZRYNA-N2107",
                    "equality requires matching scalar operands and bool result",
                    "use exact matching bool or i32 operands",
                ));
            }
        }
        I::DirectCall { callee, arguments } => {
            let target = usize::try_from(callee.module.0).ok().and_then(|module| {
                usize::try_from(callee.declaration)
                    .ok()
                    .map(|function| FunctionKey { module, function })
            });
            let signature = target.and_then(|target| {
                signatures
                    .get(target.module)
                    .and_then(|module| module.get(target.function))
                    .map(|signature| (target, signature))
            });
            let Some((target, (parameters, target_result))) = signature else {
                errors.push(error(
                    "ZRYNA-N2111",
                    "direct call targets an unknown function identity",
                    "call one function in the sealed module closure",
                ));
                return;
            };
            calls.push((key, target));
            if arguments.len() != parameters.len() || result != *target_result {
                errors.push(error(
                    "ZRYNA-N2111",
                    "direct call signature does not match target",
                    "use exact argument and result types",
                ));
            }
            for (index, argument) in arguments.iter().enumerate() {
                if checked_use(*argument, block, position, values, errors).map(|value| value.ty)
                    != parameters.get(index).copied()
                {
                    errors.push(error(
                        "ZRYNA-N2111",
                        format!("direct call argument #{index} has the wrong type"),
                        "preserve exact argument order and types",
                    ));
                }
                if errors.exhausted() {
                    return;
                }
            }
        }
    }
}
fn expect_result(actual: Type, expected: Type, label: &str, errors: &mut Errors) {
    if actual != expected {
        errors.push(error(
            "ZRYNA-N2107",
            format!("{label} has the wrong result type"),
            "use the exact native MIR operation type",
        ));
    }
}
#[allow(clippy::too_many_arguments)]
fn expect_operands(
    operands: &[raw::ValueId],
    expected: &[Type],
    result: Type,
    expected_result: Type,
    block: usize,
    position: usize,
    values: &[ValueInfo],
    errors: &mut Errors,
) {
    let valid = operands.iter().zip(expected).all(|(operand, expected)| {
        checked_use(*operand, block, position, values, errors)
            .is_some_and(|value| value.ty == *expected)
    });
    if !valid || result != expected_result {
        errors.push(error(
            "ZRYNA-N2107",
            "instruction operand or result type is invalid",
            "use exact native MIR operation types",
        ));
    }
}
fn checked_use(
    id: raw::ValueId,
    block: usize,
    position: usize,
    values: &[ValueInfo],
    errors: &mut Errors,
) -> Option<ValueInfo> {
    let Some(value) = value_info(id, values) else {
        errors.push(error(
            "ZRYNA-N2108",
            format!("operand references unknown value #{}", id.0),
            "reference a value in the same function",
        ));
        return None;
    };
    if let DefinitionLocation::Instruction(definition_block, definition_position) = value.location
        && definition_block == block
        && definition_position >= position
    {
        errors.push(error(
            "ZRYNA-N2109",
            format!("value #{} is used before definition", id.0),
            "use only earlier results in the same block",
        ));
    }
    Some(value)
}

fn verify_terminator(
    key: FunctionKey,
    block: usize,
    function: &raw::Function,
    terminator: &raw::Terminator,
    values: &[ValueInfo],
    edges: &mut Vec<Edge>,
    errors: &mut Errors,
) {
    let position = function.blocks[block].instructions.len();
    match terminator {
        raw::Terminator::Return(value) => {
            if checked_use(*value, block, position, values, errors).map(|value| value.ty)
                != Some(function.result)
            {
                errors.push(error(
                    "ZRYNA-N2106",
                    "return value has the wrong type",
                    "return the exact function result type",
                ));
            }
        }
        raw::Terminator::Jump { target, arguments } => {
            verify_edge(key, block, *target, arguments, function, values, edges, errors);
        }
        raw::Terminator::Branch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            if checked_use(*condition, block, position, values, errors).map(|value| value.ty)
                != Some(Type::Bool)
            {
                errors.push(error(
                    "ZRYNA-N2106",
                    "branch condition is not bool",
                    "branch only on exact bool",
                ));
            }
            verify_edge(key, block, *true_target, true_arguments, function, values, edges, errors);
            verify_edge(
                key,
                block,
                *false_target,
                false_arguments,
                function,
                values,
                edges,
                errors,
            );
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn verify_edge(
    key: FunctionKey,
    source: usize,
    target: raw::BlockId,
    arguments: &[raw::ValueId],
    function: &raw::Function,
    values: &[ValueInfo],
    edges: &mut Vec<Edge>,
    errors: &mut Errors,
) {
    let Some(target_index) =
        usize::try_from(target.0).ok().filter(|index| *index < function.blocks.len())
    else {
        errors.push(error(
            "ZRYNA-N2106",
            format!(
                "function #{}:{} targets unknown block #{}",
                key.module, key.function, target.0
            ),
            "target one block in the same function",
        ));
        return;
    };
    if target_index == 0 {
        errors.push(error(
            "ZRYNA-N2106",
            "control-flow edge targets the entry block",
            "entry block zero must have no predecessor",
        ));
    }
    let parameters = &function.blocks[target_index].parameters;
    if arguments.len() != parameters.len() {
        errors.push(error(
            "ZRYNA-N2106",
            "edge argument arity does not match block parameters",
            "pass one exact argument per block parameter",
        ));
    }
    let position = function.blocks[source].instructions.len();
    for (index, argument) in arguments.iter().enumerate() {
        if checked_use(*argument, source, position, values, errors).map(|value| value.ty)
            != parameters.get(index).map(|parameter| parameter.ty)
        {
            errors.push(error(
                "ZRYNA-N2106",
                format!("edge argument #{index} has the wrong type"),
                "match block parameter types exactly",
            ));
        }
    }
    if !errors.exhausted() {
        edges.push(Edge { source, target: target_index });
    }
}

#[allow(clippy::too_many_lines)]
fn verify_cfg(
    key: FunctionKey,
    function: &raw::Function,
    values: &[ValueInfo],
    edges: &[Edge],
    errors: &mut Errors,
) {
    if function.blocks.is_empty() {
        return;
    }
    let count = function.blocks.len();
    let mut successors = vec![Vec::new(); count];
    let mut predecessors = vec![Vec::new(); count];
    for edge in edges {
        successors[edge.source].push(edge.target);
        predecessors[edge.target].push(edge.source);
    }
    for list in &mut successors {
        list.sort_unstable();
    }
    for list in &mut predecessors {
        list.sort_unstable();
    }
    let reachable = reachable_from_entry(&successors);
    for block in 1..count {
        if predecessors[block].is_empty() || !reachable[block] {
            errors.push(error(
                "ZRYNA-N2110",
                format!("function #{}:{} block #{block} is unreachable", key.module, key.function),
                "emit only blocks reachable from entry",
            ));
        }
    }
    if reachable.iter().any(|value| !value) {
        return;
    }
    let Some(idom) = immediate_dominators(&successors, &predecessors) else {
        errors.push(error(
            "ZRYNA-N2110",
            "failed to construct deterministic dominators",
            "provide one reachable reducible CFG",
        ));
        return;
    };
    let (entry_time, exit_time) = dominator_intervals(&idom);
    let dominates = |left: usize, right: usize| {
        entry_time[left] <= entry_time[right] && exit_time[right] <= exit_time[left]
    };
    for (use_block, block) in function.blocks.iter().enumerate() {
        for (position, instruction) in block.instructions.iter().enumerate() {
            for operand in instruction_operands(&instruction.kind) {
                verify_dominance(operand, use_block, position, values, &dominates, errors);
            }
        }
        if block.terminators.len() == 1 {
            for operand in terminator_operands(&block.terminators[0]) {
                verify_dominance(
                    operand,
                    use_block,
                    block.instructions.len(),
                    values,
                    &dominates,
                    errors,
                );
            }
        }
    }
    let mut forward_indegree = vec![0usize; count];
    let mut forward = vec![Vec::new(); count];
    let mut loops = BTreeMap::<usize, Vec<bool>>::new();
    for edge in edges {
        if dominates(edge.target, edge.source) {
            let members = loops.entry(edge.target).or_insert_with(|| vec![false; count]);
            members[edge.target] = true;
            let mut stack = vec![edge.source];
            while let Some(node) = stack.pop() {
                if members[node] {
                    continue;
                }
                members[node] = true;
                for predecessor in &predecessors[node] {
                    if *predecessor != edge.target {
                        stack.push(*predecessor);
                    }
                }
            }
        } else {
            forward[edge.source].push(edge.target);
            forward_indegree[edge.target] = forward_indegree[edge.target].saturating_add(1);
        }
    }
    let mut queue = VecDeque::new();
    for (index, degree) in forward_indegree.iter().enumerate() {
        if *degree == 0 {
            queue.push_back(index);
        }
    }
    let mut seen = 0usize;
    while let Some(node) = queue.pop_front() {
        seen += 1;
        for target in &forward[node] {
            forward_indegree[*target] -= 1;
            if forward_indegree[*target] == 0 {
                queue.push_back(*target);
            }
        }
    }
    if seen != count {
        errors.push(error(
            "ZRYNA-N2110",
            format!("function #{}:{} contains irreducible control flow", key.module, key.function),
            "every cycle must have one dominating nonentry header",
        ));
    }
    let nesting = (0..count)
        .map(|block| loops.values().filter(|members| members[block]).count())
        .max()
        .unwrap_or(0);
    if nesting > MAX_LOOP_NESTING {
        errors.limit("loop nesting", MAX_LOOP_NESTING);
    }
}
fn reachable_from_entry(successors: &[Vec<usize>]) -> Vec<bool> {
    let mut reachable = vec![false; successors.len()];
    let mut stack = vec![0usize];
    while let Some(block) = stack.pop() {
        if reachable[block] {
            continue;
        }
        reachable[block] = true;
        for target in successors[block].iter().rev() {
            stack.push(*target);
        }
    }
    reachable
}
fn immediate_dominators(
    successors: &[Vec<usize>],
    predecessors: &[Vec<usize>],
) -> Option<Vec<usize>> {
    let count = successors.len();
    let mut visited = vec![false; count];
    let mut order = Vec::with_capacity(count);
    let mut stack = vec![(0usize, 0usize)];
    visited[0] = true;
    while let Some((node, child)) = stack.pop() {
        if child < successors[node].len() {
            stack.push((node, child + 1));
            let next = successors[node][child];
            if !visited[next] {
                visited[next] = true;
                stack.push((next, 0));
            }
        } else {
            order.push(node);
        }
    }
    if order.len() != count {
        return None;
    }
    order.reverse();
    let mut position = vec![0usize; count];
    for (index, block) in order.iter().enumerate() {
        position[*block] = index;
    }
    let mut idom = vec![usize::MAX; count];
    idom[0] = 0;
    for _ in 0..count {
        let mut changed = false;
        for block in order.iter().copied().skip(1) {
            let mut defined =
                predecessors[block].iter().copied().filter(|pred| idom[*pred] != usize::MAX);
            let Some(mut candidate) = defined.next() else {
                continue;
            };
            for predecessor in defined {
                candidate = intersect(candidate, predecessor, &idom, &position);
            }
            if idom[block] != candidate {
                idom[block] = candidate;
                changed = true;
            }
        }
        if !changed {
            return idom.iter().all(|value| *value != usize::MAX).then_some(idom);
        }
    }
    None
}
fn intersect(mut left: usize, mut right: usize, idom: &[usize], position: &[usize]) -> usize {
    while left != right {
        while position[left] > position[right] {
            left = idom[left];
        }
        while position[right] > position[left] {
            right = idom[right];
        }
    }
    left
}
fn dominator_intervals(idom: &[usize]) -> (Vec<usize>, Vec<usize>) {
    let mut children = vec![Vec::new(); idom.len()];
    for block in 1..idom.len() {
        children[idom[block]].push(block);
    }
    let mut entry = vec![0usize; idom.len()];
    let mut exit = vec![0usize; idom.len()];
    let mut clock = 0usize;
    let mut stack = vec![(0usize, false)];
    while let Some((node, exiting)) = stack.pop() {
        if exiting {
            exit[node] = clock;
            clock += 1;
        } else {
            entry[node] = clock;
            clock += 1;
            stack.push((node, true));
            for child in children[node].iter().rev() {
                stack.push((*child, false));
            }
        }
    }
    (entry, exit)
}
fn verify_dominance(
    operand: raw::ValueId,
    use_block: usize,
    use_position: usize,
    values: &[ValueInfo],
    dominates: &impl Fn(usize, usize) -> bool,
    errors: &mut Errors,
) {
    let Some(value) = value_info(operand, values) else {
        return;
    };
    let valid = match value.location {
        DefinitionLocation::Parameter => true,
        DefinitionLocation::BlockParameter(block) => dominates(block, use_block),
        DefinitionLocation::Instruction(block, position) => {
            if block == use_block {
                position < use_position
            } else {
                dominates(block, use_block)
            }
        }
    };
    if !valid {
        errors.push(error(
            "ZRYNA-N2109",
            format!("value #{} does not dominate its use", operand.0),
            "use only values available on every path",
        ));
    }
}
fn instruction_operands(kind: &raw::InstructionKind) -> Vec<raw::ValueId> {
    use raw::InstructionKind as I;
    match kind {
        I::BoolLiteral(_) | I::I32Literal(_) => vec![],
        I::I32Neg { operand } => vec![*operand],
        I::I32Add { lhs, rhs }
        | I::I32Sub { lhs, rhs }
        | I::I32Mul { lhs, rhs }
        | I::Eq { lhs, rhs }
        | I::Ne { lhs, rhs }
        | I::I32LtS { lhs, rhs }
        | I::I32LeS { lhs, rhs }
        | I::I32GtS { lhs, rhs }
        | I::I32GeS { lhs, rhs } => vec![*lhs, *rhs],
        I::DirectCall { arguments, .. } => arguments.clone(),
    }
}
fn terminator_operands(kind: &raw::Terminator) -> Vec<raw::ValueId> {
    match kind {
        raw::Terminator::Return(value) => vec![*value],
        raw::Terminator::Jump { arguments, .. } => arguments.clone(),
        raw::Terminator::Branch { condition, true_arguments, false_arguments, .. } => {
            let mut values = Vec::with_capacity(1 + true_arguments.len() + false_arguments.len());
            values.push(*condition);
            values.extend(true_arguments);
            values.extend(false_arguments);
            values
        }
    }
}

fn verify_call_graph(
    program: &raw::Program,
    calls: &[(FunctionKey, FunctionKey)],
    errors: &mut Errors,
) {
    let offsets = function_offsets(program);
    let count = offsets.last().copied().unwrap_or(0);
    let mut successors = vec![Vec::new(); count];
    let mut indegree = vec![0usize; count];
    for (caller, callee) in calls {
        let source = offsets[caller.module] + caller.function;
        let target = offsets[callee.module] + callee.function;
        successors[source].push(target);
        indegree[target] = indegree[target].saturating_add(1);
    }
    let mut queue = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            queue.push_back(index);
        }
    }
    let mut depth = vec![1usize; count];
    let mut seen = 0usize;
    while let Some(function) = queue.pop_front() {
        seen += 1;
        for callee in &successors[function] {
            depth[*callee] = depth[*callee].max(depth[function].saturating_add(1));
            indegree[*callee] -= 1;
            if indegree[*callee] == 0 {
                queue.push_back(*callee);
            }
        }
    }
    if seen != count {
        errors.push(error(
            "ZRYNA-N2112",
            "direct-call graph contains a cycle",
            "native MIR control-flow v1 requires acyclic direct calls",
        ));
    } else if depth.into_iter().max().unwrap_or(0) > MAX_STATIC_CALL_DEPTH {
        errors.limit("static call depth", MAX_STATIC_CALL_DEPTH);
    }
}
fn function_offsets(program: &raw::Program) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(program.modules.len() + 1);
    let mut total = 0usize;
    for module in &program.modules {
        offsets.push(total);
        total += module.functions.len();
    }
    offsets.push(total);
    offsets
}

fn abi_type(ty: Type) -> raw_abi::Type {
    match ty {
        Type::Unit => raw_abi::Type::Unit,
        Type::Bool => raw_abi::Type::Bool,
        Type::I32 => raw_abi::Type::I32,
    }
}
fn verify_public_abi(
    program: &raw::Program,
    errors: &mut Errors,
) -> (Option<VerifiedScalarAbiModule>, Vec<Vec<Option<usize>>>) {
    let mut exports = Vec::new();
    let mut indices =
        program.modules.iter().map(|module| vec![None; module.functions.len()]).collect::<Vec<_>>();
    if let Some(entry) =
        usize::try_from(program.entry_module.0).ok().filter(|index| *index < program.modules.len())
    {
        for (function_index, function) in program.modules[entry].functions.iter().enumerate() {
            if let Some(name) = &function.entry_export {
                indices[entry][function_index] = Some(exports.len());
                exports.push(raw_abi::Export::new(
                    name.clone(),
                    raw_abi::Signature::new(
                        function
                            .parameters
                            .iter()
                            .map(|parameter| abi_type(parameter.ty))
                            .collect(),
                        abi_type(function.result),
                    ),
                ));
            }
        }
    }
    match verify_v1(raw_abi::Module::new(exports)) {
        Ok(abi) => (Some(abi), indices),
        Err(violations) => {
            for violation in violations {
                let message = match violation.kind() {
                    AbiViolationKind::InvalidLogicalName => {
                        "entry export has an invalid logical name"
                    }
                    AbiViolationKind::DuplicateLogicalName { .. } => {
                        "entry exports duplicate a logical name"
                    }
                    AbiViolationKind::PortableNameCollision { .. } => {
                        "entry exports collide under portable identity"
                    }
                    AbiViolationKind::UnsupportedScalarType => {
                        "entry export uses an unsupported scalar type"
                    }
                    AbiViolationKind::TooManyExports
                    | AbiViolationKind::TooManyParameters
                    | AbiViolationKind::TooManyParametersInModule => {
                        errors.limit("public ABI", zryna_abi::MAX_ABI_EXPORTS);
                        continue;
                    }
                    AbiViolationKind::ViolationBudgetExceeded => {
                        errors.push(error(
                            "ZRYNA-N2202",
                            "scalar ABI diagnostic budget exceeded",
                            "fix the first public ABI violation",
                        ));
                        continue;
                    }
                };
                errors.push(error("ZRYNA-N2113", message, "use exact scalar ABI v1 export claims"));
            }
            (None, indices)
        }
    }
}

/// Lowers a sealed Universal `ControlFlowV1` program one-for-one, then independently verifies it.
///
/// # Errors
///
/// Returns native MIR diagnostics if target-specific identities, symbols, budgets, CFG, calls, or
/// public ABI metadata cannot be sealed. No partial raw program is exposed.
pub fn lower(
    source: &zryna_ir::control_flow_v1::VerifiedProgram,
) -> Result<VerifiedProgram, Vec<Diagnostic>> {
    let program = raw::Program {
        entry_module: raw::ModuleId(source.entry_module().index()),
        modules: source
            .modules()
            .map(|module| raw::Module {
                id: raw::ModuleId(module.id().index()),
                functions: module.functions().map(lower_function).collect(),
            })
            .collect(),
    };
    verify(program)
}
fn lower_function(function: zryna_ir::control_flow_v1::VerifiedFunction<'_>) -> raw::Function {
    let id = raw::FunctionId {
        module: raw::ModuleId(function.id().module().index()),
        declaration: function.id().declaration(),
    };
    raw::Function {
        id,
        internal_symbol: canonical_internal_symbol(id),
        entry_export: function
            .public_export()
            .map(|export| export.logical_name().as_str().to_owned()),
        convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
        parameters: function
            .parameters()
            .map(|(id, ty, _)| raw::ValueDefinition { id: raw::ValueId(id.index()), ty })
            .collect(),
        result: function.result(),
        blocks: function.blocks().map(lower_block).collect(),
    }
}
fn lower_block(block: zryna_ir::control_flow_v1::VerifiedBlock<'_>) -> raw::Block {
    raw::Block {
        id: raw::BlockId(block.id().index()),
        parameters: block
            .parameters()
            .map(|(id, ty, _)| raw::ValueDefinition { id: raw::ValueId(id.index()), ty })
            .collect(),
        instructions: block
            .instructions()
            .map(|instruction| raw::Instruction {
                result: raw::ValueDefinition {
                    id: raw::ValueId(instruction.result().index()),
                    ty: instruction.ty(),
                },
                kind: lower_operation(instruction.kind()),
            })
            .collect(),
        terminators: vec![lower_terminator(block.terminator().kind())],
    }
}
fn lower_operation(
    kind: zryna_ir::control_flow_v1::VerifiedInstructionKind<'_>,
) -> raw::InstructionKind {
    use zryna_ir::control_flow_v1::VerifiedInstructionKind as I;
    let id = |value: zryna_ir::control_flow_v1::ValueIdentity| raw::ValueId(value.index());
    match kind {
        I::BoolLiteral(value) => raw::InstructionKind::BoolLiteral(value),
        I::I32Literal(value) => raw::InstructionKind::I32Literal(value),
        I::I32Add(lhs, rhs) => raw::InstructionKind::I32Add { lhs: id(lhs), rhs: id(rhs) },
        I::I32Sub(lhs, rhs) => raw::InstructionKind::I32Sub { lhs: id(lhs), rhs: id(rhs) },
        I::I32Mul(lhs, rhs) => raw::InstructionKind::I32Mul { lhs: id(lhs), rhs: id(rhs) },
        I::I32Neg(operand) => raw::InstructionKind::I32Neg { operand: id(operand) },
        I::Eq(lhs, rhs) => raw::InstructionKind::Eq { lhs: id(lhs), rhs: id(rhs) },
        I::Ne(lhs, rhs) => raw::InstructionKind::Ne { lhs: id(lhs), rhs: id(rhs) },
        I::I32LtS(lhs, rhs) => raw::InstructionKind::I32LtS { lhs: id(lhs), rhs: id(rhs) },
        I::I32LeS(lhs, rhs) => raw::InstructionKind::I32LeS { lhs: id(lhs), rhs: id(rhs) },
        I::I32GtS(lhs, rhs) => raw::InstructionKind::I32GtS { lhs: id(lhs), rhs: id(rhs) },
        I::I32GeS(lhs, rhs) => raw::InstructionKind::I32GeS { lhs: id(lhs), rhs: id(rhs) },
        I::DirectCall { callee, arguments } => raw::InstructionKind::DirectCall {
            callee: raw::FunctionId {
                module: raw::ModuleId(callee.module().index()),
                declaration: callee.declaration(),
            },
            arguments: arguments.iter().map(id).collect(),
        },
    }
}
fn lower_terminator(
    kind: zryna_ir::control_flow_v1::VerifiedTerminatorKind<'_>,
) -> raw::Terminator {
    use zryna_ir::control_flow_v1::VerifiedTerminatorKind as T;
    let value = |id: zryna_ir::control_flow_v1::ValueIdentity| raw::ValueId(id.index());
    let block = |id: zryna_ir::control_flow_v1::BlockIdentity| raw::BlockId(id.index());
    match kind {
        T::Return(result) => raw::Terminator::Return(value(result)),
        T::Jump { target, arguments } => raw::Terminator::Jump {
            target: block(target),
            arguments: arguments.iter().map(value).collect(),
        },
        T::Branch { condition, true_target, true_arguments, false_target, false_arguments } => {
            raw::Terminator::Branch {
                condition: value(condition),
                true_target: block(true_target),
                true_arguments: true_arguments.iter().map(value).collect(),
                false_target: block(false_target),
                false_arguments: false_arguments.iter().map(value).collect(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zryna_source::{SourceFileInput, SourceMap};

    fn value(id: u32, ty: Type) -> raw::ValueDefinition {
        raw::ValueDefinition { id: raw::ValueId(id), ty }
    }

    fn function(
        id: u32,
        export: Option<&str>,
        parameters: Vec<raw::ValueDefinition>,
        result: Type,
        blocks: Vec<raw::Block>,
    ) -> raw::Function {
        let id = raw::FunctionId { module: raw::ModuleId(0), declaration: id };
        raw::Function {
            id,
            internal_symbol: canonical_internal_symbol(id),
            entry_export: export.map(str::to_owned),
            convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
            parameters,
            result,
            blocks,
        }
    }

    fn valid_program() -> raw::Program {
        let caller = function(
            0,
            Some("choose"),
            vec![value(0, Type::Bool), value(1, Type::I32)],
            Type::I32,
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: vec![],
                    instructions: vec![raw::Instruction {
                        result: value(2, Type::I32),
                        kind: raw::InstructionKind::DirectCall {
                            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                            arguments: vec![raw::ValueId(1)],
                        },
                    }],
                    terminators: vec![raw::Terminator::Branch {
                        condition: raw::ValueId(0),
                        true_target: raw::BlockId(1),
                        true_arguments: vec![raw::ValueId(2)],
                        false_target: raw::BlockId(2),
                        false_arguments: vec![raw::ValueId(1)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: vec![value(3, Type::I32)],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(3))],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: vec![value(4, Type::I32)],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(4))],
                },
            ],
        );
        let identity = function(
            1,
            None,
            vec![value(0, Type::I32)],
            Type::I32,
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![],
                terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
            }],
        );
        raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions: vec![caller, identity] }],
        }
    }

    fn verified_universal_program() -> zryna_ir::control_flow_v1::VerifiedProgram {
        use zryna_ir::control_flow_v1::raw as ir;
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("source map");
        let file = sources.verify_file_id(0).expect("file");
        let span = sources.span(file, 0, 1).expect("span");
        let program = ir::Program {
            entry_module: ir::ModuleId(0),
            modules: vec![ir::Module {
                id: ir::ModuleId(0),
                source_file: file,
                functions: vec![ir::Function {
                    id: ir::FunctionId { module: ir::ModuleId(0), declaration: 0 },
                    entry_export: Some("identity".to_owned()),
                    span,
                    parameters: vec![ir::ValueDefinition {
                        id: ir::ValueId(0),
                        ty: Type::I32,
                        span,
                    }],
                    result: Type::I32,
                    blocks: vec![ir::Block {
                        id: ir::BlockId(0),
                        parameters: vec![],
                        instructions: vec![],
                        terminators: vec![ir::SpannedTerminator {
                            span,
                            kind: ir::Terminator::Return(ir::ValueId(0)),
                        }],
                    }],
                }],
            }],
        };
        zryna_ir::control_flow_v1::verify(program, &sources, file).expect("verified universal IR")
    }

    #[allow(clippy::too_many_lines)]
    fn verified_all_operations_program() -> zryna_ir::control_flow_v1::VerifiedProgram {
        use zryna_ir::control_flow_v1::raw as ir;
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/operations.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("source map");
        let file = sources.verify_file_id(0).expect("file");
        let span = sources.span(file, 0, 1).expect("span");
        let definition = |id, ty| ir::ValueDefinition { id: ir::ValueId(id), ty, span };
        let instruction = |id, ty, kind| ir::Instruction { result: definition(id, ty), kind };
        let program = ir::Program {
            entry_module: ir::ModuleId(0),
            modules: vec![ir::Module {
                id: ir::ModuleId(0),
                source_file: file,
                functions: vec![
                    ir::Function {
                        id: ir::FunctionId { module: ir::ModuleId(0), declaration: 0 },
                        entry_export: Some("operations".to_owned()),
                        span,
                        parameters: vec![
                            definition(0, Type::Bool),
                            definition(1, Type::I32),
                            definition(2, Type::I32),
                        ],
                        result: Type::I32,
                        blocks: vec![ir::Block {
                            id: ir::BlockId(0),
                            parameters: vec![],
                            instructions: vec![
                                instruction(3, Type::Bool, ir::InstructionKind::BoolLiteral(true)),
                                instruction(4, Type::I32, ir::InstructionKind::I32Literal(7)),
                                instruction(
                                    5,
                                    Type::I32,
                                    ir::InstructionKind::I32Add {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    6,
                                    Type::I32,
                                    ir::InstructionKind::I32Sub {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    7,
                                    Type::I32,
                                    ir::InstructionKind::I32Mul {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    8,
                                    Type::I32,
                                    ir::InstructionKind::I32Neg { operand: ir::ValueId(1) },
                                ),
                                instruction(
                                    9,
                                    Type::Bool,
                                    ir::InstructionKind::Eq {
                                        lhs: ir::ValueId(0),
                                        rhs: ir::ValueId(3),
                                    },
                                ),
                                instruction(
                                    10,
                                    Type::Bool,
                                    ir::InstructionKind::Ne {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    11,
                                    Type::Bool,
                                    ir::InstructionKind::I32LtS {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    12,
                                    Type::Bool,
                                    ir::InstructionKind::I32LeS {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    13,
                                    Type::Bool,
                                    ir::InstructionKind::I32GtS {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    14,
                                    Type::Bool,
                                    ir::InstructionKind::I32GeS {
                                        lhs: ir::ValueId(1),
                                        rhs: ir::ValueId(2),
                                    },
                                ),
                                instruction(
                                    15,
                                    Type::I32,
                                    ir::InstructionKind::DirectCall {
                                        callee: ir::FunctionId {
                                            module: ir::ModuleId(0),
                                            declaration: 1,
                                        },
                                        arguments: vec![ir::ValueId(5)],
                                    },
                                ),
                            ],
                            terminators: vec![ir::SpannedTerminator {
                                span,
                                kind: ir::Terminator::Return(ir::ValueId(15)),
                            }],
                        }],
                    },
                    ir::Function {
                        id: ir::FunctionId { module: ir::ModuleId(0), declaration: 1 },
                        entry_export: None,
                        span,
                        parameters: vec![definition(0, Type::I32)],
                        result: Type::I32,
                        blocks: vec![ir::Block {
                            id: ir::BlockId(0),
                            parameters: vec![],
                            instructions: vec![],
                            terminators: vec![ir::SpannedTerminator {
                                span,
                                kind: ir::Terminator::Return(ir::ValueId(0)),
                            }],
                        }],
                    },
                ],
            }],
        };
        zryna_ir::control_flow_v1::verify(program, &sources, file)
            .expect("verified exhaustive operation IR")
    }

    fn call_chain(function_count: usize) -> raw::Program {
        let functions = (0..function_count)
            .map(|index| {
                let operation = if index + 1 == function_count {
                    raw::InstructionKind::I32Literal(1)
                } else {
                    raw::InstructionKind::DirectCall {
                        callee: raw::FunctionId {
                            module: raw::ModuleId(0),
                            declaration: u32::try_from(index + 1).expect("bounded call depth"),
                        },
                        arguments: vec![],
                    }
                };
                function(
                    u32::try_from(index).expect("bounded call depth"),
                    (index == 0).then_some("chain"),
                    vec![],
                    Type::I32,
                    vec![raw::Block {
                        id: raw::BlockId(0),
                        parameters: vec![],
                        instructions: vec![raw::Instruction {
                            result: value(0, Type::I32),
                            kind: operation,
                        }],
                        terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
                    }],
                )
            })
            .collect();
        raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions }],
        }
    }

    fn codes(result: Result<VerifiedProgram, Vec<Diagnostic>>) -> Vec<String> {
        result
            .expect_err("raw mutation must fail")
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect()
    }

    #[test]
    fn seals_typed_cfg_calls_symbols_and_entry_abi() {
        let first = verify(valid_program()).expect("valid M2 native MIR");
        let second = verify(valid_program()).expect("deterministic repeat");
        let functions = first.modules().next().expect("module").functions().collect::<Vec<_>>();
        assert_eq!(first.entry_module().index(), 0);
        assert_eq!(functions[0].internal_symbol(), "zryna_m2_i_m0_f0");
        assert_eq!(functions[1].internal_symbol(), "zryna_m2_i_m0_f1");
        assert_eq!(functions[0].public_export().expect("export").logical_name().as_str(), "choose");
        assert!(functions[1].public_export().is_none());
        assert_eq!(first.scalar_abi().exports().len(), 1);
        assert_eq!(
            second
                .scalar_abi()
                .exports()
                .next()
                .expect("export")
                .native_linux_x86_64_symbol()
                .as_str(),
            first
                .scalar_abi()
                .exports()
                .next()
                .expect("export")
                .native_linux_x86_64_symbol()
                .as_str()
        );
        let entry = functions[0].blocks().next().expect("entry");
        assert!(matches!(entry.terminator().kind(), VerifiedTerminatorKind::Branch { .. }));
        assert!(matches!(
            entry.instructions().next().expect("call").kind(),
            VerifiedInstructionKind::DirectCall { .. }
        ));
    }

    #[test]
    fn lowers_then_independently_reseals_universal_control_flow() {
        let universal = verified_universal_program();
        let native = lower(&universal).expect("native lowering");
        let function =
            native.modules().next().expect("module").functions().next().expect("function");
        assert_eq!(function.internal_symbol(), "zryna_m2_i_m0_f0");
        assert_eq!(function.parameters().collect::<Vec<_>>()[0].1, Type::I32);
        assert_eq!(
            function.public_export().expect("entry export").logical_name().as_str(),
            "identity"
        );
        assert!(matches!(
            function.blocks().next().expect("block").terminator().kind(),
            VerifiedTerminatorKind::Return(_)
        ));
    }

    #[test]
    fn lowering_and_views_cover_every_m2_operation() {
        let native = lower(&verified_all_operations_program()).expect("all operation lowering");
        let function =
            native.modules().next().expect("module").functions().next().expect("function");
        let kinds = function
            .blocks()
            .next()
            .expect("entry")
            .instructions()
            .map(|instruction| match instruction.kind() {
                VerifiedInstructionKind::BoolLiteral(true) => "bool",
                VerifiedInstructionKind::I32Literal(7) => "literal",
                VerifiedInstructionKind::I32Add(_, _) => "add",
                VerifiedInstructionKind::I32Sub(_, _) => "sub",
                VerifiedInstructionKind::I32Mul(_, _) => "mul",
                VerifiedInstructionKind::I32Neg(_) => "neg",
                VerifiedInstructionKind::Eq(_, _) => "eq",
                VerifiedInstructionKind::Ne(_, _) => "ne",
                VerifiedInstructionKind::I32LtS(_, _) => "lt",
                VerifiedInstructionKind::I32LeS(_, _) => "le",
                VerifiedInstructionKind::I32GtS(_, _) => "gt",
                VerifiedInstructionKind::I32GeS(_, _) => "ge",
                VerifiedInstructionKind::DirectCall { callee, arguments }
                    if callee.module().index() == 0
                        && callee.declaration() == 1
                        && arguments.iter().map(ValueIdentity::index).collect::<Vec<_>>()
                            == [5] =>
                {
                    "call"
                }
                _ => panic!("unexpected lowered operation"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                "bool", "literal", "add", "sub", "mul", "neg", "eq", "ne", "lt", "le", "gt", "ge",
                "call"
            ]
        );
        assert!(matches!(
            function.blocks().next().expect("entry").terminator().kind(),
            VerifiedTerminatorKind::Return(value) if value.index() == 15
        ));
    }

    #[test]
    fn seals_cross_module_private_direct_call() {
        let mut entry = function(
            0,
            Some("entry"),
            vec![],
            Type::I32,
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![raw::Instruction {
                    result: value(0, Type::I32),
                    kind: raw::InstructionKind::DirectCall {
                        callee: raw::FunctionId { module: raw::ModuleId(1), declaration: 0 },
                        arguments: vec![],
                    },
                }],
                terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
            }],
        );
        entry.internal_symbol = canonical_internal_symbol(entry.id);
        let mut private = function(
            0,
            None,
            vec![],
            Type::I32,
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![raw::Instruction {
                    result: value(0, Type::I32),
                    kind: raw::InstructionKind::I32Literal(9),
                }],
                terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
            }],
        );
        private.id.module = raw::ModuleId(1);
        private.internal_symbol = canonical_internal_symbol(private.id);
        let verified = verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![
                raw::Module { id: raw::ModuleId(0), functions: vec![entry] },
                raw::Module { id: raw::ModuleId(1), functions: vec![private] },
            ],
        })
        .expect("cross-module private call");
        let call = verified
            .modules()
            .next()
            .expect("entry module")
            .functions()
            .next()
            .expect("entry function")
            .blocks()
            .next()
            .expect("entry block")
            .instructions()
            .next()
            .expect("call")
            .kind();
        assert!(
            matches!(call, VerifiedInstructionKind::DirectCall { callee, .. } if callee.module().index() == 1 && callee.declaration() == 0)
        );
        assert!(
            verified
                .modules()
                .nth(1)
                .expect("dependency")
                .functions()
                .next()
                .expect("private")
                .public_export()
                .is_none()
        );
    }

    #[test]
    fn retains_natural_loop_parallel_swap_and_all_terminator_views() {
        let loop_function = function(
            0,
            Some("rotate"),
            vec![value(0, Type::Bool), value(1, Type::I32), value(2, Type::I32)],
            Type::I32,
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: vec![],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Jump {
                        target: raw::BlockId(1),
                        arguments: vec![raw::ValueId(1), raw::ValueId(2)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: vec![value(3, Type::I32), value(4, Type::I32)],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Branch {
                        condition: raw::ValueId(0),
                        true_target: raw::BlockId(1),
                        true_arguments: vec![raw::ValueId(4), raw::ValueId(3)],
                        false_target: raw::BlockId(2),
                        false_arguments: vec![raw::ValueId(3)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: vec![value(5, Type::I32)],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(5))],
                },
            ],
        );
        let verified = verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions: vec![loop_function] }],
        })
        .expect("valid natural loop");
        let blocks = verified
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .collect::<Vec<_>>();
        assert!(
            matches!(blocks[0].terminator().kind(), VerifiedTerminatorKind::Jump { target, arguments } if target.index() == 1 && arguments.iter().map(ValueIdentity::index).collect::<Vec<_>>() == [1, 2])
        );
        assert!(
            matches!(blocks[1].terminator().kind(), VerifiedTerminatorKind::Branch { true_target, true_arguments, false_target, false_arguments, .. } if true_target.index() == 1 && true_arguments.iter().map(ValueIdentity::index).collect::<Vec<_>>() == [4, 3] && false_target.index() == 2 && false_arguments.iter().map(ValueIdentity::index).collect::<Vec<_>>() == [3])
        );
        assert!(
            matches!(blocks[2].terminator().kind(), VerifiedTerminatorKind::Return(value) if value.index() == 5)
        );
    }

    #[test]
    fn counts_both_same_target_branch_edges() {
        let function = function(
            0,
            Some("sameTarget"),
            vec![value(0, Type::Bool), value(1, Type::I32)],
            Type::I32,
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: vec![],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Branch {
                        condition: raw::ValueId(0),
                        true_target: raw::BlockId(1),
                        true_arguments: vec![raw::ValueId(1)],
                        false_target: raw::BlockId(1),
                        false_arguments: vec![raw::ValueId(1)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: vec![value(2, Type::I32)],
                    instructions: vec![],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(2))],
                },
            ],
        );
        let verified = verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions: vec![function] }],
        })
        .expect("same-target branch preserves two edges");
        assert!(
            matches!(verified.modules().next().expect("module").functions().next().expect("function").blocks().next().expect("entry").terminator().kind(), VerifiedTerminatorKind::Branch { true_target, false_target, .. } if true_target == false_target)
        );
    }

    #[test]
    fn static_call_depth_accepts_exact_and_rejects_first_extra() {
        verify(call_chain(MAX_STATIC_CALL_DEPTH)).expect("exact static call depth");
        assert_eq!(codes(verify(call_chain(MAX_STATIC_CALL_DEPTH + 1))), vec!["ZRYNA-N2201"]);
    }

    #[test]
    fn rejects_noncanonical_symbol_and_convention() {
        let mut program = valid_program();
        program.modules[0].functions[0].internal_symbol = "zryna_m2_i_m0_f00".to_owned();
        program.modules[0].functions[1].convention = raw::CallingConvention::from_code(99);
        let found = codes(verify(program));
        assert!(found.iter().any(|code| code == "ZRYNA-N2103"));
        assert!(found.iter().any(|code| code == "ZRYNA-N2104"));
    }

    #[test]
    fn rejects_type_terminator_and_dominance_claims() {
        let mut condition = valid_program();
        if let raw::Terminator::Branch { condition, .. } =
            &mut condition.modules[0].functions[0].blocks[0].terminators[0]
        {
            *condition = raw::ValueId(1);
        }
        assert!(codes(verify(condition)).iter().any(|code| code == "ZRYNA-N2106"));

        let mut dominance = valid_program();
        dominance.modules[0].functions[0].blocks[2].terminators[0] =
            raw::Terminator::Return(raw::ValueId(3));
        assert!(codes(verify(dominance)).iter().any(|code| code == "ZRYNA-N2109"));

        let mut unit = valid_program();
        unit.modules[0].functions[0].result = Type::Unit;
        assert!(codes(verify(unit)).iter().any(|code| code == "ZRYNA-N2104"));
    }

    #[test]
    fn rejects_malformed_unreachable_and_cyclic_graphs() {
        let mut malformed = valid_program();
        malformed.modules[0].functions[1].blocks[0].terminators.clear();
        assert!(codes(verify(malformed)).iter().any(|code| code == "ZRYNA-N2106"));

        let mut unreachable = valid_program();
        unreachable.modules[0].functions[1].blocks.push(raw::Block {
            id: raw::BlockId(1),
            parameters: vec![value(1, Type::I32)],
            instructions: vec![],
            terminators: vec![raw::Terminator::Return(raw::ValueId(1))],
        });
        assert!(codes(verify(unreachable)).iter().any(|code| code == "ZRYNA-N2110"));

        let mut recursive = valid_program();
        let callee = &mut recursive.modules[0].functions[1];
        callee.blocks[0].instructions.push(raw::Instruction {
            result: value(1, Type::I32),
            kind: raw::InstructionKind::DirectCall {
                callee: callee.id,
                arguments: vec![raw::ValueId(0)],
            },
        });
        callee.blocks[0].terminators[0] = raw::Terminator::Return(raw::ValueId(1));
        assert!(codes(verify(recursive)).iter().any(|code| code == "ZRYNA-N2112"));
    }

    #[test]
    fn rejects_forged_identities_callees_and_dependency_exports() {
        let mut identity = valid_program();
        identity.modules[0].id = raw::ModuleId(1);
        assert!(codes(verify(identity)).iter().any(|code| code == "ZRYNA-N2102"));

        let mut callee = valid_program();
        let raw::InstructionKind::DirectCall { callee: target, .. } =
            &mut callee.modules[0].functions[0].blocks[0].instructions[0].kind
        else {
            panic!("fixture call");
        };
        *target = raw::FunctionId { module: raw::ModuleId(7), declaration: 0 };
        assert!(codes(verify(callee)).iter().any(|code| code == "ZRYNA-N2111"));

        let mut dependency = function(
            0,
            Some("leakedDependency"),
            vec![],
            Type::I32,
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![raw::Instruction {
                    result: value(0, Type::I32),
                    kind: raw::InstructionKind::I32Literal(1),
                }],
                terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
            }],
        );
        dependency.id.module = raw::ModuleId(1);
        dependency.internal_symbol = canonical_internal_symbol(dependency.id);
        let mut program = valid_program();
        program.modules.push(raw::Module { id: raw::ModuleId(1), functions: vec![dependency] });
        assert!(codes(verify(program)).iter().any(|code| code == "ZRYNA-N2105"));
    }

    #[test]
    fn rejects_wrong_call_and_edge_types() {
        let mut call = valid_program();
        let raw::InstructionKind::DirectCall { arguments, .. } =
            &mut call.modules[0].functions[0].blocks[0].instructions[0].kind
        else {
            panic!("fixture call");
        };
        arguments[0] = raw::ValueId(0);
        assert!(codes(verify(call)).iter().any(|code| code == "ZRYNA-N2111"));

        let mut edge = valid_program();
        let raw::Terminator::Branch { true_arguments, .. } =
            &mut edge.modules[0].functions[0].blocks[0].terminators[0]
        else {
            panic!("fixture branch");
        };
        true_arguments[0] = raw::ValueId(0);
        assert!(codes(verify(edge)).iter().any(|code| code == "ZRYNA-N2106"));
    }

    #[test]
    fn rejects_empty_multiple_and_entry_target_terminators() {
        let mut empty = valid_program();
        empty.modules[0].functions[1].blocks.clear();
        assert_eq!(codes(verify(empty)), vec!["ZRYNA-N2107"]);

        let mut multiple = valid_program();
        multiple.modules[0].functions[1].blocks[0]
            .terminators
            .push(raw::Terminator::Return(raw::ValueId(0)));
        assert!(codes(verify(multiple)).iter().any(|code| code == "ZRYNA-N2106"));

        let mut entry_edge = valid_program();
        entry_edge.modules[0].functions[1].blocks[0].terminators[0] =
            raw::Terminator::Jump { target: raw::BlockId(0), arguments: vec![] };
        assert!(codes(verify(entry_edge)).iter().any(|code| code == "ZRYNA-N2106"));
    }

    #[test]
    fn rejects_irreducible_cfg_and_mutual_call_cycle() {
        let mut irreducible = valid_program();
        irreducible.modules[0].functions[0].blocks = vec![
            raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![],
                terminators: vec![raw::Terminator::Branch {
                    condition: raw::ValueId(0),
                    true_target: raw::BlockId(1),
                    true_arguments: vec![],
                    false_target: raw::BlockId(2),
                    false_arguments: vec![],
                }],
            },
            raw::Block {
                id: raw::BlockId(1),
                parameters: vec![],
                instructions: vec![],
                terminators: vec![raw::Terminator::Jump {
                    target: raw::BlockId(3),
                    arguments: vec![],
                }],
            },
            raw::Block {
                id: raw::BlockId(2),
                parameters: vec![],
                instructions: vec![],
                terminators: vec![raw::Terminator::Jump {
                    target: raw::BlockId(3),
                    arguments: vec![],
                }],
            },
            raw::Block {
                id: raw::BlockId(3),
                parameters: vec![],
                instructions: vec![],
                terminators: vec![raw::Terminator::Branch {
                    condition: raw::ValueId(0),
                    true_target: raw::BlockId(1),
                    true_arguments: vec![],
                    false_target: raw::BlockId(2),
                    false_arguments: vec![],
                }],
            },
        ];
        assert!(codes(verify(irreducible)).iter().any(|code| code == "ZRYNA-N2110"));

        let mut first = function(
            0,
            Some("first"),
            vec![],
            Type::I32,
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![raw::Instruction {
                    result: value(0, Type::I32),
                    kind: raw::InstructionKind::DirectCall {
                        callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                        arguments: vec![],
                    },
                }],
                terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
            }],
        );
        let second = function(
            1,
            None,
            vec![],
            Type::I32,
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: vec![],
                instructions: vec![raw::Instruction {
                    result: value(0, Type::I32),
                    kind: raw::InstructionKind::DirectCall {
                        callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
                        arguments: vec![],
                    },
                }],
                terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
            }],
        );
        first.internal_symbol = canonical_internal_symbol(first.id);
        let cycle = raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions: vec![first, second] }],
        };
        assert!(codes(verify(cycle)).iter().any(|code| code == "ZRYNA-N2112"));
    }

    #[test]
    fn aggregate_symbol_and_diagnostic_budgets_are_terminal() {
        let mut bytes = 0usize;
        let mut exact = Errors::default();
        assert!(!add_total(
            &mut bytes,
            MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM,
            MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM,
            "aggregate internal symbol bytes",
            &mut exact,
        ));
        assert!(exact.is_empty());
        assert_eq!(bytes, MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM);

        let mut extra = Errors::default();
        assert!(add_total(
            &mut bytes,
            1,
            MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM,
            "aggregate internal symbol bytes",
            &mut extra,
        ));
        assert!(extra.exhausted());
        assert_eq!(extra.finish()[0].code(), "ZRYNA-N2201");

        let functions = (0..(MAX_DIAGNOSTICS + 32))
            .map(|index| {
                function(
                    u32::try_from(index).expect("diagnostic fixture bound"),
                    None,
                    vec![],
                    Type::Unit,
                    vec![raw::Block {
                        id: raw::BlockId(0),
                        parameters: vec![],
                        instructions: vec![raw::Instruction {
                            result: value(0, Type::I32),
                            kind: raw::InstructionKind::I32Literal(0),
                        }],
                        terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
                    }],
                )
            })
            .collect();
        let diagnostics = verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions }],
        })
        .expect_err("diagnostic flood must fail");
        assert_eq!(diagnostics.len(), MAX_DIAGNOSTICS);
        assert_eq!(diagnostics.last().expect("terminal diagnostic").code(), "ZRYNA-N2202");
    }

    #[test]
    fn terminator_and_export_limits_fail_before_semantic_work() {
        let mut terminators = valid_program();
        terminators.modules[0].functions[1].blocks[0].terminators =
            vec![raw::Terminator::Return(raw::ValueId(0)); MAX_TERMINATORS_PER_FUNCTION + 1];
        assert_eq!(codes(verify(terminators)), vec!["ZRYNA-N2201"]);

        let mut export = valid_program();
        export.modules[0].functions[0].entry_export = Some("x".repeat(MAX_ENTRY_EXPORT_BYTES + 1));
        assert_eq!(codes(verify(export)), vec!["ZRYNA-N2201"]);

        let mut terminator_total = 0usize;
        let mut exact_terminators = Errors::default();
        assert!(!add_total(
            &mut terminator_total,
            MAX_TERMINATORS_PER_PROGRAM,
            MAX_TERMINATORS_PER_PROGRAM,
            "terminator count",
            &mut exact_terminators,
        ));
        assert!(exact_terminators.is_empty());
        let mut extra_terminator = Errors::default();
        assert!(add_total(
            &mut terminator_total,
            1,
            MAX_TERMINATORS_PER_PROGRAM,
            "terminator count",
            &mut extra_terminator,
        ));
        assert_eq!(extra_terminator.finish()[0].code(), "ZRYNA-N2201");

        let mut export_bytes = 0usize;
        let mut exact_exports = Errors::default();
        assert!(!add_total(
            &mut export_bytes,
            MAX_ENTRY_EXPORT_BYTES_PER_PROGRAM,
            MAX_ENTRY_EXPORT_BYTES_PER_PROGRAM,
            "aggregate entry export bytes",
            &mut exact_exports,
        ));
        assert!(exact_exports.is_empty());
        let mut extra_export = Errors::default();
        assert!(add_total(
            &mut export_bytes,
            1,
            MAX_ENTRY_EXPORT_BYTES_PER_PROGRAM,
            "aggregate entry export bytes",
            &mut extra_export,
        ));
        assert_eq!(extra_export.finish()[0].code(), "ZRYNA-N2201");
    }

    #[test]
    fn rejects_parameter_limit_before_semantic_work() {
        let mut program = valid_program();
        program.modules[0].functions[0].parameters = (0..=MAX_PARAMETERS_PER_FUNCTION)
            .map(|index| value(u32::try_from(index).expect("small test bound"), Type::I32))
            .collect();
        assert_eq!(codes(verify(program)), vec!["ZRYNA-N2201"]);
    }

    #[test]
    fn every_m2_budget_row_has_checked_exact_and_first_extra_evidence() {
        let rows = [
            ("modules", MAX_MODULES),
            ("functions per module", MAX_FUNCTIONS_PER_MODULE),
            ("functions per program", MAX_FUNCTIONS_PER_PROGRAM),
            ("parameters per function", MAX_PARAMETERS_PER_FUNCTION),
            ("parameters per program", MAX_PARAMETERS_PER_PROGRAM),
            ("blocks per function", MAX_BLOCKS_PER_FUNCTION),
            ("blocks per program", MAX_BLOCKS_PER_PROGRAM),
            ("block parameters", MAX_BLOCK_PARAMETERS),
            ("values per function", MAX_VALUES_PER_FUNCTION),
            ("values per program", MAX_VALUES_PER_PROGRAM),
            ("CFG edges per function", MAX_CFG_EDGES_PER_FUNCTION),
            ("CFG edges per program", MAX_CFG_EDGES_PER_PROGRAM),
            ("terminators per function", MAX_TERMINATORS_PER_FUNCTION),
            ("terminators per program", MAX_TERMINATORS_PER_PROGRAM),
            ("call edges", MAX_CALL_EDGES),
            ("call arguments", MAX_CALL_ARGUMENTS_PER_PROGRAM),
            ("edge arguments", MAX_EDGE_ARGUMENTS_PER_PROGRAM),
            ("static call depth", MAX_STATIC_CALL_DEPTH),
            ("loop nesting", MAX_LOOP_NESTING),
            ("internal symbol bytes", MAX_INTERNAL_SYMBOL_BYTES),
            ("aggregate internal symbol bytes", MAX_INTERNAL_SYMBOL_BYTES_PER_PROGRAM),
            ("entry export bytes", MAX_ENTRY_EXPORT_BYTES),
            ("aggregate entry export bytes", MAX_ENTRY_EXPORT_BYTES_PER_PROGRAM),
        ];
        for (label, maximum) in rows {
            let mut total = 0usize;
            let mut exact = Errors::default();
            assert!(!add_total(&mut total, maximum, maximum, label, &mut exact), "exact {label}");
            assert!(exact.is_empty(), "exact {label}");
            let mut extra = Errors::default();
            assert!(add_total(&mut total, 1, maximum, label, &mut extra), "+1 {label}");
            assert!(extra.exhausted(), "+1 {label}");
            assert_eq!(extra.finish()[0].code(), "ZRYNA-N2201", "+1 {label}");
        }
    }
}
