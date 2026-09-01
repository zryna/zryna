//! Verified data and ownership IR for the `DataOwnershipV1` profile.
//!
//! This module is isolated from the stable M1 and M2 verifier surfaces. Values in
//! [`crate::data_ownership_v1::raw`] are
//! untrusted claims. Only [`verify`] can bind them to the exact source, layout, scalar-ABI, CFG,
//! ownership, and cleanup authorities exposed through opaque immutable views.

use std::collections::{BTreeSet, VecDeque};
use zryna_abi::{raw as raw_abi, verify_v1};
use zryna_diagnostics::Diagnostic;
use zryna_layout::{
    StorageTarget, TypeCategory, TypeId as LayoutTypeId, TypeUniverseIdentity, VerifiedLayouts,
};
use zryna_source::{FileId, SourceMap, SourceMapIdentity, Span};

/// Maximum modules in one program.
pub const MAX_MODULES: usize = 4_096;
/// Maximum functions in one module.
pub const MAX_FUNCTIONS_PER_MODULE: usize = 4_096;
/// Maximum functions in one program.
pub const MAX_FUNCTIONS_PER_PROGRAM: usize = 16_384;
/// Maximum value parameters in one function.
pub const MAX_PARAMETERS_PER_FUNCTION: usize = 256;
/// Maximum value parameters in one program.
pub const MAX_PARAMETERS_PER_PROGRAM: usize = 262_144;
/// Maximum blocks in one function.
pub const MAX_BLOCKS_PER_FUNCTION: usize = 4_096;
/// Maximum blocks in one program.
pub const MAX_BLOCKS_PER_PROGRAM: usize = 65_536;
/// Maximum parameters on one block.
pub const MAX_BLOCK_PARAMETERS: usize = 256;
/// Maximum values in one function.
pub const MAX_VALUES_PER_FUNCTION: usize = 16_384;
/// Maximum values in one program.
pub const MAX_VALUES_PER_PROGRAM: usize = 262_144;
/// Maximum CFG edges in one function.
pub const MAX_CFG_EDGES_PER_FUNCTION: usize = 8_192;
/// Maximum CFG edges in one program.
pub const MAX_CFG_EDGES_PER_PROGRAM: usize = 131_072;
/// Maximum direct-call edges in one program.
pub const MAX_CALL_EDGES: usize = 65_536;
/// Maximum acyclic direct-call depth.
pub const MAX_STATIC_CALL_DEPTH: usize = 128;
/// Maximum verified loop nesting.
pub const MAX_LOOP_NESTING: usize = 128;
/// Maximum nominal declarations in one program.
pub const MAX_NOMINAL_DECLARATIONS: usize = 4_096;
/// Maximum types in one sealed universe.
pub const MAX_INSTANTIATED_TYPES: usize = 65_536;
/// Maximum aggregate construction operands in one program.
pub const MAX_AGGREGATE_OPERANDS: usize = 262_144;
/// Maximum ownership places in one function.
pub const MAX_PLACES_PER_FUNCTION: usize = 65_536;
/// Maximum ownership transitions in one function.
pub const MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION: usize = 262_144;
/// Maximum simultaneously active borrows in one function.
pub const MAX_ACTIVE_BORROWS_PER_FUNCTION: usize = 16_384;
/// Maximum derived drop actions in one function.
pub const MAX_DROP_ACTIONS_PER_FUNCTION: usize = 262_144;
/// Maximum cleanup plans, including empty plans, in one function.
pub const MAX_CLEANUP_PLANS_PER_FUNCTION: usize = 65_536;
/// Maximum retained diagnostics including the terminal diagnostic.
pub const MAX_DIAGNOSTICS: usize = 256;

/// Exact non-executable runtime contract identity retained by this IR boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum RuntimeContractIdentity {
    /// The frozen ownership runtime ABI v1 contract. Issue #80 supplies a separate sealed,
    /// non-executable declaration authority.
    OwnershipRuntimeV1,
}

/// Untrusted `DataOwnershipV1` claims produced by future semantic lowering.
#[allow(missing_docs)]
pub mod raw {
    use serde::Serialize;
    use zryna_source::{FileId, Span};

    use super::RuntimeContractIdentity;

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct ModuleId(pub u32);
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct FunctionId {
        pub module: ModuleId,
        pub declaration: u32,
    }
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct BlockId(pub u32);
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct ValueId(pub u32);
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct PlaceId(pub u32);
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct BorrowId(pub u32);
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct CleanupPlanId(pub u32);
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub struct TypeId(pub u32);

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    pub struct AuthorityClaims {
        pub runtime: RuntimeContractIdentity,
        pub type_universe: [u8; 32],
        pub linear32_fingerprint: [u8; 32],
        pub linux_x86_64_fingerprint: [u8; 32],
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Program {
        pub authorities: AuthorityClaims,
        pub entry_module: ModuleId,
        pub modules: Vec<Module>,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Module {
        pub id: ModuleId,
        pub source_file: FileId,
        pub data_declarations: u32,
        pub functions: Vec<Function>,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Function {
        pub id: FunctionId,
        pub entry_export: Option<String>,
        pub span: Span,
        pub parameters: Vec<ValueDefinition>,
        pub borrow_parameters: Vec<BorrowParameter>,
        pub result: TypeId,
        pub places: Vec<Place>,
        pub blocks: Vec<Block>,
        pub cleanup_plans: Vec<CleanupPlan>,
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    pub struct ValueDefinition {
        pub id: ValueId,
        pub ty: TypeId,
        pub span: Span,
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    pub struct BorrowParameter {
        pub id: BorrowId,
        pub referent: TypeId,
        pub access: BorrowAccess,
        pub span: Span,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    pub enum BorrowAccess {
        Shared,
        Exclusive,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Place {
        pub id: PlaceId,
        pub ty: TypeId,
        pub span: Span,
        pub kind: PlaceKind,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub enum PlaceKind {
        Parameter(u32),
        Local(u32),
        Temporary(ValueId),
        StructField { base: PlaceId, ordinal: u32 },
        EnumPayload { base: PlaceId, variant: u32 },
        FixedArrayConstant { base: PlaceId, index: u32 },
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct BorrowDefinition {
        pub id: BorrowId,
        pub place: PlaceId,
        pub access: BorrowAccess,
        pub span: Span,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Instruction {
        pub result: Option<ValueDefinition>,
        pub span: Span,
        pub kind: InstructionKind,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub enum CallArgument {
        Value(ValueId),
        Borrow(BorrowId),
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub enum InstructionKind {
        BoolLiteral(bool),
        I32Literal(i32),
        I32Add {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32Sub {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32Mul {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32Neg {
            operand: ValueId,
        },
        Eq {
            lhs: ValueId,
            rhs: ValueId,
        },
        Ne {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32LtS {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32LeS {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32GtS {
            lhs: ValueId,
            rhs: ValueId,
        },
        I32GeS {
            lhs: ValueId,
            rhs: ValueId,
        },
        DirectCall {
            callee: FunctionId,
            arguments: Vec<CallArgument>,
            cleanup: Option<CleanupPlanId>,
        },
        StructConstruct {
            fields: Vec<ValueId>,
            cleanup: Option<CleanupPlanId>,
        },
        EnumConstruct {
            variant: u32,
            payload: Option<ValueId>,
            cleanup: Option<CleanupPlanId>,
        },
        FixedArrayConstruct {
            elements: Vec<ValueId>,
            cleanup: Option<CleanupPlanId>,
        },
        CopyFromPlace {
            place: PlaceId,
        },
        MoveFromPlace {
            place: PlaceId,
        },
        ClonePlace {
            place: PlaceId,
            cleanup: CleanupPlanId,
        },
        InitializePlace {
            place: PlaceId,
            value: ValueId,
        },
        ReplacePlace {
            place: PlaceId,
            value: ValueId,
            cleanup: CleanupPlanId,
        },
        DropPlace {
            place: PlaceId,
        },
        EnumDiscriminant {
            place: PlaceId,
        },
        FixedArrayIndexCopy {
            place: PlaceId,
            index: ValueId,
            cleanup: CleanupPlanId,
        },
        VecIndexCopy {
            place: PlaceId,
            index: ValueId,
            cleanup: CleanupPlanId,
        },
        StringClone {
            place: PlaceId,
            cleanup: CleanupPlanId,
        },
        StringConcat {
            left: PlaceId,
            right: PlaceId,
            cleanup: CleanupPlanId,
        },
        VecConstruct {
            elements: Vec<ValueId>,
            cleanup: CleanupPlanId,
        },
        VecPush {
            vector: PlaceId,
            value: ValueId,
            cleanup: CleanupPlanId,
        },
        SharedConstruct {
            value: ValueId,
            cleanup: CleanupPlanId,
        },
        SharedClone {
            place: PlaceId,
            cleanup: CleanupPlanId,
        },
        WeakDowngrade {
            place: PlaceId,
            cleanup: CleanupPlanId,
        },
        WeakClone {
            place: PlaceId,
            cleanup: CleanupPlanId,
        },
        BeginBorrow(BorrowDefinition),
        BorrowRead {
            borrow: BorrowId,
        },
        BorrowWrite {
            borrow: BorrowId,
            value: ValueId,
        },
        EndBorrow {
            borrow: BorrowId,
        },
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Block {
        pub id: BlockId,
        pub parameters: Vec<ValueDefinition>,
        pub instructions: Vec<Instruction>,
        pub terminators: Vec<SpannedTerminator>,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct Edge {
        pub target: BlockId,
        pub arguments: Vec<ValueId>,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct SpannedTerminator {
        pub span: Span,
        pub kind: Terminator,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub enum Terminator {
        Return { value: ValueId, cleanup: CleanupPlanId },
        Jump(Edge),
        Branch { condition: ValueId, when_true: Edge, when_false: Edge },
        EnumMatch { place: PlaceId, arms: Vec<EnumArm> },
        WeakUpgradeBranch { weak: PlaceId, success: Edge, expired: Edge, cleanup: CleanupPlanId },
        Trap { identity: TrapIdentity, cleanup: CleanupPlanId },
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct EnumArm {
        pub variant: u32,
        pub edge: Edge,
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    pub enum TrapIdentity {
        BoundsV1,
        AllocationV1,
        CapacityV1,
        RefcountV1,
        Utf8V1,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct CleanupPlan {
        pub id: CleanupPlanId,
        pub span: Span,
        pub actions: Vec<DropAction>,
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    pub enum DropAction {
        DropPlace(PlaceId),
    }
}

/// Opaque identity of one verified program authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramIdentity {
    source_map: SourceMapIdentity,
    universe: TypeUniverseIdentity,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque module identity branded by its verified program.
pub struct ModuleIdentity {
    owner: ProgramIdentity,
    index: u32,
}
#[allow(missing_docs)]
impl ModuleIdentity {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque function identity branded by its verified program.
pub struct FunctionIdentity {
    owner: ProgramIdentity,
    module: u32,
    declaration: u32,
}
#[allow(missing_docs)]
impl FunctionIdentity {
    #[must_use]
    pub const fn module(self) -> u32 {
        self.module
    }
    #[must_use]
    pub const fn declaration(self) -> u32 {
        self.declaration
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque block identity branded by its containing function.
pub struct BlockIdentity {
    owner: FunctionIdentity,
    index: u32,
}
#[allow(missing_docs)]
impl BlockIdentity {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque value identity branded by its containing function.
pub struct ValueIdentity {
    owner: FunctionIdentity,
    index: u32,
}
#[allow(missing_docs)]
impl ValueIdentity {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque place identity branded by its containing function.
pub struct PlaceIdentity {
    owner: FunctionIdentity,
    index: u32,
}
#[allow(missing_docs)]
impl PlaceIdentity {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque borrow identity branded by its containing function.
pub struct BorrowIdentity {
    owner: FunctionIdentity,
    index: u32,
}
#[allow(missing_docs)]
impl BorrowIdentity {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Opaque cleanup-plan identity branded by its containing function.
pub struct CleanupPlanIdentity {
    owner: FunctionIdentity,
    index: u32,
}
#[allow(missing_docs)]
impl CleanupPlanIdentity {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Program accepted only after authority, resource, identity, span, type, place, and CFG checks.
///
/// ```compile_fail
/// let _ = zryna_ir::data_ownership_v1::VerifiedProgram { program: todo!() };
/// ```
/// ```compile_fail
/// fn recover_raw(value: &zryna_ir::data_ownership_v1::VerifiedProgram) { let _ = &value.program; }
/// ```
#[derive(Clone, Debug)]
pub struct VerifiedProgram {
    program: raw::Program,
    identity: ProgramIdentity,
    linear32: VerifiedLayouts,
    linux_x86_64: VerifiedLayouts,
    abi: zryna_abi::VerifiedScalarAbiModule,
    abi_indices: Vec<Vec<Option<usize>>>,
}

#[allow(missing_docs)]
impl VerifiedProgram {
    #[must_use]
    pub const fn identity(&self) -> ProgramIdentity {
        self.identity
    }
    #[must_use]
    pub const fn source_map_identity(&self) -> SourceMapIdentity {
        self.identity.source_map
    }
    #[must_use]
    pub const fn type_universe_identity(&self) -> TypeUniverseIdentity {
        self.identity.universe
    }
    #[must_use]
    pub const fn runtime_contract(&self) -> RuntimeContractIdentity {
        RuntimeContractIdentity::OwnershipRuntimeV1
    }
    #[must_use]
    pub const fn linear32_layouts(&self) -> &VerifiedLayouts {
        &self.linear32
    }
    #[must_use]
    pub const fn linux_x86_64_layouts(&self) -> &VerifiedLayouts {
        &self.linux_x86_64
    }
    #[must_use]
    pub const fn scalar_abi(&self) -> &zryna_abi::VerifiedScalarAbiModule {
        &self.abi
    }
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = VerifiedModule<'_>> {
        self.program.modules.iter().enumerate().map(|(index, module)| VerifiedModule {
            owner: self,
            index,
            module,
        })
    }
}

#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified module.
pub struct VerifiedModule<'a> {
    owner: &'a VerifiedProgram,
    index: usize,
    module: &'a raw::Module,
}
#[allow(missing_docs)]
impl<'a> VerifiedModule<'a> {
    #[must_use]
    pub const fn id(self) -> ModuleIdentity {
        ModuleIdentity { owner: self.owner.identity, index: self.module.id.0 }
    }
    #[must_use]
    pub const fn source_file(self) -> FileId {
        self.module.source_file
    }
    #[must_use]
    pub const fn data_declarations(self) -> u32 {
        self.module.data_declarations
    }
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

#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified function.
pub struct VerifiedFunction<'a> {
    owner: &'a VerifiedProgram,
    module_index: usize,
    function_index: usize,
    function: &'a raw::Function,
}
#[allow(missing_docs)]
impl<'a> VerifiedFunction<'a> {
    #[must_use]
    pub const fn id(self) -> FunctionIdentity {
        FunctionIdentity {
            owner: self.owner.identity,
            module: self.function.id.module.0,
            declaration: self.function.id.declaration,
        }
    }
    #[must_use]
    pub fn public_export(self) -> Option<zryna_abi::VerifiedScalarExport<'a>> {
        self.owner.abi_indices[self.module_index][self.function_index]
            .and_then(|index| self.owner.abi.exports().nth(index))
    }
    #[must_use]
    /// # Panics
    /// Panics only if retained verified layout authority is internally corrupted.
    pub fn result_type(self) -> LayoutTypeId {
        layout_type(&self.owner.linear32, self.function.result).expect("verified result type").id()
    }
    #[must_use]
    /// # Panics
    /// Panics only if retained verified layout authority is internally corrupted.
    pub fn parameters(self) -> impl ExactSizeIterator<Item = VerifiedValueDefinition> + 'a {
        let owner = self.id();
        let layouts = &self.owner.linear32;
        self.function.parameters.iter().map(move |value| VerifiedValueDefinition {
            id: ValueIdentity { owner, index: value.id.0 },
            ty: layout_type(layouts, value.ty).expect("verified parameter type").id(),
            span: value.span,
        })
    }
    #[must_use]
    /// # Panics
    /// Panics only if retained verified layout authority is internally corrupted.
    pub fn borrow_parameters(self) -> impl ExactSizeIterator<Item = VerifiedBorrowParameter> + 'a {
        let owner = self.id();
        let layouts = &self.owner.linear32;
        self.function.borrow_parameters.iter().map(move |borrow| VerifiedBorrowParameter {
            id: BorrowIdentity { owner, index: borrow.id.0 },
            referent: layout_type(layouts, borrow.referent).expect("verified borrow referent").id(),
            access: borrow.access.into(),
            span: borrow.span,
        })
    }
    #[must_use]
    pub fn blocks(self) -> impl ExactSizeIterator<Item = VerifiedBlock<'a>> {
        self.function.blocks.iter().enumerate().map(move |(block_index, block)| VerifiedBlock {
            function: self,
            block_index,
            block,
        })
    }
    #[must_use]
    pub fn places(self) -> impl ExactSizeIterator<Item = VerifiedPlace<'a>> {
        self.function.places.iter().map(move |place| VerifiedPlace { function: self, place })
    }
    #[must_use]
    pub fn cleanup_plans(self) -> impl ExactSizeIterator<Item = VerifiedCleanupPlan<'a>> {
        self.function
            .cleanup_plans
            .iter()
            .map(move |plan| VerifiedCleanupPlan { function: self, plan })
    }
}

#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified basic block.
pub struct VerifiedBlock<'a> {
    function: VerifiedFunction<'a>,
    block_index: usize,
    block: &'a raw::Block,
}
#[allow(missing_docs)]
impl<'a> VerifiedBlock<'a> {
    #[must_use]
    pub const fn id(self) -> BlockIdentity {
        BlockIdentity { owner: self.function.id(), index: self.block.id.0 }
    }
    #[must_use]
    pub fn instructions(self) -> impl ExactSizeIterator<Item = VerifiedInstruction<'a>> {
        self.block.instructions.iter().enumerate().map(move |(instruction_index, instruction)| {
            VerifiedInstruction {
                function: self.function,
                block_index: self.block_index,
                instruction_index,
                instruction,
            }
        })
    }
    #[must_use]
    /// # Panics
    /// Panics only if retained verified layout authority is internally corrupted.
    pub fn parameters(self) -> impl ExactSizeIterator<Item = VerifiedValueDefinition> + 'a {
        let owner = self.function.id();
        let layouts = &self.function.owner.linear32;
        self.block.parameters.iter().map(move |value| VerifiedValueDefinition {
            id: ValueIdentity { owner, index: value.id.0 },
            ty: layout_type(layouts, value.ty).expect("verified block parameter type").id(),
            span: value.span,
        })
    }
    #[must_use]
    pub fn terminator(self) -> VerifiedTerminator<'a> {
        VerifiedTerminator {
            function: self.function,
            block_index: self.block_index,
            terminator: &self.block.terminators[0],
        }
    }
}
/// Sealed value definition exposed by a verified function or block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedValueDefinition {
    id: ValueIdentity,
    ty: LayoutTypeId,
    span: Span,
}
#[allow(missing_docs)]
impl VerifiedValueDefinition {
    #[must_use]
    pub const fn id(self) -> ValueIdentity {
        self.id
    }
    #[must_use]
    pub const fn ty(self) -> LayoutTypeId {
        self.ty
    }
    #[must_use]
    pub const fn span(self) -> Span {
        self.span
    }
}
/// Sealed borrow access mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum VerifiedBorrowAccess {
    Shared,
    Exclusive,
}
impl From<raw::BorrowAccess> for VerifiedBorrowAccess {
    fn from(value: raw::BorrowAccess) -> Self {
        match value {
            raw::BorrowAccess::Shared => Self::Shared,
            raw::BorrowAccess::Exclusive => Self::Exclusive,
        }
    }
}
/// Sealed borrow parameter signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedBorrowParameter {
    id: BorrowIdentity,
    referent: LayoutTypeId,
    access: VerifiedBorrowAccess,
    span: Span,
}
#[allow(missing_docs)]
impl VerifiedBorrowParameter {
    #[must_use]
    pub const fn id(self) -> BorrowIdentity {
        self.id
    }
    #[must_use]
    pub const fn referent(self) -> LayoutTypeId {
        self.referent
    }
    #[must_use]
    pub const fn access(self) -> VerifiedBorrowAccess {
        self.access
    }
    #[must_use]
    pub const fn span(self) -> Span {
        self.span
    }
}
#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified ownership place.
pub struct VerifiedPlace<'a> {
    function: VerifiedFunction<'a>,
    place: &'a raw::Place,
}
#[allow(missing_docs)]
impl VerifiedPlace<'_> {
    #[must_use]
    pub const fn id(self) -> PlaceIdentity {
        PlaceIdentity { owner: self.function.id(), index: self.place.id.0 }
    }
    /// # Panics
    ///
    /// Panics only if the verifier's retained sealed type authority is internally corrupted.
    #[must_use]
    pub fn ty(self) -> LayoutTypeId {
        layout_type(&self.function.owner.linear32, self.place.ty).expect("verified type").id()
    }
    #[must_use]
    pub const fn span(self) -> Span {
        self.place.span
    }
    #[must_use]
    pub const fn kind(self) -> VerifiedPlaceKind {
        let owner = self.function.id();
        match self.place.kind {
            raw::PlaceKind::Parameter(index) => VerifiedPlaceKind::Parameter(index),
            raw::PlaceKind::Local(index) => VerifiedPlaceKind::Local(index),
            raw::PlaceKind::Temporary(value) => {
                VerifiedPlaceKind::Temporary(ValueIdentity { owner, index: value.0 })
            }
            raw::PlaceKind::StructField { base, ordinal } => VerifiedPlaceKind::StructField {
                base: PlaceIdentity { owner, index: base.0 },
                ordinal,
            },
            raw::PlaceKind::EnumPayload { base, variant } => VerifiedPlaceKind::EnumPayload {
                base: PlaceIdentity { owner, index: base.0 },
                variant,
            },
            raw::PlaceKind::FixedArrayConstant { base, index } => {
                VerifiedPlaceKind::FixedArrayConstant {
                    base: PlaceIdentity { owner, index: base.0 },
                    index,
                }
            }
        }
    }
}
/// Complete sealed place-root or projection identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum VerifiedPlaceKind {
    Parameter(u32),
    Local(u32),
    Temporary(ValueIdentity),
    StructField { base: PlaceIdentity, ordinal: u32 },
    EnumPayload { base: PlaceIdentity, variant: u32 },
    FixedArrayConstant { base: PlaceIdentity, index: u32 },
}
#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified instruction.
pub struct VerifiedInstruction<'a> {
    function: VerifiedFunction<'a>,
    block_index: usize,
    instruction_index: usize,
    instruction: &'a raw::Instruction,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
/// Closed verified instruction opcode.
pub enum VerifiedInstructionKind {
    BoolLiteral,
    I32Literal,
    I32Add,
    I32Sub,
    I32Mul,
    I32Neg,
    Eq,
    Ne,
    I32LtS,
    I32LeS,
    I32GtS,
    I32GeS,
    DirectCall,
    StructConstruct,
    EnumConstruct,
    FixedArrayConstruct,
    CopyFromPlace,
    MoveFromPlace,
    ClonePlace,
    InitializePlace,
    ReplacePlace,
    DropPlace,
    EnumDiscriminant,
    FixedArrayIndexCopy,
    VecIndexCopy,
    StringClone,
    StringConcat,
    VecConstruct,
    VecPush,
    SharedConstruct,
    SharedClone,
    WeakDowngrade,
    WeakClone,
    BeginBorrow,
    BorrowRead,
    BorrowWrite,
    EndBorrow,
}
#[allow(missing_docs)]
impl<'a> VerifiedInstruction<'a> {
    #[must_use]
    pub fn result(self) -> Option<ValueIdentity> {
        self.instruction
            .result
            .map(|result| ValueIdentity { owner: self.function.id(), index: result.id.0 })
    }
    #[must_use]
    pub const fn span(self) -> Span {
        self.instruction.span
    }
    #[must_use]
    pub const fn kind(self) -> VerifiedInstructionKind {
        use raw::InstructionKind as I;
        match &self.instruction.kind {
            I::BoolLiteral(_) => VerifiedInstructionKind::BoolLiteral,
            I::I32Literal(_) => VerifiedInstructionKind::I32Literal,
            I::I32Add { .. } => VerifiedInstructionKind::I32Add,
            I::I32Sub { .. } => VerifiedInstructionKind::I32Sub,
            I::I32Mul { .. } => VerifiedInstructionKind::I32Mul,
            I::I32Neg { .. } => VerifiedInstructionKind::I32Neg,
            I::Eq { .. } => VerifiedInstructionKind::Eq,
            I::Ne { .. } => VerifiedInstructionKind::Ne,
            I::I32LtS { .. } => VerifiedInstructionKind::I32LtS,
            I::I32LeS { .. } => VerifiedInstructionKind::I32LeS,
            I::I32GtS { .. } => VerifiedInstructionKind::I32GtS,
            I::I32GeS { .. } => VerifiedInstructionKind::I32GeS,
            I::DirectCall { .. } => VerifiedInstructionKind::DirectCall,
            I::StructConstruct { .. } => VerifiedInstructionKind::StructConstruct,
            I::EnumConstruct { .. } => VerifiedInstructionKind::EnumConstruct,
            I::FixedArrayConstruct { .. } => VerifiedInstructionKind::FixedArrayConstruct,
            I::CopyFromPlace { .. } => VerifiedInstructionKind::CopyFromPlace,
            I::MoveFromPlace { .. } => VerifiedInstructionKind::MoveFromPlace,
            I::ClonePlace { .. } => VerifiedInstructionKind::ClonePlace,
            I::InitializePlace { .. } => VerifiedInstructionKind::InitializePlace,
            I::ReplacePlace { .. } => VerifiedInstructionKind::ReplacePlace,
            I::DropPlace { .. } => VerifiedInstructionKind::DropPlace,
            I::EnumDiscriminant { .. } => VerifiedInstructionKind::EnumDiscriminant,
            I::FixedArrayIndexCopy { .. } => VerifiedInstructionKind::FixedArrayIndexCopy,
            I::VecIndexCopy { .. } => VerifiedInstructionKind::VecIndexCopy,
            I::StringClone { .. } => VerifiedInstructionKind::StringClone,
            I::StringConcat { .. } => VerifiedInstructionKind::StringConcat,
            I::VecConstruct { .. } => VerifiedInstructionKind::VecConstruct,
            I::VecPush { .. } => VerifiedInstructionKind::VecPush,
            I::SharedConstruct { .. } => VerifiedInstructionKind::SharedConstruct,
            I::SharedClone { .. } => VerifiedInstructionKind::SharedClone,
            I::WeakDowngrade { .. } => VerifiedInstructionKind::WeakDowngrade,
            I::WeakClone { .. } => VerifiedInstructionKind::WeakClone,
            I::BeginBorrow(_) => VerifiedInstructionKind::BeginBorrow,
            I::BorrowRead { .. } => VerifiedInstructionKind::BorrowRead,
            I::BorrowWrite { .. } => VerifiedInstructionKind::BorrowWrite,
            I::EndBorrow { .. } => VerifiedInstructionKind::EndBorrow,
        }
    }
    #[must_use]
    pub fn result_type(self) -> Option<LayoutTypeId> {
        self.instruction
            .result
            .and_then(|result| layout_type(&self.function.owner.linear32, result.ty))
            .map(zryna_layout::VerifiedType::id)
    }
    #[must_use]
    pub fn value_operands(self) -> impl ExactSizeIterator<Item = ValueIdentity> {
        let owner = self.function.id();
        instruction_operands(&self.instruction.kind)
            .into_iter()
            .map(move |value| ValueIdentity { owner, index: value.0 })
    }
    #[must_use]
    pub fn place_operands(self) -> impl ExactSizeIterator<Item = PlaceIdentity> {
        let owner = self.function.id();
        instruction_place_operands(&self.instruction.kind)
            .into_iter()
            .map(move |place| PlaceIdentity { owner, index: place.0 })
    }
    #[must_use]
    pub fn cleanup(self) -> Option<CleanupPlanIdentity> {
        instruction_cleanup(&self.instruction.kind)
            .map(|plan| CleanupPlanIdentity { owner: self.function.id(), index: plan.0 })
    }
    #[must_use]
    pub const fn bool_literal(self) -> Option<bool> {
        if let raw::InstructionKind::BoolLiteral(value) = &self.instruction.kind {
            Some(*value)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn i32_literal(self) -> Option<i32> {
        if let raw::InstructionKind::I32Literal(value) = &self.instruction.kind {
            Some(*value)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn callee(self) -> Option<FunctionIdentity> {
        if let raw::InstructionKind::DirectCall { callee, .. } = &self.instruction.kind {
            Some(FunctionIdentity {
                owner: self.function.owner.identity,
                module: callee.module.0,
                declaration: callee.declaration,
            })
        } else {
            None
        }
    }
    #[must_use]
    pub fn call_arguments(self) -> impl ExactSizeIterator<Item = VerifiedCallArgument> + 'a {
        let owner = self.function.id();
        let arguments = match &self.instruction.kind {
            raw::InstructionKind::DirectCall { arguments, .. } => &arguments[..],
            _ => &[],
        };
        arguments.iter().map(move |argument| match argument {
            raw::CallArgument::Value(value) => {
                VerifiedCallArgument::Value(ValueIdentity { owner, index: value.0 })
            }
            raw::CallArgument::Borrow(borrow) => {
                VerifiedCallArgument::Borrow(BorrowIdentity { owner, index: borrow.0 })
            }
        })
    }
    #[must_use]
    pub const fn variant(self) -> Option<u32> {
        match &self.instruction.kind {
            raw::InstructionKind::EnumConstruct { variant, .. } => Some(*variant),
            _ => None,
        }
    }
    #[must_use]
    pub const fn borrow(self) -> Option<BorrowIdentity> {
        let id = match &self.instruction.kind {
            raw::InstructionKind::BeginBorrow(definition) => definition.id,
            raw::InstructionKind::BorrowRead { borrow }
            | raw::InstructionKind::BorrowWrite { borrow, .. }
            | raw::InstructionKind::EndBorrow { borrow } => *borrow,
            _ => return None,
        };
        Some(BorrowIdentity { owner: self.function.id(), index: id.0 })
    }
    #[must_use]
    pub const fn borrow_access(self) -> Option<VerifiedBorrowAccess> {
        match &self.instruction.kind {
            raw::InstructionKind::BeginBorrow(definition) => Some(match definition.access {
                raw::BorrowAccess::Shared => VerifiedBorrowAccess::Shared,
                raw::BorrowAccess::Exclusive => VerifiedBorrowAccess::Exclusive,
            }),
            _ => None,
        }
    }
    #[must_use]
    pub fn derived_drop_actions(self) -> impl ExactSizeIterator<Item = VerifiedDropAction> {
        let actions = instruction_cleanup(&self.instruction.kind)
            .and_then(|plan| {
                derive_state_before(
                    self.function.function,
                    &self.function.owner.linear32,
                    self.block_index,
                    self.instruction_index,
                )
                .map(|(states, variants)| {
                    sealed_drop_actions(
                        self.function.id(),
                        self.function.function,
                        plan,
                        &states,
                        &variants,
                    )
                })
            })
            .unwrap_or_default();
        actions.into_iter()
    }
}
/// One sealed direct-call argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum VerifiedCallArgument {
    Value(ValueIdentity),
    Borrow(BorrowIdentity),
}
#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified terminator.
pub struct VerifiedTerminator<'a> {
    function: VerifiedFunction<'a>,
    block_index: usize,
    terminator: &'a raw::SpannedTerminator,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
/// Closed verified terminator opcode.
pub enum VerifiedTerminatorKind {
    Return,
    Jump,
    Branch,
    EnumMatch,
    WeakUpgradeBranch,
    Trap,
}
/// One verified CFG edge with owner-branded target and arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedEdge {
    target: BlockIdentity,
    arguments: Vec<ValueIdentity>,
}
#[allow(missing_docs)]
impl VerifiedEdge {
    #[must_use]
    pub const fn target(&self) -> BlockIdentity {
        self.target
    }
    #[must_use]
    pub fn arguments(&self) -> impl ExactSizeIterator<Item = ValueIdentity> + '_ {
        self.arguments.iter().copied()
    }
}
#[allow(missing_docs)]
impl<'a> VerifiedTerminator<'a> {
    #[must_use]
    pub const fn span(self) -> Span {
        self.terminator.span
    }
    #[must_use]
    pub const fn kind(self) -> VerifiedTerminatorKind {
        match &self.terminator.kind {
            raw::Terminator::Return { .. } => VerifiedTerminatorKind::Return,
            raw::Terminator::Jump(_) => VerifiedTerminatorKind::Jump,
            raw::Terminator::Branch { .. } => VerifiedTerminatorKind::Branch,
            raw::Terminator::EnumMatch { .. } => VerifiedTerminatorKind::EnumMatch,
            raw::Terminator::WeakUpgradeBranch { .. } => VerifiedTerminatorKind::WeakUpgradeBranch,
            raw::Terminator::Trap { .. } => VerifiedTerminatorKind::Trap,
        }
    }
    #[must_use]
    pub const fn owner(self) -> FunctionIdentity {
        self.function.id()
    }
    #[must_use]
    pub fn value_operands(self) -> impl ExactSizeIterator<Item = ValueIdentity> {
        let owner = self.function.id();
        terminator_operands(&self.terminator.kind)
            .into_iter()
            .map(move |value| ValueIdentity { owner, index: value.0 })
    }
    #[must_use]
    pub fn place_operands(self) -> impl ExactSizeIterator<Item = PlaceIdentity> {
        let owner = self.function.id();
        terminator_place_operands(&self.terminator.kind)
            .into_iter()
            .map(move |place| PlaceIdentity { owner, index: place.0 })
    }
    #[must_use]
    pub fn cleanup(self) -> Option<CleanupPlanIdentity> {
        let (raw::Terminator::Return { cleanup, .. }
        | raw::Terminator::WeakUpgradeBranch { cleanup, .. }
        | raw::Terminator::Trap { cleanup, .. }) = &self.terminator.kind
        else {
            return None;
        };
        let plan = cleanup;
        Some(CleanupPlanIdentity { owner: self.function.id(), index: plan.0 })
    }
    #[must_use]
    pub const fn trap_identity(self) -> Option<VerifiedTrapIdentity> {
        let raw::Terminator::Trap { identity, .. } = &self.terminator.kind else { return None };
        Some(match identity {
            raw::TrapIdentity::BoundsV1 => VerifiedTrapIdentity::BoundsV1,
            raw::TrapIdentity::AllocationV1 => VerifiedTrapIdentity::AllocationV1,
            raw::TrapIdentity::CapacityV1 => VerifiedTrapIdentity::CapacityV1,
            raw::TrapIdentity::RefcountV1 => VerifiedTrapIdentity::RefcountV1,
            raw::TrapIdentity::Utf8V1 => VerifiedTrapIdentity::Utf8V1,
        })
    }
    #[must_use]
    pub fn enum_arms(self) -> impl ExactSizeIterator<Item = VerifiedEnumArm> + 'a {
        let owner = self.function.id();
        let arms = match &self.terminator.kind {
            raw::Terminator::EnumMatch { arms, .. } => &arms[..],
            _ => &[],
        };
        arms.iter().map(move |arm| VerifiedEnumArm {
            variant: arm.variant,
            edge: verified_edge(owner, &arm.edge),
        })
    }
    #[must_use]
    pub fn branch_edges(self) -> Option<(VerifiedEdge, VerifiedEdge)> {
        let raw::Terminator::Branch { when_true, when_false, .. } = &self.terminator.kind else {
            return None;
        };
        let owner = self.function.id();
        Some((verified_edge(owner, when_true), verified_edge(owner, when_false)))
    }
    #[must_use]
    pub fn weak_upgrade_edges(self) -> Option<(VerifiedEdge, VerifiedEdge)> {
        let raw::Terminator::WeakUpgradeBranch { success, expired, .. } = &self.terminator.kind
        else {
            return None;
        };
        let owner = self.function.id();
        Some((verified_edge(owner, success), verified_edge(owner, expired)))
    }
    #[must_use]
    pub fn edges(self) -> impl ExactSizeIterator<Item = VerifiedEdge> {
        let owner = self.function.id();
        terminator_edges(&self.terminator.kind).into_iter().map(move |edge| VerifiedEdge {
            target: BlockIdentity { owner, index: edge.target.0 },
            arguments: edge
                .arguments
                .iter()
                .map(|value| ValueIdentity { owner, index: value.0 })
                .collect(),
        })
    }
    #[must_use]
    pub fn derived_drop_actions(self) -> impl ExactSizeIterator<Item = VerifiedDropAction> {
        let actions = self.cleanup().and_then(|plan| {
            derive_state_before(
                self.function.function,
                &self.function.owner.linear32,
                self.block_index,
                self.function.function.blocks[self.block_index].instructions.len(),
            )
            .map(|(states, variants)| {
                sealed_drop_actions(
                    self.function.id(),
                    self.function.function,
                    raw::CleanupPlanId(plan.index),
                    &states,
                    &variants,
                )
            })
        });
        actions.unwrap_or_default().into_iter()
    }
}
/// Executable sealed cleanup action derived for one exact cleanup site.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDropAction {
    root: PlaceIdentity,
    moved_projections: Vec<PlaceIdentity>,
    initialized_projections: Vec<PlaceIdentity>,
    active_variant: Option<u32>,
}
#[allow(missing_docs)]
impl VerifiedDropAction {
    #[must_use]
    pub const fn root(&self) -> PlaceIdentity {
        self.root
    }
    #[must_use]
    pub fn moved_projections(&self) -> impl ExactSizeIterator<Item = PlaceIdentity> + '_ {
        self.moved_projections.iter().copied()
    }
    #[must_use]
    pub fn initialized_projections(&self) -> impl ExactSizeIterator<Item = PlaceIdentity> + '_ {
        self.initialized_projections.iter().copied()
    }
    #[must_use]
    pub const fn active_variant(&self) -> Option<u32> {
        self.active_variant
    }
}
/// Sealed trap ABI identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum VerifiedTrapIdentity {
    BoundsV1,
    AllocationV1,
    CapacityV1,
    RefcountV1,
    Utf8V1,
}
/// One role-preserving exhaustive enum arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedEnumArm {
    variant: u32,
    edge: VerifiedEdge,
}
#[allow(missing_docs)]
impl VerifiedEnumArm {
    #[must_use]
    pub const fn variant(&self) -> u32 {
        self.variant
    }
    #[must_use]
    pub const fn edge(&self) -> &VerifiedEdge {
        &self.edge
    }
}

fn verified_edge(owner: FunctionIdentity, edge: &raw::Edge) -> VerifiedEdge {
    VerifiedEdge {
        target: BlockIdentity { owner, index: edge.target.0 },
        arguments: edge
            .arguments
            .iter()
            .map(|value| ValueIdentity { owner, index: value.0 })
            .collect(),
    }
}
#[derive(Clone, Copy, Debug)]
/// Immutable view of one verified cleanup plan.
pub struct VerifiedCleanupPlan<'a> {
    function: VerifiedFunction<'a>,
    plan: &'a raw::CleanupPlan,
}
#[allow(missing_docs)]
impl<'a> VerifiedCleanupPlan<'a> {
    #[must_use]
    pub const fn id(self) -> CleanupPlanIdentity {
        CleanupPlanIdentity { owner: self.function.id(), index: self.plan.id.0 }
    }
    #[must_use]
    pub fn actions(self) -> impl ExactSizeIterator<Item = PlaceIdentity> + 'a {
        self.plan.actions.iter().map(move |action| match action {
            raw::DropAction::DropPlace(place) => {
                PlaceIdentity { owner: self.function.id(), index: place.0 }
            }
        })
    }
    #[must_use]
    pub const fn span(self) -> Span {
        self.plan.span
    }
}

/// Verifies one raw M3 program against exact source and dual-target layout authorities.
///
/// # Errors
///
/// Returns deterministic bounded diagnostics and no partial authority when any claim fails.
pub fn verify(
    program: raw::Program,
    sources: &SourceMap,
    expected_entry: FileId,
    linear32: VerifiedLayouts,
    linux_x86_64: VerifiedLayouts,
) -> Result<VerifiedProgram, Vec<Diagnostic>> {
    let mut errors = Errors::default();
    preflight(&program, &linear32, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    verify_authorities(&program, sources, expected_entry, &linear32, &linux_x86_64, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    verify_structure(&program, sources, &linear32, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    verify_calls(&program, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let (abi, abi_indices) = verify_public_abi(&program, &linear32, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let Some(abi) = abi else {
        return Err(vec![error(
            "ZRYNA-I3202",
            "DataOwnershipV1 verifier could not seal scalar ABI authority",
            "report the smallest reproducible compiler invariant failure",
        )]);
    };
    let identity =
        ProgramIdentity { source_map: sources.identity(), universe: linear32.universe_identity() };
    Ok(VerifiedProgram { program, identity, linear32, linux_x86_64, abi, abi_indices })
}

#[allow(clippy::too_many_lines)]
fn preflight(program: &raw::Program, layouts: &VerifiedLayouts, errors: &mut Errors) {
    if program.modules.is_empty() || program.modules.len() > MAX_MODULES {
        errors.limit("module count", MAX_MODULES);
        return;
    }
    if layouts.types().len() > MAX_INSTANTIATED_TYPES {
        errors.limit("instantiated type count", MAX_INSTANTIATED_TYPES);
        return;
    }
    let mut functions = 0usize;
    let mut parameters = 0usize;
    let mut blocks = 0usize;
    let mut values = 0usize;
    let mut edges = 0usize;
    let mut aggregate_operands = 0usize;
    let mut nominals = 0usize;
    let mut calls = 0usize;
    for module in &program.modules {
        nominals = checked_add(
            nominals,
            module.data_declarations as usize,
            "nominal declaration count",
            errors,
        );
        if nominals > MAX_NOMINAL_DECLARATIONS {
            errors.limit("nominal declaration count", MAX_NOMINAL_DECLARATIONS);
            return;
        }
        if module.functions.len() > MAX_FUNCTIONS_PER_MODULE {
            errors.limit("functions per module", MAX_FUNCTIONS_PER_MODULE);
            return;
        }
        functions = checked_add(functions, module.functions.len(), "function count", errors);
        if functions > MAX_FUNCTIONS_PER_PROGRAM {
            errors.limit("function count", MAX_FUNCTIONS_PER_PROGRAM);
            return;
        }
        for function in &module.functions {
            if function.parameters.len() > MAX_PARAMETERS_PER_FUNCTION {
                errors.limit("parameters per function", MAX_PARAMETERS_PER_FUNCTION);
                return;
            }
            if function.borrow_parameters.len() > MAX_ACTIVE_BORROWS_PER_FUNCTION {
                errors.limit("active borrows per function", MAX_ACTIVE_BORROWS_PER_FUNCTION);
                return;
            }
            parameters =
                checked_add(parameters, function.parameters.len(), "parameter count", errors);
            if parameters > MAX_PARAMETERS_PER_PROGRAM {
                errors.limit("parameter count", MAX_PARAMETERS_PER_PROGRAM);
                return;
            }
            if function.places.len() > MAX_PLACES_PER_FUNCTION {
                errors.limit("places per function", MAX_PLACES_PER_FUNCTION);
                return;
            }
            if function.cleanup_plans.len() > MAX_CLEANUP_PLANS_PER_FUNCTION {
                errors.limit("cleanup plans per function", MAX_CLEANUP_PLANS_PER_FUNCTION);
                return;
            }
            if function.blocks.is_empty() || function.blocks.len() > MAX_BLOCKS_PER_FUNCTION {
                errors.limit("blocks per function", MAX_BLOCKS_PER_FUNCTION);
                return;
            }
            blocks = checked_add(blocks, function.blocks.len(), "block count", errors);
            if blocks > MAX_BLOCKS_PER_PROGRAM {
                errors.limit("block count", MAX_BLOCKS_PER_PROGRAM);
                return;
            }
            let mut function_values = function.parameters.len();
            let mut transitions = 0usize;
            let mut function_edges = 0usize;
            let mut drops = 0usize;
            let mut peak_borrows = function.borrow_parameters.len();
            for plan in &function.cleanup_plans {
                drops = checked_add(drops, plan.actions.len(), "drop action count", errors);
            }
            if drops > MAX_DROP_ACTIONS_PER_FUNCTION {
                errors.limit("drop actions per function", MAX_DROP_ACTIONS_PER_FUNCTION);
                return;
            }
            for block in &function.blocks {
                let mut active_borrows = function.borrow_parameters.len();
                if block.parameters.len() > MAX_BLOCK_PARAMETERS {
                    errors.limit("block parameter count", MAX_BLOCK_PARAMETERS);
                    return;
                }
                function_values = checked_add(
                    function_values,
                    block.parameters.len()
                        + block.instructions.iter().filter(|i| i.result.is_some()).count(),
                    "value count",
                    errors,
                );
                if function_values > MAX_VALUES_PER_FUNCTION {
                    errors.limit("values per function", MAX_VALUES_PER_FUNCTION);
                    return;
                }
                transitions = checked_add(
                    transitions,
                    block.instructions.len(),
                    "ownership transition count",
                    errors,
                );
                for instruction in &block.instructions {
                    match instruction.kind {
                        raw::InstructionKind::BeginBorrow(_) => {
                            active_borrows = active_borrows.saturating_add(1);
                            peak_borrows = peak_borrows.max(active_borrows);
                        }
                        raw::InstructionKind::EndBorrow { .. } => {
                            active_borrows = active_borrows.saturating_sub(1);
                        }
                        _ => {}
                    }
                    aggregate_operands = checked_add(
                        aggregate_operands,
                        aggregate_operand_count(&instruction.kind),
                        "aggregate operand count",
                        errors,
                    );
                    if matches!(instruction.kind, raw::InstructionKind::DirectCall { .. }) {
                        calls = checked_add(calls, 1, "call edge count", errors);
                    }
                }
                if transitions > MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION {
                    errors.limit(
                        "ownership transitions per function",
                        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                    );
                    return;
                }
                if block.terminators.len() == 1 {
                    function_edges = checked_add(
                        function_edges,
                        terminator_edge_count(&block.terminators[0].kind),
                        "CFG edges per function",
                        errors,
                    );
                }
            }
            if peak_borrows > MAX_ACTIVE_BORROWS_PER_FUNCTION {
                errors.limit(
                    "simultaneously active borrows per function",
                    MAX_ACTIVE_BORROWS_PER_FUNCTION,
                );
                return;
            }
            if function_edges > MAX_CFG_EDGES_PER_FUNCTION {
                errors.limit("CFG edges per function", MAX_CFG_EDGES_PER_FUNCTION);
                return;
            }
            values = checked_add(values, function_values, "value count", errors);
            edges = checked_add(edges, function_edges, "CFG edge count", errors);
            if values > MAX_VALUES_PER_PROGRAM
                || edges > MAX_CFG_EDGES_PER_PROGRAM
                || aggregate_operands > MAX_AGGREGATE_OPERANDS
                || calls > MAX_CALL_EDGES
            {
                errors.limit("program IR count", MAX_VALUES_PER_PROGRAM);
                return;
            }
        }
    }
}

fn verify_authorities(
    program: &raw::Program,
    sources: &SourceMap,
    expected_entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    errors: &mut Errors,
) {
    let claims = program.authorities;
    if claims.runtime != RuntimeContractIdentity::OwnershipRuntimeV1
        || linear.source_map_identity() != sources.identity()
        || linux.source_map_identity() != sources.identity()
        || linear.target() != StorageTarget::Linear32V1
        || linux.target() != StorageTarget::LinuxX8664V1
        || linear.universe_identity() != linux.universe_identity()
        || claims.type_universe != linear.universe_identity().as_bytes()
        || &claims.linear32_fingerprint != linear.fingerprint()
        || &claims.linux_x86_64_fingerprint != linux.fingerprint()
        || !same_logical_universe(linear, linux)
    {
        errors.push(error(
            "ZRYNA-I3003",
            "DataOwnershipV1 layout or runtime authority does not match its sealed claim",
            "use both exact layouts issued from the final source-bound type universe",
        ));
        return;
    }
    let expected_nominals = program
        .modules
        .iter()
        .flat_map(|module| {
            (0..module.data_declarations).map(move |declaration| (module.id.0, declaration))
        })
        .collect::<BTreeSet<_>>();
    let layout_nominals = linear
        .types()
        .filter_map(zryna_layout::VerifiedType::nominal_identity)
        .collect::<BTreeSet<_>>();
    if expected_nominals != layout_nominals {
        errors.push(error(
            "ZRYNA-I3003",
            "module nominal declaration inventory does not match the sealed layout universe",
            "bind every source-ordered nominal declaration exactly once",
        ));
        return;
    }
    if program.modules.len() != sources.len() {
        errors.push(error(
            "ZRYNA-I3001",
            "module inventory does not match the final SourceMap",
            "provide every final module exactly once in canonical order",
        ));
    }
    let entry =
        usize::try_from(program.entry_module.0).ok().and_then(|index| program.modules.get(index));
    if entry.map(|module| module.source_file) != Some(expected_entry)
        || sources.source(expected_entry).is_none()
    {
        errors.push(error(
            "ZRYNA-I3001",
            "entry module is not bound to the independently supplied entry source",
            "use the exact entry FileId from the final SourceMap",
        ));
    }
}

#[allow(clippy::too_many_lines)]
fn verify_structure(
    program: &raw::Program,
    sources: &SourceMap,
    layouts: &VerifiedLayouts,
    errors: &mut Errors,
) {
    for (module_index, module) in program.modules.iter().enumerate() {
        if module.id.0 as usize != module_index
            || sources
                .verify_file_id(u32::try_from(module_index).expect("bounded module index"))
                .ok()
                != Some(module.source_file)
        {
            errors.push(error(
                "ZRYNA-I3002",
                "module has a noncanonical identity or source owner",
                "use dense modules in final SourceMap order",
            ));
        }
        for (function_index, function) in module.functions.iter().enumerate() {
            if function.id.module != module.id || function.id.declaration as usize != function_index
            {
                errors.push(error_at(
                    "ZRYNA-I3002",
                    function.span,
                    "function has a noncanonical owner or declaration identity",
                    "use its containing module and dense declaration index",
                ));
            }
            check_span(function.span, module.source_file, sources, errors);
            if layout_type(layouts, function.result).is_none() {
                errors.push(error_at(
                    "ZRYNA-I3003",
                    function.span,
                    "function result names an unknown layout TypeId",
                    "use a type from the exact sealed universe",
                ));
            }
            let mut next_value = 0u32;
            let mut next_borrow = 0u32;
            for value in &function.parameters {
                check_value(value, &mut next_value, module.source_file, sources, layouts, errors);
            }
            for borrow in &function.borrow_parameters {
                if borrow.id.0 != next_borrow || layout_type(layouts, borrow.referent).is_none() {
                    errors.push(error_at(
                        "ZRYNA-I3011",
                        borrow.span,
                        "borrow parameter has a noncanonical identity or unknown referent type",
                        "use dense borrow parameters bound to sealed types",
                    ));
                }
                next_borrow = next_borrow.saturating_add(1);
                check_span(borrow.span, module.source_file, sources, errors);
            }
            let mut root_keys = BTreeSet::new();
            let mut projection_keys = BTreeSet::new();
            for (place_index, place) in function.places.iter().enumerate() {
                if place.id.0 as usize != place_index || layout_type(layouts, place.ty).is_none() {
                    errors.push(error_at(
                        "ZRYNA-I3006",
                        place.span,
                        "place has a noncanonical identity or unknown type",
                        "use dense places and sealed layout types",
                    ));
                }
                let root = match place.kind {
                    raw::PlaceKind::Parameter(index) => Some((0u8, index)),
                    raw::PlaceKind::Local(index) => Some((1u8, index)),
                    raw::PlaceKind::Temporary(value) => Some((2u8, value.0)),
                    _ => None,
                };
                if root.is_some_and(|key| !root_keys.insert(key)) {
                    errors.push(error_at(
                        "ZRYNA-I3006",
                        place.span,
                        "place root aliases an existing parameter, local, or temporary owner",
                        "declare exactly one root place for each owned storage identity",
                    ));
                }
                let projection = match place.kind {
                    raw::PlaceKind::StructField { base, ordinal } => Some((0u8, base.0, ordinal)),
                    raw::PlaceKind::EnumPayload { base, variant } => Some((1u8, base.0, variant)),
                    raw::PlaceKind::FixedArrayConstant { base, index } => {
                        Some((2u8, base.0, index))
                    }
                    _ => None,
                };
                if projection.is_some_and(|key| !projection_keys.insert(key)) {
                    errors.push(error_at(
                        "ZRYNA-I3006",
                        place.span,
                        "projection aliases an existing base and selector",
                        "declare each canonical field, payload, or array projection once",
                    ));
                }
                let mapped_type = match place.kind {
                    raw::PlaceKind::Parameter(index) => {
                        function.parameters.get(index as usize).map(|value| value.ty)
                    }
                    raw::PlaceKind::Temporary(value) => function_value_type(function, value),
                    _ => Some(place.ty),
                };
                if mapped_type != Some(place.ty) {
                    errors.push(error_at(
                        "ZRYNA-I3006",
                        place.span,
                        "root place does not exactly match its parameter or temporary value type",
                        "bind roots only to existing values of the same sealed type",
                    ));
                }
                check_span(place.span, module.source_file, sources, errors);
                verify_projection(place, function, layouts, errors);
            }
            for (plan_index, plan) in function.cleanup_plans.iter().enumerate() {
                let mut dropped = BTreeSet::new();
                if plan.id.0 as usize != plan_index
                    || plan.actions.iter().any(|a| match a {
                        raw::DropAction::DropPlace(p) => {
                            p.0 as usize >= function.places.len() || !dropped.insert(*p)
                        }
                    })
                {
                    errors.push(error_at(
                        "ZRYNA-I3012",
                        plan.span,
                        "cleanup plan has a noncanonical identity or foreign place",
                        "use dense plans containing each local verified place at most once",
                    ));
                }
                check_span(plan.span, module.source_file, sources, errors);
            }
            for (block_index, block) in function.blocks.iter().enumerate() {
                if block.id.0 as usize != block_index {
                    errors.push(error_at(
                        "ZRYNA-I3002",
                        function.span,
                        "block identity is not dense",
                        "use dense block identities in arena order",
                    ));
                }
                if block_index == 0 && !block.parameters.is_empty() {
                    errors.push(error_at(
                        "ZRYNA-I3007",
                        function.span,
                        "entry block declares block parameters",
                        "use function parameters for entry values",
                    ));
                }
                for value in &block.parameters {
                    check_value(
                        value,
                        &mut next_value,
                        module.source_file,
                        sources,
                        layouts,
                        errors,
                    );
                }
                for instruction in &block.instructions {
                    check_span(instruction.span, module.source_file, sources, errors);
                    if let Some(result) = &instruction.result {
                        check_value(
                            result,
                            &mut next_value,
                            module.source_file,
                            sources,
                            layouts,
                            errors,
                        );
                    }
                    if let raw::InstructionKind::BeginBorrow(definition) = &instruction.kind {
                        if definition.id.0 != next_borrow {
                            errors.push(error_at(
                                "ZRYNA-I3011",
                                definition.span,
                                "borrow definitions are not in canonical dense order",
                                "allocate borrow parameters then lexical borrows in instruction order",
                            ));
                        }
                        next_borrow = next_borrow.saturating_add(1);
                        check_span(definition.span, module.source_file, sources, errors);
                    }
                    verify_instruction_shape(instruction, function, errors);
                }
                if block.terminators.len() != 1 {
                    errors.push(error_at(
                        "ZRYNA-I3007",
                        function.span,
                        "block does not contain exactly one terminator",
                        "emit exactly one closed DataOwnershipV1 terminator",
                    ));
                }
                if let Some(terminator) = block.terminators.first() {
                    check_span(terminator.span, module.source_file, sources, errors);
                    verify_terminator(terminator, function, errors);
                }
            }
            let mut referenced_plans = BTreeSet::new();
            for block in &function.blocks {
                referenced_plans.extend(
                    block
                        .instructions
                        .iter()
                        .filter_map(|instruction| instruction_cleanup(&instruction.kind)),
                );
                if let Some(terminator) = block.terminators.first() {
                    match terminator.kind {
                        raw::Terminator::Return { cleanup, .. }
                        | raw::Terminator::WeakUpgradeBranch { cleanup, .. }
                        | raw::Terminator::Trap { cleanup, .. } => {
                            referenced_plans.insert(cleanup);
                        }
                        raw::Terminator::Jump(_)
                        | raw::Terminator::Branch { .. }
                        | raw::Terminator::EnumMatch { .. } => {}
                    }
                }
            }
            if let Some(plan) =
                function.cleanup_plans.iter().find(|plan| !referenced_plans.contains(&plan.id))
            {
                errors.push(error_at(
                    "ZRYNA-I3012",
                    plan.span,
                    "function contains an orphan cleanup plan",
                    "retain only cleanup plans referenced by a verified fallible operation or exit",
                ));
            }
            if !errors.is_empty() {
                return;
            }
            verify_function_graph(function, layouts, errors);
            if !errors.is_empty() {
                return;
            }
        }
    }
}

#[derive(Clone, Copy)]
enum DefinitionSite {
    FunctionParameter,
    BlockParameter(usize),
    Instruction(usize, usize),
}

#[derive(Clone, Copy)]
struct ValueInfo {
    ty: raw::TypeId,
    site: DefinitionSite,
}

#[allow(clippy::too_many_lines)]
fn verify_function_graph(function: &raw::Function, layouts: &VerifiedLayouts, errors: &mut Errors) {
    if function.blocks.is_empty() {
        return;
    }
    let mut values = Vec::new();
    values.extend(
        function
            .parameters
            .iter()
            .map(|value| ValueInfo { ty: value.ty, site: DefinitionSite::FunctionParameter }),
    );
    for (block_index, block) in function.blocks.iter().enumerate() {
        values.extend(block.parameters.iter().map(|value| ValueInfo {
            ty: value.ty,
            site: DefinitionSite::BlockParameter(block_index),
        }));
        values.extend(block.instructions.iter().enumerate().filter_map(
            |(instruction_index, instruction)| {
                instruction.result.map(|result| ValueInfo {
                    ty: result.ty,
                    site: DefinitionSite::Instruction(block_index, instruction_index),
                })
            },
        ));
    }

    let mut successors = vec![Vec::new(); function.blocks.len()];
    let mut predecessors = vec![Vec::new(); function.blocks.len()];
    for (block_index, block) in function.blocks.iter().enumerate() {
        let Some(terminator) = block.terminators.first() else {
            continue;
        };
        for edge in terminator_edges(&terminator.kind) {
            let Ok(target) = usize::try_from(edge.target.0) else {
                continue;
            };
            let Some(target_block) = function.blocks.get(target) else {
                continue;
            };
            successors[block_index].push(target);
            predecessors[target].push(block_index);
            let synthesized = matches!(
                &terminator.kind,
                raw::Terminator::WeakUpgradeBranch { success, .. }
                    if std::ptr::eq(edge, success)
            );
            let target_parameters = if synthesized {
                &target_block.parameters[usize::from(!target_block.parameters.is_empty())..]
            } else {
                &target_block.parameters[..]
            };
            if edge.arguments.len() != target_parameters.len() {
                errors.push(error_at(
                    "ZRYNA-I3007",
                    terminator.span,
                    "CFG edge argument arity does not match target block parameters",
                    "pass one exact typed value for every target block parameter",
                ));
                continue;
            }
            for (argument, parameter) in edge.arguments.iter().zip(target_parameters) {
                if value_info(&values, *argument).is_none_or(|info| info.ty != parameter.ty) {
                    errors.push(error_at(
                        "ZRYNA-I3007",
                        terminator.span,
                        "CFG edge argument type does not match its target parameter",
                        "pass exact sealed types across every edge",
                    ));
                }
            }
        }
    }
    if !predecessors[0].is_empty() {
        errors.push(error_at(
            "ZRYNA-I3007",
            function.span,
            "entry block has a predecessor",
            "never target the entry block",
        ));
    }
    let mut reachable = vec![false; function.blocks.len()];
    let mut queue = VecDeque::from([0usize]);
    while let Some(block) = queue.pop_front() {
        if reachable[block] {
            continue;
        }
        reachable[block] = true;
        queue.extend(successors[block].iter().copied());
    }
    if reachable.iter().any(|value| !value) || predecessors.iter().skip(1).any(Vec::is_empty) {
        errors.push(error_at(
            "ZRYNA-I3007",
            function.span,
            "function contains an unreachable or predecessor-free nonentry block",
            "emit only blocks reachable from entry",
        ));
    }

    let dominators = compute_dominators(&predecessors, &reachable);
    verify_reducible_loops(&successors, &predecessors, &dominators, Some(function.span), errors);
    if !errors.is_empty() {
        return;
    }
    verify_enum_payload_dominance(function, &dominators, errors);
    if !errors.is_empty() {
        return;
    }
    for (block_index, block) in function.blocks.iter().enumerate() {
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            for operand in instruction_operands(&instruction.kind) {
                verify_use(
                    operand,
                    block_index,
                    instruction_index,
                    instruction.span,
                    &values,
                    &dominators,
                    errors,
                );
            }
            verify_operation_types(instruction, function, &values, layouts, errors);
        }
        if let Some(terminator) = block.terminators.first() {
            for operand in terminator_operands(&terminator.kind) {
                verify_use(
                    operand,
                    block_index,
                    block.instructions.len(),
                    terminator.span,
                    &values,
                    &dominators,
                    errors,
                );
            }
            verify_terminator_types(terminator, function, &values, layouts, errors);
        }
    }
    verify_owned_value_owners(function, &values, layouts, &successors, errors);
    if !errors.is_empty() {
        return;
    }
    verify_ownership_dataflow(function, layouts, &successors, errors);
}

fn verify_enum_payload_dominance(
    function: &raw::Function,
    dominators: &[BTreeSet<usize>],
    errors: &mut Errors,
) {
    for place in &function.places {
        let raw::PlaceKind::EnumPayload { base, variant } = place.kind else { continue };
        let refinements = function
            .blocks
            .iter()
            .filter_map(|block| block.terminators.first())
            .filter_map(|terminator| match &terminator.kind {
                raw::Terminator::EnumMatch { place, arms } if *place == base => arms
                    .iter()
                    .find(|arm| arm.variant == variant)
                    .map(|arm| arm.edge.target.0 as usize),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (block_index, block) in function.blocks.iter().enumerate() {
            let used = block.instructions.iter().any(|instruction| {
                instruction_place_operands(&instruction.kind).contains(&place.id)
            }) || block.terminators.first().is_some_and(|terminator| {
                terminator_place_operands(&terminator.kind).contains(&place.id)
            });
            if used
                && !refinements
                    .iter()
                    .any(|refinement| dominators[block_index].contains(refinement))
            {
                errors.push(error_at(
                    "ZRYNA-I3013",
                    place.span,
                    "enum payload use is not dominated by its matching active-variant arm",
                    "use a payload only below the exact exhaustive-match successor for its variant",
                ));
                return;
            }
        }
    }
}

fn instruction_place_operands(kind: &raw::InstructionKind) -> Vec<raw::PlaceId> {
    use raw::InstructionKind as I;
    match kind {
        I::CopyFromPlace { place }
        | I::MoveFromPlace { place }
        | I::ClonePlace { place, .. }
        | I::InitializePlace { place, .. }
        | I::ReplacePlace { place, .. }
        | I::DropPlace { place }
        | I::EnumDiscriminant { place }
        | I::FixedArrayIndexCopy { place, .. }
        | I::VecIndexCopy { place, .. }
        | I::StringClone { place, .. }
        | I::SharedClone { place, .. }
        | I::WeakDowngrade { place, .. }
        | I::WeakClone { place, .. } => vec![*place],
        I::StringConcat { left, right, .. } => vec![*left, *right],
        I::VecPush { vector, .. } => vec![*vector],
        I::BeginBorrow(definition) => vec![definition.place],
        _ => vec![],
    }
}

fn terminator_place_operands(kind: &raw::Terminator) -> Vec<raw::PlaceId> {
    match kind {
        raw::Terminator::EnumMatch { place, .. } => vec![*place],
        raw::Terminator::WeakUpgradeBranch { weak, .. } => vec![*weak],
        _ => vec![],
    }
}

#[allow(clippy::too_many_lines)]
fn verify_owned_value_owners(
    function: &raw::Function,
    values: &[ValueInfo],
    layouts: &VerifiedLayouts,
    successors: &[Vec<usize>],
    errors: &mut Errors,
) {
    const UNAVAILABLE: u8 = 0;
    const AVAILABLE: u8 = 1;
    const CONSUMED: u8 = 2;
    let non_copy = |id: raw::ValueId| {
        value_info(values, id)
            .and_then(|info| layout_type(layouts, info.ty))
            .is_some_and(|ty| ty.drop_kind() != 0)
    };
    let mut initial = vec![UNAVAILABLE; values.len()];
    for parameter in &function.parameters {
        if non_copy(parameter.id) {
            initial[parameter.id.0 as usize] = AVAILABLE;
        }
    }
    for place in &function.places {
        if let raw::PlaceKind::Parameter(parameter) = place.kind {
            let id = raw::ValueId(parameter);
            if non_copy(id) {
                initial[id.0 as usize] = CONSUMED;
            }
        }
    }
    let consume = |id: raw::ValueId, state: &mut [u8], span: Span, errors: &mut Errors| {
        if !non_copy(id) {
            return;
        }
        let Some(slot) = state.get_mut(id.0 as usize) else { return };
        if *slot == AVAILABLE {
            *slot = CONSUMED;
        } else {
            errors.push(error_at(
                "ZRYNA-I3008",
                span,
                "non-Copy value is consumed more than once or before it is available",
                "transfer each non-Copy value exactly once on every reachable path",
            ));
        }
    };
    let mut entries = vec![None::<Vec<u8>>; function.blocks.len()];
    entries[0] = Some(initial);
    let mut queue = VecDeque::from([0usize]);
    while let Some(block_index) = queue.pop_front() {
        let Some(mut state) = entries[block_index].clone() else { continue };
        let block = &function.blocks[block_index];
        for parameter in &block.parameters {
            if non_copy(parameter.id) {
                state[parameter.id.0 as usize] = AVAILABLE;
            }
        }
        for instruction in &block.instructions {
            for operand in consuming_instruction_operands(&instruction.kind) {
                consume(operand, &mut state, instruction.span, errors);
            }
            if let Some(result) = instruction.result
                && non_copy(result.id)
            {
                state[result.id.0 as usize] = if function.places.iter().any(
                    |place| matches!(place.kind, raw::PlaceKind::Temporary(value) if value == result.id),
                ) {
                    CONSUMED
                } else {
                    AVAILABLE
                };
            }
        }
        let Some(terminator) = block.terminators.first() else { continue };
        if let raw::Terminator::Return { value, .. } = terminator.kind {
            consume(value, &mut state, terminator.span, errors);
            if state.contains(&AVAILABLE) {
                errors.push(error_at(
                    "ZRYNA-I3008",
                    terminator.span,
                    "reachable return leaves a non-Copy SSA value without an owner",
                    "place, transfer, or return every non-Copy value on this path",
                ));
            }
            continue;
        }
        if matches!(terminator.kind, raw::Terminator::Trap { .. }) {
            if state.contains(&AVAILABLE) {
                errors.push(error_at(
                    "ZRYNA-I3008",
                    terminator.span,
                    "reachable trap leaves a non-Copy SSA value without an owner",
                    "place or transfer every non-Copy value before trapping",
                ));
            }
            continue;
        }
        for (edge, successor) in
            terminator_edges(&terminator.kind).into_iter().zip(&successors[block_index])
        {
            let mut edge_state = state.clone();
            for argument in &edge.arguments {
                consume(*argument, &mut edge_state, terminator.span, errors);
            }
            match &entries[*successor] {
                None => {
                    entries[*successor] = Some(edge_state);
                    queue.push_back(*successor);
                }
                Some(existing) if existing != &edge_state => errors.push(error_at(
                    "ZRYNA-I3008",
                    terminator.span,
                    "non-Copy value ownership differs across a CFG join or backedge",
                    "make every incoming path transfer exactly the same SSA ownership state",
                )),
                Some(_) => {}
            }
        }
    }
}

fn consuming_instruction_operands(kind: &raw::InstructionKind) -> Vec<raw::ValueId> {
    use raw::InstructionKind as I;
    match kind {
        I::DirectCall { arguments, .. } => arguments
            .iter()
            .filter_map(|argument| match argument {
                raw::CallArgument::Value(value) => Some(*value),
                raw::CallArgument::Borrow(_) => None,
            })
            .collect(),
        I::StructConstruct { fields, .. } => fields.clone(),
        I::EnumConstruct { payload, .. } => payload.iter().copied().collect(),
        I::FixedArrayConstruct { elements, .. } | I::VecConstruct { elements, .. } => {
            elements.clone()
        }
        I::InitializePlace { value, .. }
        | I::ReplacePlace { value, .. }
        | I::VecPush { value, .. }
        | I::SharedConstruct { value, .. }
        | I::BorrowWrite { value, .. } => vec![*value],
        _ => vec![],
    }
}

fn verify_reducible_loops(
    successors: &[Vec<usize>],
    predecessors: &[Vec<usize>],
    dominators: &[BTreeSet<usize>],
    span: Option<Span>,
    errors: &mut Errors,
) {
    let components = strongly_connected_components(successors);
    let mut nesting = vec![0usize; successors.len()];
    for component in components {
        let cyclic = component.len() > 1
            || component.first().is_some_and(|node| successors[*node].contains(node));
        if !cyclic {
            continue;
        }
        let headers = component
            .iter()
            .copied()
            .filter(|candidate| component.iter().all(|node| dominators[*node].contains(candidate)))
            .collect::<Vec<_>>();
        if headers.len() != 1 {
            errors.push(error_with_optional_span(
                "ZRYNA-I3007",
                span,
                "CFG contains an irreducible control-flow cycle",
                "use one dominating loop header for every cycle",
            ));
            return;
        }
        let header = headers[0];
        if component.iter().any(|node| {
            predecessors[*node]
                .iter()
                .any(|predecessor| !component.contains(predecessor) && *node != header)
        }) {
            errors.push(error_with_optional_span(
                "ZRYNA-I3007",
                span,
                "loop has an entry that bypasses its verified header",
                "route every external loop entry through its dominating header",
            ));
            return;
        }
        for source in &component {
            for target in &successors[*source] {
                if component.contains(target) && dominators[*source].contains(target) {
                    let mut loop_nodes = BTreeSet::from([*target, *source]);
                    let mut work = VecDeque::from([*source]);
                    while let Some(node) = work.pop_front() {
                        for predecessor in &predecessors[node] {
                            if loop_nodes.insert(*predecessor) {
                                work.push_back(*predecessor);
                            }
                        }
                    }
                    for node in loop_nodes {
                        nesting[node] = nesting[node].saturating_add(1);
                        if nesting[node] > MAX_LOOP_NESTING {
                            errors.limit("verified loop nesting", MAX_LOOP_NESTING);
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn strongly_connected_components(successors: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut visited = vec![false; successors.len()];
    let mut order = Vec::with_capacity(successors.len());
    for start in 0..successors.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, edge_index)) = stack.pop() {
            if let Some(target) = successors[node].get(edge_index).copied() {
                stack.push((node, edge_index + 1));
                if !visited[target] {
                    visited[target] = true;
                    stack.push((target, 0));
                }
            } else {
                order.push(node);
            }
        }
    }
    let mut reverse = vec![Vec::new(); successors.len()];
    for (source, edges) in successors.iter().enumerate() {
        for target in edges {
            reverse[*target].push(source);
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    for start in order.into_iter().rev() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.push(node);
            for predecessor in &reverse[node] {
                if !visited[*predecessor] {
                    visited[*predecessor] = true;
                    stack.push(*predecessor);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

fn verify_terminator_types(
    terminator: &raw::SpannedTerminator,
    function: &raw::Function,
    values: &[ValueInfo],
    layouts: &VerifiedLayouts,
    errors: &mut Errors,
) {
    let valid = match &terminator.kind {
        raw::Terminator::Return { value, .. } => {
            value_info(values, *value).is_some_and(|info| info.ty == function.result)
        }
        raw::Terminator::Branch { condition, .. } => value_info(values, *condition)
            .and_then(|info| layout_type(layouts, info.ty))
            .is_some_and(|ty| ty.category() == TypeCategory::Bool),
        raw::Terminator::EnumMatch { place, arms } => function
            .places
            .get(place.0 as usize)
            .and_then(|place| layout_type(layouts, place.ty))
            .is_some_and(|ty| {
                ty.category() == TypeCategory::Enum
                    && arms.len() == ty.variants().len()
                    && arms
                        .iter()
                        .zip(ty.variants())
                        .all(|(arm, variant)| arm.variant == variant.ordinal())
            }),
        raw::Terminator::WeakUpgradeBranch { weak, success, expired, .. } => function
            .places
            .get(weak.0 as usize)
            .and_then(|place| layout_type(layouts, place.ty))
            .is_some_and(|weak_ty| {
                let Some(payload) = weak_ty.referenced_type() else { return false };
                if weak_ty.category() != TypeCategory::Weak {
                    return false;
                }
                let Some(success_block) = function.blocks.get(success.target.0 as usize) else {
                    return false;
                };
                let Some(first) = success_block.parameters.first() else { return false };
                layout_type(layouts, first.ty).is_some_and(|shared| {
                    shared.category() == TypeCategory::Shared
                        && shared.referenced_type() == Some(payload)
                        && success.arguments.len() + 1 == success_block.parameters.len()
                }) && function
                    .blocks
                    .get(expired.target.0 as usize)
                    .is_some_and(|block| expired.arguments.len() == block.parameters.len())
            }),
        raw::Terminator::Jump(_) | raw::Terminator::Trap { .. } => true,
    };
    if !valid {
        errors.push(error_at(
            "ZRYNA-I3014",
            terminator.span,
            "terminator result, condition, enum arms, or weak-upgrade edge shape is invalid",
            "use exact return/bool/exhaustive-enum/weak-upgrade types and successor shapes",
        ));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaceStateKind {
    Uninitialized,
    Initialized,
    PartiallyInitialized,
    PartiallyMoved,
    Moved,
    Dropped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlaceState {
    kind: PlaceStateKind,
    completion: Option<usize>,
}

fn derive_state_before(
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    target_block: usize,
    target_instruction: usize,
) -> Option<(Vec<PlaceState>, Vec<Option<u32>>)> {
    let mut initial = function
        .places
        .iter()
        .map(|place| PlaceState {
            kind: if matches!(place.kind, raw::PlaceKind::Parameter(_)) {
                PlaceStateKind::Initialized
            } else {
                PlaceStateKind::Uninitialized
            },
            completion: matches!(place.kind, raw::PlaceKind::Parameter(_)).then_some(0),
        })
        .collect::<Vec<_>>();
    for index in 0..function.places.len() {
        if let Some(base) = projection_base(&function.places[index].kind)
            && initial[base.0 as usize].kind == PlaceStateKind::Initialized
        {
            initial[index] = PlaceState { kind: PlaceStateKind::Initialized, completion: Some(0) };
        }
    }
    let mut entries = vec![None; function.blocks.len()];
    entries[0] = Some((initial, vec![None; function.places.len()]));
    let mut queue = VecDeque::from([0usize]);
    while let Some(block_index) = queue.pop_front() {
        let (mut states, mut variants) = entries[block_index].clone()?;
        let block = &function.blocks[block_index];
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            if block_index == target_block && instruction_index == target_instruction {
                return Some((states, variants));
            }
            let completion = block_index
                .saturating_mul(MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
                .saturating_add(instruction_index + 1);
            match instruction.kind {
                raw::InstructionKind::MoveFromPlace { place }
                    if !place_is_copy(place, function, layouts) =>
                {
                    states[place.0 as usize] =
                        PlaceState { kind: PlaceStateKind::Moved, completion: None };
                    mark_ancestors_partial(place, function, &mut states);
                }
                raw::InstructionKind::InitializePlace { place, .. } => {
                    states[place.0 as usize] = PlaceState {
                        kind: PlaceStateKind::Initialized,
                        completion: Some(completion),
                    };
                    initialize_projections(place, function, &mut states, completion);
                    mark_ancestors_partially_initialized(place, function, &mut states, completion);
                    variants[place.0 as usize] = None;
                }
                raw::InstructionKind::ReplacePlace { place, .. } => {
                    states[place.0 as usize].completion = Some(completion);
                    variants[place.0 as usize] = None;
                }
                raw::InstructionKind::DropPlace { place } => {
                    states[place.0 as usize] =
                        PlaceState { kind: PlaceStateKind::Dropped, completion: None };
                    mark_ancestors_partial(place, function, &mut states);
                }
                _ => {}
            }
            if let Some(result) = instruction.result {
                for (index, place) in function.places.iter().enumerate() {
                    if matches!(place.kind, raw::PlaceKind::Temporary(value) if value == result.id)
                    {
                        states[index] = PlaceState {
                            kind: PlaceStateKind::Initialized,
                            completion: Some(completion),
                        };
                        initialize_projections(place.id, function, &mut states, completion);
                        variants[index] = match instruction.kind {
                            raw::InstructionKind::EnumConstruct { variant, .. } => Some(variant),
                            _ => None,
                        };
                    }
                }
            }
        }
        if block_index == target_block && target_instruction == block.instructions.len() {
            return Some((states, variants));
        }
        let Some(terminator) = block.terminators.first() else { continue };
        for (edge_index, edge) in terminator_edges(&terminator.kind).into_iter().enumerate() {
            let mut edge_variants = variants.clone();
            if let raw::Terminator::EnumMatch { place, arms } = &terminator.kind {
                edge_variants[place.0 as usize] = Some(arms[edge_index].variant);
            }
            let target = edge.target.0 as usize;
            if entries[target].is_none() {
                entries[target] = Some((states.clone(), edge_variants));
                queue.push_back(target);
            }
        }
    }
    None
}

fn sealed_drop_actions(
    owner: FunctionIdentity,
    function: &raw::Function,
    plan: raw::CleanupPlanId,
    states: &[PlaceState],
    variants: &[Option<u32>],
) -> Vec<VerifiedDropAction> {
    function.cleanup_plans[plan.0 as usize]
        .actions
        .iter()
        .map(|action| {
            let raw::DropAction::DropPlace(root) = action;
            let moved_projections = function
                .places
                .iter()
                .filter(|place| {
                    is_projection_below(place.id, *root, &function.places)
                        && matches!(
                            states[place.id.0 as usize].kind,
                            PlaceStateKind::Moved | PlaceStateKind::Dropped
                        )
                })
                .map(|place| PlaceIdentity { owner, index: place.id.0 })
                .collect();
            let initialized_projections = function
                .places
                .iter()
                .filter(|place| {
                    is_projection_below(place.id, *root, &function.places)
                        && states[place.id.0 as usize].kind == PlaceStateKind::Initialized
                })
                .map(|place| PlaceIdentity { owner, index: place.id.0 })
                .collect();
            VerifiedDropAction {
                root: PlaceIdentity { owner, index: root.0 },
                moved_projections,
                initialized_projections,
                active_variant: variants[root.0 as usize],
            }
        })
        .collect()
}

fn is_projection_below(mut place: raw::PlaceId, root: raw::PlaceId, places: &[raw::Place]) -> bool {
    let mut visited = BTreeSet::new();
    while visited.insert(place) {
        let Some(base) =
            places.get(place.0 as usize).and_then(|place| projection_base(&place.kind))
        else {
            return false;
        };
        if base == root {
            return true;
        }
        place = base;
    }
    false
}

#[allow(clippy::too_many_lines)]
fn verify_ownership_dataflow(
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    successors: &[Vec<usize>],
    errors: &mut Errors,
) {
    if function.blocks.is_empty() {
        return;
    }
    let mut initial = function
        .places
        .iter()
        .map(|place| PlaceState {
            kind: match place.kind {
                raw::PlaceKind::Parameter(_) => PlaceStateKind::Initialized,
                _ => PlaceStateKind::Uninitialized,
            },
            completion: matches!(place.kind, raw::PlaceKind::Parameter(_)).then_some(0),
        })
        .collect::<Vec<_>>();
    for index in 0..function.places.len() {
        if let Some(base) = projection_base(&function.places[index].kind)
            && initial
                .get(base.0 as usize)
                .is_some_and(|state| state.kind == PlaceStateKind::Initialized)
        {
            initial[index] = PlaceState { kind: PlaceStateKind::Initialized, completion: Some(0) };
        }
    }
    let initial_variants = vec![None::<u32>; function.places.len()];
    let mut entries = vec![None::<(Vec<PlaceState>, Vec<Option<u32>>)>; function.blocks.len()];
    entries[0] = Some((initial, initial_variants));
    let mut queue = VecDeque::from([0usize]);
    while let Some(block_index) = queue.pop_front() {
        let Some((mut state, mut active_variants)) = entries[block_index].clone() else {
            continue;
        };
        let mut active = vec![
            None::<(raw::PlaceId, raw::BorrowAccess)>;
            function.borrow_parameters.len().saturating_add(
                function.blocks.iter().map(|block| block.instructions.len()).sum::<usize>(),
            )
        ];
        for parameter in &function.borrow_parameters {
            if let Some(slot) = active.get_mut(parameter.id.0 as usize) {
                *slot = Some((raw::PlaceId(u32::MAX), parameter.access));
            }
        }
        let block = &function.blocks[block_index];
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            let completion = block_index
                .saturating_mul(MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
                .saturating_add(instruction_index)
                .saturating_add(1);
            if let Some(cleanup) = instruction_cleanup(&instruction.kind) {
                verify_cleanup(cleanup, function, layouts, &state, &active_variants, errors);
            }
            apply_ownership_instruction(
                instruction,
                function,
                layouts,
                &mut state,
                &mut active_variants,
                &mut active,
                completion,
                errors,
            );
            if let Some(result) = instruction.result {
                for (place_index, place) in function.places.iter().enumerate() {
                    if matches!(place.kind, raw::PlaceKind::Temporary(value) if value == result.id)
                    {
                        state[place_index] = PlaceState {
                            kind: PlaceStateKind::Initialized,
                            completion: Some(completion),
                        };
                        initialize_projections(place.id, function, &mut state, completion);
                        active_variants[place_index] = match instruction.kind {
                            raw::InstructionKind::EnumConstruct { variant, .. } => Some(variant),
                            _ => None,
                        };
                    }
                }
            }
        }
        if active.iter().skip(function.borrow_parameters.len()).any(Option::is_some) {
            errors.push(error_at(
                "ZRYNA-I3011",
                block.terminators[0].span,
                "borrow remains active at a control-flow edge",
                "end every borrow before a branch, jump, loop edge, return, or trap",
            ));
        }
        if let Some(terminator) = block.terminators.first() {
            let place_read = match terminator.kind {
                raw::Terminator::EnumMatch { place, .. }
                | raw::Terminator::WeakUpgradeBranch { weak: place, .. } => Some(place),
                _ => None,
            };
            if place_read.is_some_and(|place| {
                state
                    .get(place.0 as usize)
                    .is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                    || overlaps_exclusive(place, &active, &function.places)
            }) {
                errors.push(error_at(
                    "ZRYNA-I3010",
                    terminator.span,
                    "enum or weak terminator reads an unavailable or exclusively borrowed owner",
                    "branch only on one initialized owner not hidden by an exclusive borrow",
                ));
            }
            match &terminator.kind {
                raw::Terminator::Return { cleanup, .. }
                | raw::Terminator::Trap { cleanup, .. }
                | raw::Terminator::WeakUpgradeBranch { cleanup, .. } => {
                    verify_cleanup(*cleanup, function, layouts, &state, &active_variants, errors);
                }
                _ => {}
            }
        }
        for (edge_index, successor) in successors[block_index].iter().enumerate() {
            let mut successor_variants = active_variants.clone();
            if let Some(raw::SpannedTerminator {
                kind: raw::Terminator::EnumMatch { place, arms },
                ..
            }) = block.terminators.first()
                && let Some(arm) = arms.get(edge_index)
            {
                successor_variants[place.0 as usize] = Some(arm.variant);
            }
            let incoming = (state.clone(), successor_variants);
            match &entries[*successor] {
                None => { entries[*successor] = Some(incoming); queue.push_back(*successor); }
                Some(existing) if existing != &incoming => errors.push(error_at("ZRYNA-I3010", block.terminators[0].span, "ownership, initialization, or active-enum state differs across a CFG join or backedge", "restore every live place and enum refinement to one exact state on every incoming edge")),
                Some(_) => {}
            }
        }
    }
}

fn instruction_cleanup(kind: &raw::InstructionKind) -> Option<raw::CleanupPlanId> {
    use raw::InstructionKind as I;
    match kind {
        I::DirectCall { cleanup, .. }
        | I::StructConstruct { cleanup, .. }
        | I::EnumConstruct { cleanup, .. }
        | I::FixedArrayConstruct { cleanup, .. } => *cleanup,
        I::ClonePlace { cleanup, .. }
        | I::ReplacePlace { cleanup, .. }
        | I::FixedArrayIndexCopy { cleanup, .. }
        | I::VecIndexCopy { cleanup, .. }
        | I::StringClone { cleanup, .. }
        | I::StringConcat { cleanup, .. }
        | I::VecConstruct { cleanup, .. }
        | I::VecPush { cleanup, .. }
        | I::SharedConstruct { cleanup, .. }
        | I::SharedClone { cleanup, .. }
        | I::WeakDowngrade { cleanup, .. }
        | I::WeakClone { cleanup, .. } => Some(*cleanup),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn apply_ownership_instruction(
    instruction: &raw::Instruction,
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    states: &mut [PlaceState],
    active_variants: &mut [Option<u32>],
    active: &mut Vec<Option<(raw::PlaceId, raw::BorrowAccess)>>,
    completion: usize,
    errors: &mut Errors,
) {
    use raw::InstructionKind as I;
    let state = |place: raw::PlaceId, states: &[PlaceState]| states.get(place.0 as usize).copied();
    for place in instruction_place_operands(&instruction.kind) {
        if let Some(raw::Place { kind: raw::PlaceKind::EnumPayload { base, variant }, .. }) =
            function.places.get(place.0 as usize)
            && active_variants.get(base.0 as usize).copied().flatten() != Some(*variant)
        {
            errors.push(error_at(
                "ZRYNA-I3013",
                instruction.span,
                "enum payload ownership operation does not match the active variant",
                "operate on a payload only in its exact refined enum arm",
            ));
        }
    }
    match &instruction.kind {
        I::CopyFromPlace { place } => {
            if state(*place, states).is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                || !place_is_copy(*place, function, layouts)
                || overlaps_exclusive(*place, active, &function.places)
            {
                ownership_error(
                    instruction.span,
                    "copy from a non-Copy or non-initialized place",
                    errors,
                );
            }
        }
        I::MoveFromPlace { place } => {
            if state(*place, states).is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                || overlaps_active(*place, active, &function.places)
            {
                ownership_error(
                    instruction.span,
                    "move from an unavailable or borrowed place",
                    errors,
                );
            } else if !place_is_copy(*place, function, layouts) {
                states[place.0 as usize] =
                    PlaceState { kind: PlaceStateKind::Moved, completion: None };
                mark_ancestors_partial(*place, function, states);
            }
        }
        I::ClonePlace { place, .. }
        | I::EnumDiscriminant { place }
        | I::FixedArrayIndexCopy { place, .. }
        | I::VecIndexCopy { place, .. }
        | I::StringClone { place, .. }
        | I::SharedClone { place, .. }
        | I::WeakDowngrade { place, .. }
        | I::WeakClone { place, .. } => {
            if state(*place, states).is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                || overlaps_exclusive(*place, active, &function.places)
            {
                ownership_error(instruction.span, "operation reads an unavailable place", errors);
            }
        }
        I::StringConcat { left, right, .. } => {
            if [*left, *right].into_iter().any(|place| {
                state(place, states).is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                    || overlaps_exclusive(place, active, &function.places)
            }) {
                ownership_error(
                    instruction.span,
                    "string concatenation reads an unavailable place",
                    errors,
                );
            }
        }
        I::VecPush { vector, .. } => {
            if state(*vector, states).is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                || overlaps_active(*vector, active, &function.places)
            {
                ownership_error(
                    instruction.span,
                    "vector push mutates an unavailable or borrowed place",
                    errors,
                );
            }
        }
        I::InitializePlace { place, .. } => {
            let Some(slot) = states.get_mut(place.0 as usize) else { return };
            if slot.kind == PlaceStateKind::Initialized
                || overlaps_active(*place, active, &function.places)
            {
                ownership_error(
                    instruction.span,
                    "initialization targets an initialized or borrowed place",
                    errors,
                );
            } else {
                *slot =
                    PlaceState { kind: PlaceStateKind::Initialized, completion: Some(completion) };
                initialize_projections(*place, function, states, completion);
                mark_ancestors_partially_initialized(*place, function, states, completion);
                active_variants[place.0 as usize] = None;
            }
        }
        I::ReplacePlace { place, .. } => {
            let unavailable = state(*place, states)
                .is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                || overlaps_active(*place, active, &function.places);
            if unavailable {
                ownership_error(
                    instruction.span,
                    "replacement targets an unavailable or borrowed place",
                    errors,
                );
            } else {
                states[place.0 as usize].completion = Some(completion);
                active_variants[place.0 as usize] = None;
            }
        }
        I::DropPlace { place } => {
            let unavailable = state(*place, states).is_none_or(|state| {
                !matches!(
                    state.kind,
                    PlaceStateKind::Initialized
                        | PlaceStateKind::PartiallyInitialized
                        | PlaceStateKind::PartiallyMoved
                )
            }) || overlaps_active(*place, active, &function.places)
                || place_is_copy(*place, function, layouts);
            if unavailable {
                ownership_error(
                    instruction.span,
                    "drop targets a Copy, unavailable, or borrowed place",
                    errors,
                );
            } else {
                states[place.0 as usize] =
                    PlaceState { kind: PlaceStateKind::Dropped, completion: None };
                mark_ancestors_partial(*place, function, states);
            }
        }
        I::BeginBorrow(definition) => {
            let index = definition.id.0 as usize;
            if state(definition.place, states)
                .is_none_or(|state| state.kind != PlaceStateKind::Initialized)
                || conflicts(definition, active, &function.places)
            {
                errors.push(error_at(
                    "ZRYNA-I3011",
                    instruction.span,
                    "borrow identity, owner state, or overlap is invalid",
                    "use a dense borrow ID and obey shared/exclusive overlap rules",
                ));
            } else {
                if active.len() <= index {
                    active.resize(index + 1, None);
                }
                if active[index].is_some() {
                    errors.push(error_at(
                        "ZRYNA-I3011",
                        instruction.span,
                        "borrow identity is defined more than once",
                        "define each borrow once",
                    ));
                } else {
                    active[index] = Some((definition.place, definition.access));
                }
            }
        }
        I::BorrowRead { borrow } => {
            if active.get(borrow.0 as usize).is_none_or(Option::is_none) {
                errors.push(error_at(
                    "ZRYNA-I3011",
                    instruction.span,
                    "borrow read uses an inactive authority",
                    "read only through one active borrow",
                ));
            }
        }
        I::BorrowWrite { borrow, .. } => {
            if active
                .get(borrow.0 as usize)
                .and_then(|entry| *entry)
                .is_none_or(|(_, access)| access != raw::BorrowAccess::Exclusive)
            {
                errors.push(error_at(
                    "ZRYNA-I3011",
                    instruction.span,
                    "borrow write lacks an active exclusive authority",
                    "write only through one active exclusive borrow",
                ));
            }
        }
        I::DirectCall { arguments, .. } => {
            if arguments.iter().any(|argument| {
                let raw::CallArgument::Borrow(borrow) = argument else {
                    return false;
                };
                active.get(borrow.0 as usize).is_none_or(Option::is_none)
            }) {
                errors.push(error_at(
                    "ZRYNA-I3011",
                    instruction.span,
                    "direct call uses an inactive borrow authority",
                    "pass only a borrow parameter or a currently active lexical borrow",
                ));
            }
        }
        I::EndBorrow { borrow }
            if (borrow.0 as usize) < function.borrow_parameters.len()
                || active.get_mut(borrow.0 as usize).and_then(Option::take).is_none() =>
        {
            errors.push(error_at(
                "ZRYNA-I3011",
                instruction.span,
                "borrow end uses an inactive authority",
                "end each active borrow exactly once",
            ));
        }
        _ => {}
    }
}

fn verify_cleanup(
    id: raw::CleanupPlanId,
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    states: &[PlaceState],
    active_variants: &[Option<u32>],
    errors: &mut Errors,
) {
    let Some(plan) = function.cleanup_plans.get(id.0 as usize) else { return };
    let mut expected = function
        .places
        .iter()
        .enumerate()
        .filter(|(index, place)| {
            projection_base(&place.kind).is_none()
                && matches!(
                    states[*index].kind,
                    PlaceStateKind::Initialized
                        | PlaceStateKind::PartiallyInitialized
                        | PlaceStateKind::PartiallyMoved
                )
                && layout_type(layouts, place.ty).is_some_and(|ty| ty.drop_kind() != 0)
        })
        .map(|(index, _)| {
            (
                states[index].completion.unwrap_or(0),
                raw::PlaceId(u32::try_from(index).expect("bounded place index")),
            )
        })
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| right.cmp(left));
    let expected = expected.into_iter().map(|(_, place)| place).collect::<Vec<_>>();
    let claimed = plan
        .actions
        .iter()
        .map(|action| match action {
            raw::DropAction::DropPlace(place) => *place,
        })
        .collect::<Vec<_>>();
    if claimed != expected {
        errors.push(error_at(
            "ZRYNA-I3012",
            plan.span,
            "cleanup plan is incomplete, duplicated, or out of reverse-completion order",
            "drop every live non-Copy root exactly once in reverse completion order",
        ));
    }
    for place in &expected {
        let index = place.0 as usize;
        if matches!(
            states[index].kind,
            PlaceStateKind::PartiallyInitialized | PlaceStateKind::PartiallyMoved
        ) && layout_type(layouts, function.places[index].ty)
            .is_some_and(|ty| ty.category() == TypeCategory::Enum)
            && active_variants[index].is_none()
        {
            errors.push(error_at(
                "ZRYNA-I3013",
                plan.span,
                "partial enum cleanup lacks an exact active-variant refinement",
                "derive cleanup only while the enum's active variant is sealed on this path",
            ));
        }
    }
}

fn ownership_error(span: Span, message: &'static str, errors: &mut Errors) {
    errors.push(error_at(
        "ZRYNA-I3010",
        span,
        message,
        "emit only legal ownership state transitions",
    ));
}
fn place_is_copy(place: raw::PlaceId, function: &raw::Function, layouts: &VerifiedLayouts) -> bool {
    function
        .places
        .get(place.0 as usize)
        .and_then(|place| layout_type(layouts, place.ty))
        .is_some_and(|ty| ty.drop_kind() == 0)
}
fn projection_base(kind: &raw::PlaceKind) -> Option<raw::PlaceId> {
    match kind {
        raw::PlaceKind::StructField { base, .. }
        | raw::PlaceKind::EnumPayload { base, .. }
        | raw::PlaceKind::FixedArrayConstant { base, .. } => Some(*base),
        _ => None,
    }
}
fn initialize_projections(
    root: raw::PlaceId,
    function: &raw::Function,
    states: &mut [PlaceState],
    completion: usize,
) {
    let mut visited = vec![false; function.places.len()];
    let mut queue = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        let Ok(parent_index) = usize::try_from(parent.0) else { continue };
        if parent_index >= visited.len() || visited[parent_index] {
            continue;
        }
        visited[parent_index] = true;
        for (index, place) in function.places.iter().enumerate() {
            if projection_base(&place.kind) == Some(parent) && !visited[index] {
                states[index] =
                    PlaceState { kind: PlaceStateKind::Initialized, completion: Some(completion) };
                queue.push_back(raw::PlaceId(u32::try_from(index).expect("bounded place index")));
            }
        }
    }
}

fn mark_ancestors_partial(
    mut place: raw::PlaceId,
    function: &raw::Function,
    states: &mut [PlaceState],
) {
    let mut visited = vec![false; function.places.len()];
    while let Some(base) =
        function.places.get(place.0 as usize).and_then(|place| projection_base(&place.kind))
    {
        let Ok(index) = usize::try_from(base.0) else { break };
        if index >= visited.len() || visited[index] {
            break;
        }
        visited[index] = true;
        if states[index].kind == PlaceStateKind::Initialized {
            states[index].kind = PlaceStateKind::PartiallyMoved;
        }
        place = base;
    }
}
fn mark_ancestors_partially_initialized(
    mut place: raw::PlaceId,
    function: &raw::Function,
    states: &mut [PlaceState],
    completion: usize,
) {
    let mut visited = vec![false; function.places.len()];
    while let Some(base) =
        function.places.get(place.0 as usize).and_then(|place| projection_base(&place.kind))
    {
        let Ok(index) = usize::try_from(base.0) else {
            break;
        };
        if index >= visited.len() || visited[index] {
            break;
        }
        visited[index] = true;
        if states[index].kind == PlaceStateKind::Uninitialized {
            states[index].kind = PlaceStateKind::PartiallyInitialized;
            states[index].completion = Some(completion);
        }
        place = base;
    }
}
fn places_overlap(left: raw::PlaceId, right: raw::PlaceId, places: &[raw::Place]) -> bool {
    fn ancestors(mut id: raw::PlaceId, places: &[raw::Place]) -> Vec<raw::PlaceId> {
        let mut out = vec![id];
        let mut visited = vec![false; places.len()];
        while let Some(base) =
            places.get(id.0 as usize).and_then(|place| projection_base(&place.kind))
        {
            let Ok(index) = usize::try_from(id.0) else { break };
            if index >= visited.len() || visited[index] || out.len() > MAX_PLACES_PER_FUNCTION {
                break;
            }
            visited[index] = true;
            out.push(base);
            id = base;
        }
        out
    }
    let left = ancestors(left, places);
    let right = ancestors(right, places);
    left.iter().any(|id| right.contains(id))
}
fn overlaps_active(
    place: raw::PlaceId,
    active: &[Option<(raw::PlaceId, raw::BorrowAccess)>],
    places: &[raw::Place],
) -> bool {
    active.iter().flatten().any(|(borrowed, _)| places_overlap(place, *borrowed, places))
}
fn overlaps_exclusive(
    place: raw::PlaceId,
    active: &[Option<(raw::PlaceId, raw::BorrowAccess)>],
    places: &[raw::Place],
) -> bool {
    active.iter().flatten().any(|(borrowed, access)| {
        *access == raw::BorrowAccess::Exclusive && places_overlap(place, *borrowed, places)
    })
}
fn conflicts(
    definition: &raw::BorrowDefinition,
    active: &[Option<(raw::PlaceId, raw::BorrowAccess)>],
    places: &[raw::Place],
) -> bool {
    active.iter().flatten().any(|(place, access)| {
        places_overlap(definition.place, *place, places)
            && (definition.access == raw::BorrowAccess::Exclusive
                || *access == raw::BorrowAccess::Exclusive)
    })
}

fn compute_dominators(predecessors: &[Vec<usize>], reachable: &[bool]) -> Vec<BTreeSet<usize>> {
    let all = (0..predecessors.len()).filter(|index| reachable[*index]).collect::<BTreeSet<_>>();
    let mut result = vec![all.clone(); predecessors.len()];
    result[0] = BTreeSet::from([0]);
    loop {
        let mut changed = false;
        for block in 1..predecessors.len() {
            if !reachable[block] {
                continue;
            }
            let mut next = all.clone();
            for predecessor in &predecessors[block] {
                next = next.intersection(&result[*predecessor]).copied().collect();
            }
            next.insert(block);
            if next != result[block] {
                result[block] = next;
                changed = true;
            }
        }
        if !changed {
            return result;
        }
    }
}

fn verify_use(
    value: raw::ValueId,
    block: usize,
    position: usize,
    span: Span,
    values: &[ValueInfo],
    dominators: &[BTreeSet<usize>],
    errors: &mut Errors,
) {
    let Some(info) = value_info(values, value) else {
        errors.push(error_at(
            "ZRYNA-I3008",
            span,
            "operation references a foreign or undefined value",
            "use a value defined by the containing function",
        ));
        return;
    };
    let dominates = match info.site {
        DefinitionSite::FunctionParameter => true,
        DefinitionSite::BlockParameter(definition_block) => {
            dominators[block].contains(&definition_block)
        }
        DefinitionSite::Instruction(definition_block, definition_position) => {
            definition_block == block && definition_position < position
                || definition_block != block && dominators[block].contains(&definition_block)
        }
    };
    if !dominates {
        errors.push(error_at(
            "ZRYNA-I3008",
            span,
            "value definition does not dominate its use",
            "use only earlier same-block or dominating definitions",
        ));
    }
}

#[allow(clippy::too_many_lines)]
fn verify_operation_types(
    instruction: &raw::Instruction,
    function: &raw::Function,
    values: &[ValueInfo],
    layouts: &VerifiedLayouts,
    errors: &mut Errors,
) {
    use raw::InstructionKind as I;

    let category = |value: raw::ValueId| {
        value_info(values, value)
            .and_then(|info| layout_type(layouts, info.ty))
            .map(zryna_layout::VerifiedType::category)
    };
    let result_type = instruction.result.map(|value| value.ty);
    let result = result_type
        .and_then(|ty| layout_type(layouts, ty))
        .map(zryna_layout::VerifiedType::category);
    let place_type =
        |place: raw::PlaceId| function.places.get(place.0 as usize).map(|place| place.ty);
    let valid = match &instruction.kind {
        I::BoolLiteral(_) => result == Some(TypeCategory::Bool),
        I::I32Literal(_) => result == Some(TypeCategory::I32),
        I::I32Add { lhs, rhs } | I::I32Sub { lhs, rhs } | I::I32Mul { lhs, rhs } => {
            category(*lhs) == Some(TypeCategory::I32)
                && category(*rhs) == Some(TypeCategory::I32)
                && result == Some(TypeCategory::I32)
        }
        I::I32Neg { operand } => {
            category(*operand) == Some(TypeCategory::I32) && result == Some(TypeCategory::I32)
        }
        I::I32LtS { lhs, rhs }
        | I::I32LeS { lhs, rhs }
        | I::I32GtS { lhs, rhs }
        | I::I32GeS { lhs, rhs } => {
            category(*lhs) == Some(TypeCategory::I32)
                && category(*rhs) == Some(TypeCategory::I32)
                && result == Some(TypeCategory::Bool)
        }
        I::Eq { lhs, rhs } | I::Ne { lhs, rhs } => {
            value_info(values, *lhs).zip(value_info(values, *rhs)).is_some_and(|(left, right)| {
                left.ty == right.ty
                    && matches!(category(*lhs), Some(TypeCategory::Bool | TypeCategory::I32))
                    && result == Some(TypeCategory::Bool)
            })
        }
        I::StructConstruct { fields, .. } => {
            result_type.and_then(|ty| layout_type(layouts, ty)).is_some_and(|ty| {
                ty.category() == TypeCategory::Struct
                    && fields.len() == ty.fields().len()
                    && fields.iter().zip(ty.fields()).all(|(value, field)| {
                        value_info(values, *value)
                            .is_some_and(|info| info.ty.0 == field.ty().index())
                    })
            })
        }
        I::EnumConstruct { variant, payload, .. } => result_type
            .and_then(|ty| layout_type(layouts, ty))
            .and_then(|ty| ty.variants().get(*variant as usize).copied())
            .is_some_and(|record| match (payload, record.payload()) {
                (None, None) => true,
                (Some(value), Some(ty)) => {
                    value_info(values, *value).is_some_and(|info| info.ty.0 == ty.index())
                }
                _ => false,
            }),
        I::FixedArrayConstruct { elements, .. } => {
            result_type.and_then(|ty| layout_type(layouts, ty)).is_some_and(|ty| {
                ty.category() == TypeCategory::FixedArray
                    && ty.array_length() == Some(elements.len() as u64)
                    && ty.referenced_type().is_some_and(|element| {
                        elements.iter().all(|value| {
                            value_info(values, *value)
                                .is_some_and(|info| info.ty.0 == element.index())
                        })
                    })
            })
        }
        I::CopyFromPlace { place } | I::MoveFromPlace { place } | I::ClonePlace { place, .. } => {
            place_type(*place) == result_type
        }
        I::InitializePlace { place, value } | I::ReplacePlace { place, value, .. } => {
            place_type(*place) == value_info(values, *value).map(|info| info.ty)
        }
        I::BorrowWrite { borrow, value } => {
            instruction.result.is_none()
                && borrow_definition(function, *borrow).is_some_and(|(referent, access)| {
                    access == raw::BorrowAccess::Exclusive
                        && layout_type(layouts, referent).is_some_and(|ty| ty.drop_kind() == 0)
                        && value_info(values, *value).is_some_and(|info| info.ty == referent)
                })
        }
        I::DropPlace { .. } | I::BeginBorrow(_) | I::EndBorrow { .. } => {
            instruction.result.is_none()
        }
        I::EnumDiscriminant { place } => {
            place_type(*place)
                .and_then(|ty| layout_type(layouts, ty))
                .is_some_and(|ty| ty.category() == TypeCategory::Enum)
                && result == Some(TypeCategory::I32)
        }
        I::FixedArrayIndexCopy { place, index, .. } => indexed_result(
            *place,
            *index,
            TypeCategory::FixedArray,
            function,
            values,
            layouts,
            result_type,
        ),
        I::VecIndexCopy { place, index, .. } => indexed_result(
            *place,
            *index,
            TypeCategory::Vec,
            function,
            values,
            layouts,
            result_type,
        ),
        I::StringClone { place, .. } => {
            place_type(*place) == result_type && result == Some(TypeCategory::String)
        }
        I::StringConcat { left, right, .. } => {
            place_type(*left) == result_type
                && place_type(*right) == result_type
                && result == Some(TypeCategory::String)
        }
        I::VecConstruct { elements, .. } => {
            container_elements(elements, TypeCategory::Vec, result_type, values, layouts)
        }
        I::VecPush { vector, value, .. } => {
            instruction.result.is_none()
                && function
                    .places
                    .get(vector.0 as usize)
                    .and_then(|place| layout_type(layouts, place.ty))
                    .is_some_and(|ty| {
                        ty.category() == TypeCategory::Vec
                            && ty.referenced_type().is_some_and(|element| {
                                value_info(values, *value)
                                    .is_some_and(|info| info.ty.0 == element.index())
                            })
                    })
        }
        I::SharedConstruct { value, .. } => {
            result_type.and_then(|ty| layout_type(layouts, ty)).is_some_and(|ty| {
                ty.category() == TypeCategory::Shared
                    && ty.referenced_type().is_some_and(|payload| {
                        value_info(values, *value).is_some_and(|info| info.ty.0 == payload.index())
                    })
            })
        }
        I::SharedClone { place, .. } => {
            place_type(*place) == result_type && result == Some(TypeCategory::Shared)
        }
        I::WeakDowngrade { place, .. } => place_type(*place)
            .and_then(|ty| layout_type(layouts, ty))
            .zip(result_type.and_then(|ty| layout_type(layouts, ty)))
            .is_some_and(|(shared, weak)| {
                shared.category() == TypeCategory::Shared
                    && weak.category() == TypeCategory::Weak
                    && shared.referenced_type() == weak.referenced_type()
            }),
        I::WeakClone { place, .. } => {
            place_type(*place) == result_type && result == Some(TypeCategory::Weak)
        }
        I::BorrowRead { borrow } => borrow_definition(function, *borrow).is_some_and(|(ty, _)| {
            result_type == Some(ty)
                && layout_type(layouts, ty).is_some_and(|ty| ty.drop_kind() == 0)
        }),
        I::DirectCall { .. } => true,
    };
    if !valid {
        errors.push(error_at(
            "ZRYNA-I3005",
            instruction.span,
            "instruction operands or result have an invalid sealed type",
            "use the exact operand and result types required by the operation",
        ));
    }
}

fn value_info(values: &[ValueInfo], id: raw::ValueId) -> Option<ValueInfo> {
    usize::try_from(id.0).ok().and_then(|index| values.get(index)).copied()
}

fn indexed_result(
    place: raw::PlaceId,
    index: raw::ValueId,
    category: TypeCategory,
    function: &raw::Function,
    values: &[ValueInfo],
    layouts: &VerifiedLayouts,
    result: Option<raw::TypeId>,
) -> bool {
    value_info(values, index)
        .and_then(|info| layout_type(layouts, info.ty))
        .is_some_and(|ty| ty.category() == TypeCategory::I32)
        && function
            .places
            .get(place.0 as usize)
            .and_then(|place| layout_type(layouts, place.ty))
            .is_some_and(|ty| {
                ty.category() == category
                    && ty.referenced_type().is_some_and(|element| {
                        result.is_some_and(|result| result.0 == element.index())
                            && layout_type(layouts, raw::TypeId(element.index()))
                                .is_some_and(|element| element.drop_kind() == 0)
                    })
            })
}

fn container_elements(
    elements: &[raw::ValueId],
    category: TypeCategory,
    result: Option<raw::TypeId>,
    values: &[ValueInfo],
    layouts: &VerifiedLayouts,
) -> bool {
    result.and_then(|ty| layout_type(layouts, ty)).is_some_and(|ty| {
        ty.category() == category
            && ty.referenced_type().is_some_and(|element| {
                elements.iter().all(|value| {
                    value_info(values, *value).is_some_and(|info| info.ty.0 == element.index())
                })
            })
    })
}

fn borrow_definition(
    function: &raw::Function,
    id: raw::BorrowId,
) -> Option<(raw::TypeId, raw::BorrowAccess)> {
    if let Some(parameter) = function.borrow_parameters.iter().find(|value| value.id == id) {
        return Some((parameter.referent, parameter.access));
    }
    function.blocks.iter().flat_map(|block| &block.instructions).find_map(|instruction| {
        match &instruction.kind {
            raw::InstructionKind::BeginBorrow(definition) if definition.id == id => function
                .places
                .get(definition.place.0 as usize)
                .map(|place| (place.ty, definition.access)),
            _ => None,
        }
    })
}

fn lexical_borrow_place(function: &raw::Function, id: raw::BorrowId) -> Option<raw::PlaceId> {
    function.blocks.iter().flat_map(|block| &block.instructions).find_map(|instruction| {
        match &instruction.kind {
            raw::InstructionKind::BeginBorrow(definition) if definition.id == id => {
                Some(definition.place)
            }
            _ => None,
        }
    })
}

fn terminator_edges(kind: &raw::Terminator) -> Vec<&raw::Edge> {
    match kind {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => vec![],
        raw::Terminator::Jump(edge) => vec![edge],
        raw::Terminator::Branch { when_true, when_false, .. } => vec![when_true, when_false],
        raw::Terminator::EnumMatch { arms, .. } => arms.iter().map(|arm| &arm.edge).collect(),
        raw::Terminator::WeakUpgradeBranch { success, expired, .. } => vec![success, expired],
    }
}

fn instruction_operands(kind: &raw::InstructionKind) -> Vec<raw::ValueId> {
    use raw::InstructionKind as I;
    match kind {
        I::I32Add { lhs, rhs }
        | I::I32Sub { lhs, rhs }
        | I::I32Mul { lhs, rhs }
        | I::Eq { lhs, rhs }
        | I::Ne { lhs, rhs }
        | I::I32LtS { lhs, rhs }
        | I::I32LeS { lhs, rhs }
        | I::I32GtS { lhs, rhs }
        | I::I32GeS { lhs, rhs } => vec![*lhs, *rhs],
        I::I32Neg { operand } => vec![*operand],
        I::DirectCall { arguments, .. } => arguments
            .iter()
            .filter_map(|argument| match argument {
                raw::CallArgument::Value(value) => Some(*value),
                raw::CallArgument::Borrow(_) => None,
            })
            .collect(),
        I::StructConstruct { fields, .. } => fields.clone(),
        I::EnumConstruct { payload, .. } => payload.iter().copied().collect(),
        I::FixedArrayConstruct { elements, .. } | I::VecConstruct { elements, .. } => {
            elements.clone()
        }
        I::InitializePlace { value, .. }
        | I::ReplacePlace { value, .. }
        | I::VecPush { value, .. }
        | I::SharedConstruct { value, .. }
        | I::BorrowWrite { value, .. } => vec![*value],
        I::FixedArrayIndexCopy { index, .. } | I::VecIndexCopy { index, .. } => vec![*index],
        _ => vec![],
    }
}

fn terminator_operands(kind: &raw::Terminator) -> Vec<raw::ValueId> {
    let mut values = terminator_edges(kind)
        .into_iter()
        .flat_map(|edge| edge.arguments.iter().copied())
        .collect::<Vec<_>>();
    match kind {
        raw::Terminator::Return { value, .. } => values.push(*value),
        raw::Terminator::Branch { condition, .. } => values.push(*condition),
        _ => {}
    }
    values
}

fn verify_instruction_shape(
    instruction: &raw::Instruction,
    function: &raw::Function,
    errors: &mut Errors,
) {
    use raw::InstructionKind as I;
    let effect_only = matches!(
        instruction.kind,
        I::InitializePlace { .. }
            | I::ReplacePlace { .. }
            | I::DropPlace { .. }
            | I::BeginBorrow(_)
            | I::BorrowWrite { .. }
            | I::EndBorrow { .. }
    );
    if effect_only == instruction.result.is_some() {
        errors.push(error_at(
            "ZRYNA-I3005",
            instruction.span,
            "instruction result shape does not match its operation",
            "emit a result exactly for value-producing operations",
        ));
    }
    let place_valid = |place: raw::PlaceId| (place.0 as usize) < function.places.len();
    let cleanup_valid = |plan: raw::CleanupPlanId| (plan.0 as usize) < function.cleanup_plans.len();
    let bad = match &instruction.kind {
        I::CopyFromPlace { place }
        | I::MoveFromPlace { place }
        | I::DropPlace { place }
        | I::EnumDiscriminant { place } => !place_valid(*place),
        I::ClonePlace { place, cleanup }
        | I::FixedArrayIndexCopy { place, cleanup, .. }
        | I::VecIndexCopy { place, cleanup, .. }
        | I::StringClone { place, cleanup }
        | I::SharedClone { place, cleanup }
        | I::WeakDowngrade { place, cleanup }
        | I::WeakClone { place, cleanup }
        | I::ReplacePlace { place, cleanup, .. } => {
            !place_valid(*place) || !cleanup_valid(*cleanup)
        }
        I::StringConcat { left, right, cleanup } => {
            !place_valid(*left) || !place_valid(*right) || !cleanup_valid(*cleanup)
        }
        I::VecPush { vector, cleanup, .. } => !place_valid(*vector) || !cleanup_valid(*cleanup),
        I::VecConstruct { cleanup, .. } | I::SharedConstruct { cleanup, .. } => {
            !cleanup_valid(*cleanup)
        }
        I::BeginBorrow(def) => !place_valid(def.place),
        _ => false,
    };
    if bad {
        errors.push(error_at(
            "ZRYNA-I3006",
            instruction.span,
            "instruction references a foreign place or cleanup plan",
            "use identities owned by the containing function",
        ));
    }
}

fn verify_terminator(
    terminator: &raw::SpannedTerminator,
    function: &raw::Function,
    errors: &mut Errors,
) {
    let block_ok = |edge: &raw::Edge| {
        (edge.target.0 as usize) < function.blocks.len()
            && edge.arguments.len() <= MAX_BLOCK_PARAMETERS
    };
    let cleanup_ok = |id: raw::CleanupPlanId| (id.0 as usize) < function.cleanup_plans.len();
    let valid = match &terminator.kind {
        raw::Terminator::Return { cleanup, .. } | raw::Terminator::Trap { cleanup, .. } => {
            cleanup_ok(*cleanup)
        }
        raw::Terminator::Jump(edge) => block_ok(edge),
        raw::Terminator::Branch { when_true, when_false, .. } => {
            block_ok(when_true) && block_ok(when_false)
        }
        raw::Terminator::EnumMatch { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| block_ok(&arm.edge))
        }
        raw::Terminator::WeakUpgradeBranch { success, expired, cleanup, .. } => {
            block_ok(success) && block_ok(expired) && cleanup_ok(*cleanup)
        }
    };
    if !valid {
        errors.push(error_at(
            "ZRYNA-I3007",
            terminator.span,
            "terminator contains an invalid edge or cleanup identity",
            "use local blocks, bounded edge arguments, and local cleanup plans",
        ));
    }
}

fn check_value(
    value: &raw::ValueDefinition,
    expected: &mut u32,
    file: FileId,
    sources: &SourceMap,
    layouts: &VerifiedLayouts,
    errors: &mut Errors,
) {
    if value.id.0 != *expected {
        errors.push(error_at(
            "ZRYNA-I3002",
            value.span,
            "value definitions are not in canonical dense order",
            "allocate parameters, block parameters, and results in canonical order",
        ));
    }
    *expected = expected.saturating_add(1);
    if layout_type(layouts, value.ty).is_none() {
        errors.push(error_at(
            "ZRYNA-I3003",
            value.span,
            "value names an unknown layout TypeId",
            "use a type from the exact sealed universe",
        ));
    }
    check_span(value.span, file, sources, errors);
}

fn verify_projection(
    place: &raw::Place,
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    errors: &mut Errors,
) {
    let places = &function.places;
    let valid = match place.kind {
        raw::PlaceKind::Parameter(parameter) => {
            function.parameters.get(parameter as usize).is_some_and(|value| value.ty == place.ty)
        }
        raw::PlaceKind::Local(local) => {
            local < u32::try_from(MAX_PLACES_PER_FUNCTION).expect("place limit fits u32")
        }
        raw::PlaceKind::Temporary(value) => function_value_type(function, value) == Some(place.ty),
        raw::PlaceKind::StructField { base, ordinal } => (base.0 < place.id.0)
            .then(|| places.get(base.0 as usize))
            .flatten()
            .and_then(|base| layout_type(layouts, base.ty))
            .and_then(|ty| ty.fields().get(ordinal as usize).copied())
            .is_some_and(|field| field.ordinal() == ordinal && field.ty().index() == place.ty.0),
        raw::PlaceKind::EnumPayload { base, variant } => (base.0 < place.id.0)
            .then(|| places.get(base.0 as usize))
            .flatten()
            .and_then(|base| layout_type(layouts, base.ty))
            .and_then(|ty| ty.variants().get(variant as usize).copied())
            .and_then(zryna_layout::VerifiedVariant::payload)
            .is_some_and(|ty| ty.index() == place.ty.0),
        raw::PlaceKind::FixedArrayConstant { base, index } => (base.0 < place.id.0)
            .then(|| places.get(base.0 as usize))
            .flatten()
            .and_then(|base| layout_type(layouts, base.ty))
            .is_some_and(|ty| {
                ty.array_length().is_some_and(|length| u64::from(index) < length)
                    && ty.referenced_type().is_some_and(|element| element.index() == place.ty.0)
            }),
    };
    if !valid {
        errors.push(error_at(
            "ZRYNA-I3006",
            place.span,
            "place projection does not match its sealed layout",
            "use an in-range source ordinal and the exact projected type",
        ));
    }
}

#[allow(clippy::too_many_lines)]
fn verify_calls(program: &raw::Program, errors: &mut Errors) {
    let function_count = program.modules.iter().map(|module| module.functions.len()).sum();
    let mut offsets = Vec::with_capacity(program.modules.len());
    let mut next = 0usize;
    for module in &program.modules {
        offsets.push(next);
        next += module.functions.len();
    }
    let mut graph = vec![Vec::<usize>::new(); function_count];
    for (module_index, module) in program.modules.iter().enumerate() {
        for (function_index, function) in module.functions.iter().enumerate() {
            let caller = offsets[module_index] + function_index;
            for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
                let raw::InstructionKind::DirectCall { callee, arguments, .. } = &instruction.kind
                else {
                    continue;
                };
                let Some(callee_module) = program.modules.get(callee.module.0 as usize) else {
                    call_error_at(instruction.span, "direct call names an unknown module", errors);
                    continue;
                };
                let Some(target) = callee_module.functions.get(callee.declaration as usize) else {
                    call_error_at(
                        instruction.span,
                        "direct call names an unknown function",
                        errors,
                    );
                    continue;
                };
                if target.id != *callee
                    || arguments.len() != target.parameters.len() + target.borrow_parameters.len()
                {
                    call_error_at(
                        instruction.span,
                        "direct call identity or arity does not match its callee",
                        errors,
                    );
                    continue;
                }
                let mut valid = true;
                let mut exclusive_arguments = Vec::<(raw::BorrowId, Option<raw::PlaceId>)>::new();
                for (argument, parameter) in arguments.iter().zip(&target.parameters) {
                    let raw::CallArgument::Value(value) = argument else {
                        valid = false;
                        break;
                    };
                    valid &= function_value_type(function, *value) == Some(parameter.ty);
                }
                for (argument, parameter) in
                    arguments.iter().skip(target.parameters.len()).zip(&target.borrow_parameters)
                {
                    let raw::CallArgument::Borrow(borrow) = argument else {
                        valid = false;
                        break;
                    };
                    valid &=
                        borrow_definition(function, *borrow).is_some_and(|(referent, access)| {
                            referent == parameter.referent && access == parameter.access
                        });
                    if parameter.access == raw::BorrowAccess::Exclusive {
                        let place = lexical_borrow_place(function, *borrow);
                        if exclusive_arguments.iter().any(|(prior, prior_place)| {
                            prior == borrow
                                || prior_place.zip(place).is_some_and(|(left, right)| {
                                    places_overlap(left, right, &function.places)
                                })
                        }) {
                            valid = false;
                        }
                        exclusive_arguments.push((*borrow, place));
                    }
                }
                valid &= instruction.result.is_some_and(|result| result.ty == target.result);
                if !valid {
                    call_error_at(
                        instruction.span,
                        "direct call argument or result type does not match its callee",
                        errors,
                    );
                }
                graph[caller].push(offsets[callee.module.0 as usize] + callee.declaration as usize);
            }
        }
    }
    let mut indegree = vec![0usize; function_count];
    for edges in &graph {
        for target in edges {
            indegree[*target] += 1;
        }
    }
    let mut queue = VecDeque::new();
    let mut depth = vec![1usize; function_count];
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            queue.push_back(index);
        }
    }
    let mut visited = 0usize;
    while let Some(function) = queue.pop_front() {
        visited += 1;
        for target in &graph[function] {
            depth[*target] = depth[*target].max(depth[function].saturating_add(1));
            if depth[*target] > MAX_STATIC_CALL_DEPTH {
                call_error("static call depth exceeds its limit", errors);
                return;
            }
            indegree[*target] -= 1;
            if indegree[*target] == 0 {
                queue.push_back(*target);
            }
        }
    }
    if visited != function_count {
        call_error("direct call graph contains a cycle", errors);
    }
}

fn function_value_type(function: &raw::Function, id: raw::ValueId) -> Option<raw::TypeId> {
    let mut values = function.parameters.iter().map(|value| value.ty).collect::<Vec<_>>();
    for block in &function.blocks {
        values.extend(block.parameters.iter().map(|value| value.ty));
        values.extend(
            block
                .instructions
                .iter()
                .filter_map(|instruction| instruction.result.map(|value| value.ty)),
        );
    }
    values.get(id.0 as usize).copied()
}

fn call_error(message: &'static str, errors: &mut Errors) {
    errors.push(error("ZRYNA-I3009", message, "use one acyclic exact-signature direct call graph"));
}
fn call_error_at(span: Span, message: &'static str, errors: &mut Errors) {
    errors.push(error_at(
        "ZRYNA-I3009",
        span,
        message,
        "use one acyclic exact-signature direct call graph",
    ));
}

fn verify_public_abi(
    program: &raw::Program,
    layouts: &VerifiedLayouts,
    errors: &mut Errors,
) -> (Option<zryna_abi::VerifiedScalarAbiModule>, Vec<Vec<Option<usize>>>) {
    let mut exports = Vec::new();
    let mut indices = Vec::new();
    let mut first_export_span = None;
    for module in &program.modules {
        let mut module_indices = Vec::new();
        for function in &module.functions {
            if let Some(name) = &function.entry_export {
                first_export_span.get_or_insert(function.span);
                if module.id != program.entry_module || !function.borrow_parameters.is_empty() {
                    errors.push(error_at(
                        "ZRYNA-I3009",
                        function.span,
                        "only borrow-free entry-module functions may claim public exports",
                        "keep aggregate and borrowed functions internal",
                    ));
                    module_indices.push(None);
                    continue;
                }
                let Some(parameters) = function
                    .parameters
                    .iter()
                    .map(|p| abi_type(layouts, p.ty))
                    .collect::<Option<Vec<_>>>()
                else {
                    errors.push(error_at(
                        "ZRYNA-I3009",
                        function.span,
                        "public export contains a non-scalar parameter",
                        "public DataOwnershipV1 ABI remains scalar bool/i32 v1",
                    ));
                    module_indices.push(None);
                    continue;
                };
                let Some(result) = abi_type(layouts, function.result) else {
                    errors.push(error_at(
                        "ZRYNA-I3009",
                        function.span,
                        "public export contains a non-scalar result",
                        "public DataOwnershipV1 ABI remains scalar bool/i32 v1",
                    ));
                    module_indices.push(None);
                    continue;
                };
                module_indices.push(Some(exports.len()));
                exports.push(raw_abi::Export::new(
                    name.clone(),
                    raw_abi::Signature::new(parameters, result),
                ));
            } else {
                module_indices.push(None);
            }
        }
        indices.push(module_indices);
    }
    if let Ok(abi) = verify_v1(raw_abi::Module::new(exports)) {
        (Some(abi), indices)
    } else {
        errors.push(first_export_span.map_or_else(
            || {
                error(
                    "ZRYNA-I3009",
                    "public scalar ABI claims are invalid",
                    "use unique portable scalar ABI v1 exports",
                )
            },
            |span| {
                error_at(
                    "ZRYNA-I3009",
                    span,
                    "public scalar ABI claims are invalid",
                    "use unique portable scalar ABI v1 exports",
                )
            },
        ));
        (None, indices)
    }
}

fn abi_type(layouts: &VerifiedLayouts, id: raw::TypeId) -> Option<raw_abi::Type> {
    match layout_type(layouts, id)?.category() {
        TypeCategory::Bool => Some(raw_abi::Type::Bool),
        TypeCategory::I32 => Some(raw_abi::Type::I32),
        _ => None,
    }
}
fn layout_type(
    layouts: &VerifiedLayouts,
    id: raw::TypeId,
) -> Option<zryna_layout::VerifiedType<'_>> {
    layouts.types().nth(id.0 as usize)
}
fn check_span(span: Span, file: FileId, sources: &SourceMap, errors: &mut Errors) {
    if !sources.resolve(span).is_ok_and(|resolved| resolved.source().id() == file) {
        errors.push(error(
            "ZRYNA-I3004",
            "span is not owned by its containing module",
            "use an exact span from the module's final source file",
        ));
    }
}
fn same_logical_universe(a: &VerifiedLayouts, b: &VerifiedLayouts) -> bool {
    let mut left = a.types();
    let mut right = b.types();
    if left.len() != right.len() {
        return false;
    }
    left.all(|x| {
        right.next().is_some_and(|y| {
            x.id().index() == y.id().index()
                && x.category() == y.category()
                && x.nominal_identity() == y.nominal_identity()
                && x.fields()
                    .iter()
                    .map(|f| (f.ordinal(), f.ty().index()))
                    .eq(y.fields().iter().map(|f| (f.ordinal(), f.ty().index())))
                && x.variants()
                    .iter()
                    .map(|v| (v.ordinal(), v.payload().map(LayoutTypeId::index)))
                    .eq(y
                        .variants()
                        .iter()
                        .map(|v| (v.ordinal(), v.payload().map(LayoutTypeId::index))))
                && x.array_length() == y.array_length()
                && x.referenced_type().map(LayoutTypeId::index)
                    == y.referenced_type().map(LayoutTypeId::index)
        })
    })
}
fn aggregate_operand_count(kind: &raw::InstructionKind) -> usize {
    match kind {
        raw::InstructionKind::StructConstruct { fields, .. } => fields.len(),
        raw::InstructionKind::FixedArrayConstruct { elements, .. }
        | raw::InstructionKind::VecConstruct { elements, .. } => elements.len(),
        raw::InstructionKind::EnumConstruct { payload, .. } => usize::from(payload.is_some()),
        _ => 0,
    }
}
fn terminator_edge_count(kind: &raw::Terminator) -> usize {
    match kind {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
        raw::Terminator::Jump(_) => 1,
        raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
        raw::Terminator::EnumMatch { arms, .. } => arms.len(),
    }
}
fn checked_add(current: usize, extra: usize, label: &str, errors: &mut Errors) -> usize {
    current.checked_add(extra).unwrap_or_else(|| {
        errors.limit(label, usize::MAX);
        usize::MAX
    })
}
fn error(code: &'static str, message: impl Into<String>, guidance: &'static str) -> Diagnostic {
    Diagnostic::error(code, None, message, guidance)
}
fn error_at(
    code: &'static str,
    span: Span,
    message: impl Into<String>,
    guidance: &'static str,
) -> Diagnostic {
    Diagnostic::error_at(code, span, message, guidance)
}
fn error_with_optional_span(
    code: &'static str,
    span: Option<Span>,
    message: impl Into<String>,
    guidance: &'static str,
) -> Diagnostic {
    let message = message.into();
    match span {
        Some(span) => error_at(code, span, message, guidance),
        None => error(code, message, guidance),
    }
}

#[derive(Default)]
struct Errors {
    diagnostics: Vec<Diagnostic>,
    exhausted: bool,
}
impl Errors {
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.exhausted {
            return;
        }
        if self.diagnostics.len() < MAX_DIAGNOSTICS - 1 {
            self.diagnostics.push(diagnostic);
        } else {
            self.diagnostics.push(error(
                "ZRYNA-I3202",
                format!(
                    "DataOwnershipV1 verification reached its diagnostic limit of {MAX_DIAGNOSTICS}"
                ),
                "fix retained diagnostics before verifying again",
            ));
            self.exhausted = true;
        }
    }
    fn limit(&mut self, label: &str, maximum: usize) {
        if !self.exhausted {
            self.diagnostics.push(error(
                "ZRYNA-I3201",
                format!("DataOwnershipV1 {label} exceeds its limit of {maximum}"),
                "reduce the program before IR verification",
            ));
            self.exhausted = true;
        }
    }
    fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
    fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests;
