//! Public immutable execution views; construction is confined to the checked owning seal.
use super::{VerifiedExecution, raw};
mod clone;
mod drops;
pub use drops::{VerifiedDropTarget, VerifiedDropTargets};
mod containers;
mod index;
mod indexed_loan;
pub use indexed_loan::VerifiedIndexedLoan;
mod push;
mod upgrade;
pub use containers::{VerifiedSharedConstruct, VerifiedVecConstruct, VerifiedWeakDowngrade};
pub use index::VerifiedIndexCopy;
pub use push::VerifiedVecPush;
pub use upgrade::{VerifiedUpgradeEdge, VerifiedWeakUpgrade};
mod signature;
mod storage;
pub use super::frontier::CloneNodeKind;
pub use clone::{VerifiedClone, VerifiedCloneNode, VerifiedCloneSource};
pub use signature::VerifiedParameter;
pub use storage::{
    VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedLoan, VerifiedLoanOrigin, VerifiedPlace,
    VerifiedValue,
};

/// Exact immutable function in one execution seal.
#[derive(Clone, Copy)]
pub struct VerifiedFunction<'p, 'a> {
    pub(super) owner: &'p VerifiedExecution<'a>,
    pub(super) index: usize,
}
/// Exact immutable block in one execution function.
#[derive(Clone, Copy)]
pub struct VerifiedBlock<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    function: usize,
    block: usize,
}
/// Same-seal source site metadata; this value alone grants no operation authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationSite {
    /// Owning function inventory index.
    pub function: usize,
    /// Dense block ID.
    pub block: u32,
    /// Instruction position.
    pub instruction: u32,
}
/// One source/type/state/plan-verified operation, never caller-constructible.
#[derive(Clone, Copy)]
pub struct VerifiedOperation<'p, 'a> {
    pub(super) owner: &'p VerifiedExecution<'a>,
    pub(super) index: usize,
}

impl<'a> VerifiedExecution<'a> {
    /// Complete retained source-built closed type inventory size.
    #[must_use]
    pub fn type_count(&self) -> usize {
        self.source.world().len()
    }
    /// Immutable functions from this exact source-bound execution seal.
    #[must_use]
    pub fn functions(&self) -> impl ExactSizeIterator<Item = VerifiedFunction<'_, 'a>> {
        (0..self.program.functions.len()).map(|index| VerifiedFunction { owner: self, index })
    }
    /// Retained operation index; no source or plan rescan is performed.
    #[must_use]
    pub fn operations(&self) -> impl ExactSizeIterator<Item = VerifiedOperation<'_, 'a>> {
        (0..self.typed.sites.len()).map(|index| VerifiedOperation { owner: self, index })
    }
}
impl<'p, 'a> VerifiedFunction<'p, 'a> {
    /// Exact canonical function inventory index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
    /// Exact canonical key; it does not identify a different universe/program.
    #[must_use]
    pub fn key(self) -> &'p [u8] {
        &self.owner.program.functions[self.index].key
    }
    /// Independently checked complete private result type.
    #[must_use]
    pub fn result_type(self) -> raw::ValueType {
        self.owner.program.functions[self.index].result_type
    }
    /// Checked CFG blocks, including every claimed block.
    #[must_use]
    pub fn blocks(self) -> impl ExactSizeIterator<Item = VerifiedBlock<'p, 'a>> {
        (0..self.owner.program.functions[self.index].blocks.len()).map(move |block| VerifiedBlock {
            owner: self.owner,
            function: self.index,
            block,
        })
    }
    /// Exact checked place arena.
    #[must_use]
    pub fn places(self) -> impl ExactSizeIterator<Item = VerifiedPlace<'p, 'a>> {
        self.owner.program.functions[self.index].places.iter().map(move |place| VerifiedPlace {
            owner: self.owner,
            function: self.index,
            id: place.id,
        })
    }
    /// Exact checked loan arena; a row does not assert runtime liveness at every site.
    #[must_use]
    pub fn loans(self) -> impl ExactSizeIterator<Item = VerifiedLoan<'p, 'a>> {
        self.owner.program.functions[self.index].loans.iter().map(move |loan| VerifiedLoan {
            owner: self.owner,
            function: self.index,
            id: loan.id,
        })
    }
}
impl<'p, 'a> VerifiedBlock<'p, 'a> {
    /// Dense block ID in the exact owning function.
    #[must_use]
    pub fn id(self) -> u32 {
        self.owner.program.functions[self.function].blocks[self.block].id
    }
    /// Retained same-seal operation views without reconstructing an index.
    #[must_use]
    pub fn operations(self) -> impl ExactSizeIterator<Item = VerifiedOperation<'p, 'a>> {
        self.owner.typed.functions[self.function].sites[self.block]
            .iter()
            .map(move |index| VerifiedOperation { owner: self.owner, index: *index })
    }
    /// Immutable checked terminator metadata in this block's owning domain.
    #[must_use]
    pub fn terminator(self) -> &'p raw::Terminator {
        &self.owner.program.functions[self.function].blocks[self.block].terminator
    }
}
impl<'p, 'a> VerifiedOperation<'p, 'a> {
    pub(super) fn instruction(self) -> &'p raw::Instruction {
        let s = self.owner.typed.sites[self.index];
        &self.owner.program.functions[s.function].blocks[s.block].instructions[s.instruction]
    }
    /// Complete exact function/block/instruction identity.
    ///
    /// # Panics
    /// Panics if an internal sealed instruction index exceeds its verified bound.
    #[must_use]
    pub fn site(self) -> OperationSite {
        let s = self.owner.typed.sites[self.index];
        OperationSite {
            function: s.function,
            block: self.owner.program.functions[s.function].blocks[s.block].id,
            instruction: u32::try_from(s.instruction).expect("sealed bounded instruction index"),
        }
    }
    /// Source/type/owner checked operation metadata, borrowed from this same seal only.
    #[must_use]
    pub fn kind(self) -> &'p raw::Operation {
        &self.instruction().operation
    }
    /// Result becomes available only on successful operation completion.
    #[must_use]
    pub fn result(self) -> Option<VerifiedValue<'p, 'a>> {
        self.instruction().result.as_ref().map(|d| VerifiedValue {
            owner: self.owner,
            function: self.site().function,
            id: d.id,
        })
    }
    /// Retained same-seal private callee, without reinterpreting a raw instance ID.
    #[must_use]
    pub fn call_target(self) -> Option<VerifiedFunction<'p, 'a>> {
        let site = self.site();
        self.owner.typed.functions[site.function].callees[site.block as usize]
            [site.instruction as usize]
            .map(|index| VerifiedFunction { owner: self.owner, index })
    }
    /// Exact outer failure cleanup excludes every unwritten result.
    #[must_use]
    pub fn failure(self) -> Option<VerifiedCleanupPlan<'p, 'a>> {
        self.owner.plan.operations[self.index].failure.map(|id| VerifiedCleanupPlan {
            owner: self.owner,
            function: self.site().function,
            id,
        })
    }
    /// Exact independent source-authorized Clone; no other opcode has a clone frontier.
    #[must_use]
    pub fn clone_frontier(self) -> Option<VerifiedClone<'p, 'a>> {
        matches!(self.kind(), raw::Operation::Clone { .. })
            .then_some(VerifiedClone { operation: self })
    }
}

impl<'p, 'a> VerifiedOperation<'p, 'a> {
    /// Exact checked Vec preparation, available only for that verified opcode.
    #[must_use]
    pub fn vec_construct(self) -> Option<VerifiedVecConstruct<'p, 'a>> {
        matches!(self.kind(), raw::Operation::VecConstruct { .. })
            .then_some(VerifiedVecConstruct { operation: self })
    }
    /// Exact checked Shared control/payload preparation.
    #[must_use]
    pub fn shared_construct(self) -> Option<VerifiedSharedConstruct<'p, 'a>> {
        matches!(self.kind(), raw::Operation::SharedConstruct { .. })
            .then_some(VerifiedSharedConstruct { operation: self })
    }
    /// Exact source-preserving checked weak-count acquisition.
    #[must_use]
    pub fn weak_downgrade(self) -> Option<VerifiedWeakDowngrade<'p, 'a>> {
        matches!(self.kind(), raw::Operation::WeakDowngrade { .. })
            .then_some(VerifiedWeakDowngrade { operation: self })
    }
    /// Exact original Copy Vec/array element observation.
    #[must_use]
    pub fn index_copy(self) -> Option<VerifiedIndexCopy<'p, 'a>> {
        matches!(self.kind(), raw::Operation::IndexCopy { .. })
            .then_some(VerifiedIndexCopy { operation: self })
    }
}
