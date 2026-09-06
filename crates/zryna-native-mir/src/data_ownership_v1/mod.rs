//! Independently verified Linux x86-64 native MIR for `DataOwnershipV1`.

use zryna_diagnostics::Diagnostic;

mod lower;
pub mod raw;
mod verify;

/// Opaque verified native MIR. Raw claims cannot be recovered or mutated.
#[derive(Clone, Debug)]
pub struct VerifiedMirModule {
    program: raw::Program,
}

impl VerifiedMirModule {
    /// Iterates exact verified Linux layout records.
    #[must_use]
    pub fn types(&self) -> impl ExactSizeIterator<Item = VerifiedType<'_>> {
        self.program.types.iter().map(|ty| VerifiedType { ty })
    }
    /// Iterates immutable verified function views.
    #[must_use]
    pub fn functions(&self) -> impl ExactSizeIterator<Item = VerifiedFunction<'_>> {
        self.program.functions.iter().map(|function| VerifiedFunction { function })
    }
    /// Returns the exact approved runtime symbol inventory.
    #[must_use]
    pub fn runtime_symbols(&self) -> impl ExactSizeIterator<Item = &str> {
        self.program.runtime_symbols.iter().map(String::as_str)
    }
}

/// Immutable verified native MIR function view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedFunction<'a> {
    function: &'a raw::Function,
}

impl<'a> VerifiedFunction<'a> {
    /// Returns the dense module/declaration identity.
    #[must_use]
    pub const fn identity(self) -> (u32, u32) {
        (self.function.module, self.function.declaration)
    }
    /// Returns the deterministic private native symbol.
    #[must_use]
    pub fn symbol(self) -> &'a str {
        &self.function.symbol
    }
    /// Iterates dense typed parameters.
    #[must_use]
    pub fn parameters(self) -> impl ExactSizeIterator<Item = VerifiedValue> + 'a {
        self.function.parameters.iter().copied().map(VerifiedValue::from)
    }
    /// Iterates dense typed borrow-address parameters after value parameters.
    #[must_use]
    pub fn borrow_parameters(self) -> impl ExactSizeIterator<Item = VerifiedBorrowParameter> + 'a {
        self.function.borrow_parameters.iter().map(|parameter| VerifiedBorrowParameter {
            id: parameter.id,
            referent: parameter.referent,
            access: parameter.access,
        })
    }
    /// Returns the sealed result type identity.
    #[must_use]
    pub const fn result_type(self) -> u32 {
        self.function.result_type
    }
    /// Iterates exact addressable places.
    #[must_use]
    pub fn places(self) -> impl ExactSizeIterator<Item = VerifiedPlace<'a>> {
        self.function.places.iter().map(|place| VerifiedPlace { place })
    }
    /// Iterates immutable verified blocks.
    #[must_use]
    pub fn blocks(self) -> impl ExactSizeIterator<Item = VerifiedBlock<'a>> {
        self.function.blocks.iter().map(|block| VerifiedBlock { block })
    }
    /// Returns the exact place count.
    #[must_use]
    pub fn place_count(self) -> usize {
        self.function.places.len()
    }
    /// Returns the exact cleanup-plan count.
    #[must_use]
    pub fn cleanup_plan_count(self) -> usize {
        self.function.cleanup_plans.len()
    }
    /// Returns one exact cleanup plan by dense identity.
    #[must_use]
    pub fn cleanup_plan(self, id: u32) -> Option<VerifiedCleanupPlan<'a>> {
        self.function
            .cleanup_plans
            .get(usize::try_from(id).ok()?)
            .map(|plan| VerifiedCleanupPlan { plan })
    }
    /// Returns a value type retained by parameters, block parameters, or results.
    #[must_use]
    pub fn value_type(self, id: u32) -> Option<u32> {
        self.function
            .parameters
            .iter()
            .chain(self.function.blocks.iter().flat_map(|block| block.parameters.iter()))
            .chain(self.function.blocks.iter().flat_map(|block| {
                block.operations.iter().filter_map(|operation| operation.result.as_ref())
            }))
            .find(|value| value.id == id)
            .map(|value| value.ty)
    }
}

/// Immutable verified cleanup plan.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedCleanupPlan<'a> {
    plan: &'a raw::CleanupPlan,
}

#[allow(missing_docs)]
impl<'a> VerifiedCleanupPlan<'a> {
    #[must_use]
    pub const fn id(self) -> u32 {
        self.plan.id
    }
    #[must_use]
    pub fn actions(self) -> impl DoubleEndedIterator<Item = VerifiedDropAction> + 'a {
        self.plan.actions.iter().copied().map(VerifiedDropAction::from)
    }
}

/// Immutable verified cleanup action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedDropAction {
    place: u32,
    kind: raw::DropKind,
}

#[allow(missing_docs)]
impl VerifiedDropAction {
    #[must_use]
    pub const fn place(self) -> u32 {
        self.place
    }
    #[must_use]
    pub const fn kind(self) -> raw::DropKind {
        self.kind
    }
}

impl From<raw::DropAction> for VerifiedDropAction {
    fn from(action: raw::DropAction) -> Self {
        Self { place: action.place, kind: action.kind }
    }
}

/// Immutable verified native MIR block view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedBlock<'a> {
    block: &'a raw::Block,
}

impl<'a> VerifiedBlock<'a> {
    /// Returns the dense block identity.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.block.id
    }
    /// Returns the exact operation count.
    #[must_use]
    pub fn operation_count(self) -> usize {
        self.block.operations.len()
    }
    /// Iterates dense typed block parameters.
    #[must_use]
    pub fn parameters(self) -> impl ExactSizeIterator<Item = VerifiedValue> + 'a {
        self.block.parameters.iter().copied().map(VerifiedValue::from)
    }
    /// Iterates immutable verified operations.
    #[must_use]
    pub fn operations(self) -> impl ExactSizeIterator<Item = VerifiedOperation<'a>> {
        self.block.operations.iter().map(|operation| VerifiedOperation { operation })
    }
    /// Returns the closed terminator.
    #[must_use]
    pub const fn terminator(self) -> &'a raw::Terminator {
        &self.block.terminator
    }
    /// Returns the exact terminator cleanup plan, when present.
    #[must_use]
    pub const fn cleanup(self) -> Option<u32> {
        self.block.cleanup
    }
}

/// Immutable verified Linux layout view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedType<'a> {
    ty: &'a raw::Type,
}

#[allow(missing_docs)]
impl<'a> VerifiedType<'a> {
    #[must_use]
    pub const fn id(self) -> u32 {
        self.ty.id
    }
    #[must_use]
    pub const fn category(self) -> raw::TypeCategory {
        self.ty.category
    }
    #[must_use]
    pub const fn size(self) -> u64 {
        self.ty.size
    }
    #[must_use]
    pub const fn alignment(self) -> u64 {
        self.ty.alignment
    }
    #[must_use]
    pub const fn drop_kind(self) -> u32 {
        self.ty.drop_kind
    }
    #[must_use]
    pub const fn runtime_kind(self) -> u32 {
        self.ty.runtime_kind
    }
    #[must_use]
    pub fn fields(self) -> &'a [raw::Field] {
        &self.ty.fields
    }
    #[must_use]
    pub fn variants(self) -> &'a [raw::Variant] {
        &self.ty.variants
    }
    #[must_use]
    pub const fn array_stride(self) -> Option<u64> {
        self.ty.array_stride
    }
    #[must_use]
    pub const fn array_length(self) -> Option<u64> {
        self.ty.array_length
    }
    #[must_use]
    pub const fn enum_payload(self) -> Option<(u64, u64)> {
        self.ty.enum_payload
    }
    #[must_use]
    pub const fn referenced_type(self) -> Option<u32> {
        self.ty.referenced_type
    }
}

/// Dense typed value view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedValue {
    id: u32,
    ty: u32,
}

/// Dense borrow-address parameter view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedBorrowParameter {
    id: u32,
    referent: u32,
    access: raw::BorrowAccess,
}

#[allow(missing_docs)]
impl VerifiedBorrowParameter {
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }
    #[must_use]
    pub const fn referent(self) -> u32 {
        self.referent
    }
    #[must_use]
    pub const fn access(self) -> raw::BorrowAccess {
        self.access
    }
}

#[allow(missing_docs)]
impl VerifiedValue {
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }
    #[must_use]
    pub const fn ty(self) -> u32 {
        self.ty
    }
}

impl From<raw::Value> for VerifiedValue {
    fn from(value: raw::Value) -> Self {
        Self { id: value.id, ty: value.ty }
    }
}

/// Immutable addressable place view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedPlace<'a> {
    place: &'a raw::Place,
}

#[allow(missing_docs)]
impl<'a> VerifiedPlace<'a> {
    #[must_use]
    pub const fn id(self) -> u32 {
        self.place.id
    }
    #[must_use]
    pub const fn ty(self) -> u32 {
        self.place.ty
    }
    #[must_use]
    pub const fn kind(self) -> &'a raw::PlaceKind {
        &self.place.kind
    }
}

/// Immutable verified operation view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedOperation<'a> {
    operation: &'a raw::Operation,
}

#[allow(missing_docs)]
impl<'a> VerifiedOperation<'a> {
    #[must_use]
    pub const fn opcode(self) -> raw::Opcode {
        self.operation.opcode
    }
    #[must_use]
    pub fn result(self) -> Option<VerifiedValue> {
        self.operation.result.map(VerifiedValue::from)
    }
    #[must_use]
    pub fn values(self) -> &'a [u32] {
        &self.operation.values
    }
    #[must_use]
    pub fn places(self) -> &'a [u32] {
        &self.operation.places
    }
    #[must_use]
    pub fn borrows(self) -> &'a [u32] {
        &self.operation.borrows
    }
    #[must_use]
    pub const fn callee(self) -> Option<(u32, u32)> {
        self.operation.callee
    }
    #[must_use]
    pub fn runtime_symbol(self) -> Option<&'a str> {
        self.operation.runtime_symbol.as_deref()
    }
    #[must_use]
    pub const fn cleanup(self) -> Option<u32> {
        self.operation.cleanup
    }
    #[must_use]
    pub fn call_arguments(self) -> impl ExactSizeIterator<Item = VerifiedCallArgument> + 'a {
        self.operation.call_arguments.iter().map(|argument| match argument {
            raw::CallArgument::Value(id) => VerifiedCallArgument::Value(*id),
            raw::CallArgument::Borrow(id) => VerifiedCallArgument::Borrow(*id),
        })
    }
    #[must_use]
    pub const fn borrow_type(self) -> Option<u32> {
        self.operation.borrow_type
    }
    #[must_use]
    pub fn immediate(self) -> VerifiedImmediate<'a> {
        match &self.operation.immediate {
            raw::Immediate::None => VerifiedImmediate::None,
            raw::Immediate::Bool(value) => VerifiedImmediate::Bool(*value),
            raw::Immediate::I32(value) => VerifiedImmediate::I32(*value),
            raw::Immediate::Utf8(value) => VerifiedImmediate::Utf8(value),
            raw::Immediate::Variant(value) => VerifiedImmediate::Variant(*value),
        }
    }
}

/// Exact verified direct-call argument order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedCallArgument {
    /// Ordinary typed SSA value.
    Value(u32),
    /// Borrow-address carrier.
    Borrow(u32),
}

/// Closed immutable literal or constructor payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum VerifiedImmediate<'a> {
    None,
    Bool(bool),
    I32(i32),
    Utf8(&'a [u8]),
    Variant(u32),
}

/// Lowers sealed Universal IR one-for-one and invokes the mandatory independent verifier.
pub use lower::{lower, lower_unverified};
/// Verifies untrusted native MIR claims against exact layout and runtime ABI authorities.
pub use verify::verify;

fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        code,
        None,
        message,
        "provide canonical MIR derived from the sealed DataOwnershipV1 program",
    )
}
