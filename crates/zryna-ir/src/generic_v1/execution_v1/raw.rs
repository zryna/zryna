//! Untrusted full-profile execution claims; none of these rows grants authority.
pub use crate::generic_v1::raw::{Declaration, Module};
use zryna_source::UntrustedSpan;
#[path = "wire/raw_control.rs"]
mod control;
#[path = "wire/raw_operations.rs"]
mod operations;
pub use control::*;
pub use operations::*;
/// Accepted source language contract, independently authenticated by the issuer.
pub const CONTRACT: &str = "zryna.bounded-generics-option-result.v1";
/// A stored type identity or an unstored unit result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    /// Exact claimed complete type key index.
    Stored(u32),
    /// Unstored unit.
    Unit,
}
/// One function-global claimed value definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueDefinition {
    /// Dense value identity.
    pub id: u32,
    /// Exact claimed type.
    pub ty: ValueType,
}
/// Loan access mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Access {
    /// Read-only access.
    Shared,
    /// Exclusive access.
    Exclusive,
}
/// One claimed loan definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoanDefinition {
    /// Dense loan identity.
    pub id: u32,
    /// Stored referent type.
    pub referent: u32,
    /// Claimed access.
    pub access: Access,
}
/// A private signature parameter, in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Parameter {
    /// Ordinary value parameter.
    Value(ValueDefinition),
    /// Nonstored formal loan parameter.
    Loan(LoanDefinition),
}
/// A statically retained place projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaceKind {
    /// Original parameter ordinal.
    Parameter(u32),
    /// Original local ordinal.
    Local(u32),
    /// Prepared temporary value identity.
    Temporary(u32),
    /// Static declaration-ordered field.
    StructField {
        /// Parent place.
        base: u32,
        /// Field ordinal.
        ordinal: u32,
    },
    /// Active enum payload.
    EnumPayload {
        /// Parent place.
        base: u32,
        /// Variant ordinal.
        variant: u32,
    },
    /// Constant array element.
    FixedArrayConstant {
        /// Parent place.
        base: u32,
        /// Constant index.
        index: u32,
    },
}
/// One untrusted place row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Place {
    /// Dense place identity.
    pub id: u32,
    /// Stored referent type.
    pub ty: u32,
    /// Exact claimed original range.
    pub span: UntrustedSpan,
    /// Root or static projection.
    pub kind: PlaceKind,
}
/// A static loan projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticProjection {
    /// Declaration field ordinal.
    StructField(u32),
    /// Fixed variant ordinal.
    EnumPayload(u32),
    /// Constant array index.
    FixedArrayConstant(u32),
}
/// A claimed readable or mutable source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaceOrLoan {
    /// Place arena identity.
    Place(u32),
    /// Loan arena identity.
    Loan(u32),
}
/// Claimed origin; the independent source/state owners establish permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoanOrigin {
    /// Formal parameter ordinal.
    Formal(u32),
    /// Static place identity.
    Place(u32),
    /// Compiler-created inherited reborrow.
    InternalReborrow(u32),
    /// Projection of an existing loan.
    StaticProjection {
        /// Parent loan identity.
        parent: u32,
        /// Static projection.
        projection: StaticProjection,
    },
    /// Checked transient indexed access.
    Indexed {
        /// Indexed container.
        base: PlaceOrLoan,
        /// Prepared index value.
        index: u32,
    },
}
/// Authenticated source-region role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionRole {
    /// Lexically scoped borrow.
    Lexical,
    /// Compiler-created selected match arm.
    SelectedMatchArm,
    /// Compiler-created checked indexed access.
    TransientIndexedAccess,
}
/// Claimed source interval and compiler role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    /// Exact original range.
    pub span: UntrustedSpan,
    /// Claimed binding role.
    pub role: RegionRole,
}
/// One loan arena row, not live loan authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Loan {
    /// Dense loan identity.
    pub id: u32,
    /// Stored referent type.
    pub referent: u32,
    /// Claimed access.
    pub access: Access,
    /// Authenticated source-region claim.
    pub region: Region,
    /// Claimed origin.
    pub origin: LoanOrigin,
}
/// One instruction at an exact original source site.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    /// Exact original range.
    pub site_span: UntrustedSpan,
    /// Absent for effect-only operations.
    pub result: Option<ValueDefinition>,
    /// Complete closed operation vocabulary.
    pub operation: Operation,
}
/// One claimed dense basic block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    /// Dense block identity; entry is zero.
    pub id: u32,
    /// Incoming ordinary values in evaluation order.
    pub value_parameters: Vec<ValueDefinition>,
    /// Evaluation-ordered instructions.
    pub instructions: Vec<Instruction>,
    /// Exactly one explicit transfer.
    pub terminator: Terminator,
}
/// One closed generic instance or nongeneric source root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    /// Canonical unsigned-key-ordered identity.
    pub key: Vec<u8>,
    /// Exact original declaration range.
    pub original_span: UntrustedSpan,
    /// Independently authenticated scalar export claim.
    pub executable_export: Option<String>,
    /// Source-ordered private signature.
    pub parameters: Vec<Parameter>,
    /// Claimed result type.
    pub result_type: ValueType,
    /// Complete dense place arena.
    pub places: Vec<Place>,
    /// Complete dense loan arena.
    pub loans: Vec<Loan>,
    /// Complete dense block arena.
    pub blocks: Vec<Block>,
    /// Complete dense cleanup-plan arena.
    pub cleanup_plans: Vec<CleanupPlan>,
}
/// Complete untrusted full-profile claim, never an execution seal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// Exact accepted contract claim.
    pub language_contract: String,
    /// Original complete module inventory.
    pub modules: Vec<Module>,
    /// Original declaration inventory, including unused declarations.
    pub declarations: Vec<Declaration>,
    /// Canonical complete stored type keys.
    pub type_keys: Vec<Vec<u8>>,
    /// Source-universe identity claim.
    pub universe: [u8; 32],
    /// Linear32 target fingerprint claim.
    pub linear32: [u8; 32],
    /// Linux x86-64 target fingerprint claim.
    pub linux_x86_64: [u8; 32],
    /// Claimed original entry module.
    pub entry_module: u32,
    /// Canonically ordered closed functions and source roots.
    pub functions: Vec<Function>,
}
