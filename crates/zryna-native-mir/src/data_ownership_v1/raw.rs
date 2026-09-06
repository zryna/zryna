//! Untrusted native MIR claims for the internal `DataOwnershipV1` profile.

#![allow(missing_docs)]

/// Claimed authority bindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authority {
    pub type_universe: [u8; 32],
    pub linux_layout: [u8; 32],
    pub runtime_identifier: String,
}

/// Complete untrusted MIR program.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub authority: Authority,
    pub types: Vec<Type>,
    pub functions: Vec<Function>,
    pub runtime_symbols: Vec<String>,
}

/// Closed stored type category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeCategory {
    Bool,
    I32,
    Struct,
    Enum,
    FixedArray,
    String,
    Vec,
    Shared,
    Weak,
}

/// Claimed exact Linux x86-64 layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Type {
    pub id: u32,
    pub category: TypeCategory,
    pub size: u64,
    pub alignment: u64,
    pub drop_kind: u32,
    pub runtime_kind: u32,
}

/// One untrusted function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub module: u32,
    pub declaration: u32,
    pub symbol: String,
    pub parameters: Vec<Value>,
    pub borrow_parameters: Vec<BorrowParameter>,
    pub result_type: u32,
    pub places: Vec<Place>,
    pub blocks: Vec<Block>,
    pub cleanup_plans: Vec<CleanupPlan>,
}

/// Dense typed SSA value definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Value {
    pub id: u32,
    pub ty: u32,
}

/// Native address carrier access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BorrowAccess {
    Shared,
    Exclusive,
}

/// Dense borrow-address parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BorrowParameter {
    pub id: u32,
    pub referent: u32,
    pub access: BorrowAccess,
}

/// Addressable native place.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Place {
    pub id: u32,
    pub ty: u32,
    pub kind: PlaceKind,
}

/// Closed native address derivation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlaceKind {
    Parameter(u32),
    Local(u32),
    Temporary(u32),
    Field { base: u32, offset: u64 },
    EnumPayload { base: u32, offset: u64, variant: u32 },
    ArrayElement { base: u32, offset: u64 },
}

/// One MIR basic block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub id: u32,
    pub parameters: Vec<Value>,
    pub operations: Vec<Operation>,
    pub terminator: Terminator,
    pub cleanup: Option<u32>,
}

/// Closed native operation identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opcode {
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
    Call,
    Construct,
    Copy,
    Move,
    Clone,
    Initialize,
    Replace,
    Drop,
    Discriminant,
    Index,
    String,
    StringConcat,
    VecConstruct,
    VecPush,
    SharedConstruct,
    SharedClone,
    WeakDowngrade,
    WeakClone,
    BeginBorrow,
    BeginIndexedBorrow,
    ProjectBorrow,
    BindBorrow,
    BorrowReplace,
    BorrowRead,
    BorrowWrite,
    EndBorrow,
}

/// One typed native operation with closed operand roles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub opcode: Opcode,
    pub result: Option<Value>,
    pub values: Vec<u32>,
    pub places: Vec<u32>,
    pub borrows: Vec<u32>,
    pub callee: Option<(u32, u32)>,
    pub runtime_symbol: Option<String>,
    pub cleanup: Option<u32>,
}

/// One target edge and parallel value arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub target: u32,
    pub arguments: Vec<u32>,
}

/// Closed native control-flow operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Terminator {
    Return(u32),
    Jump(Edge),
    Branch { condition: u32, when_true: Edge, when_false: Edge },
    EnumMatch { place: u32, arms: Vec<(u32, Edge)> },
    WeakUpgrade { weak: u32, success: Edge, expired: Edge, runtime_symbol: String },
    Trap(u8),
}

/// Exact cleanup plan referenced by verified fallible operations or exits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupPlan {
    pub id: u32,
    pub actions: Vec<DropAction>,
}

/// Closed cleanup action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DropAction {
    pub place: u32,
    pub kind: DropKind,
}

/// Native cleanup behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DropKind {
    Place,
    VecPrefix,
    AggregatePrefix,
    GenericPrefix,
}
