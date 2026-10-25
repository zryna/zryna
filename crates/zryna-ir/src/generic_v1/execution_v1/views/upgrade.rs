//! Atomic upgrade branches retain one owner and expose generated ownership only on success.
use super::{VerifiedBlock, VerifiedCleanupPlan, VerifiedLoan, VerifiedPlace, VerifiedValue, raw};
/// One independently checked indivisible Weak acquisition terminator.
#[derive(Clone, Copy)]
pub struct VerifiedWeakUpgrade<'p, 'a> {
    block: VerifiedBlock<'p, 'a>,
}
/// One exact ordinary successor edge from the same checked upgrade block.
#[derive(Clone, Copy)]
pub struct VerifiedUpgradeEdge<'p, 'a> {
    upgrade: VerifiedWeakUpgrade<'p, 'a>,
    success: bool,
}
impl<'p, 'a> VerifiedBlock<'p, 'a> {
    /// Exact source-preserving upgrade; count failure selects neither successor.
    #[must_use]
    pub fn weak_upgrade(self) -> Option<VerifiedWeakUpgrade<'p, 'a>> {
        matches!(self.terminator(), raw::Terminator::WeakUpgradeBranch { .. })
            .then_some(VerifiedWeakUpgrade { block: self })
    }
}
impl<'p, 'a> VerifiedWeakUpgrade<'p, 'a> {
    /// Original readable Weak place, absent for a loan source.
    #[must_use]
    pub fn source_place(self) -> Option<VerifiedPlace<'p, 'a>> {
        let raw::PlaceOrLoan::Place(id) = self.source() else { return None };
        Some(VerifiedPlace { owner: self.block.owner, function: self.block.function, id })
    }
    /// Original readable Weak loan, absent for a place source.
    #[must_use]
    pub fn source_loan(self) -> Option<VerifiedLoan<'p, 'a>> {
        let raw::PlaceOrLoan::Loan(id) = self.source() else { return None };
        Some(VerifiedLoan { owner: self.block.owner, function: self.block.function, id })
    }
    /// Complete exact original Weak identity.
    #[must_use]
    pub fn weak_type(self) -> u32 {
        match self.source() {
            raw::PlaceOrLoan::Place(id) => {
                self.block.owner.typed.functions[self.block.function].places.rows[id as usize].ty
            }
            raw::PlaceOrLoan::Loan(id) => {
                self.block.owner.program.functions[self.block.function].loans[id as usize].referent
            }
        }
    }
    /// Exact original referent shared by Weak and generated Shared.
    #[must_use]
    pub fn referent_type(self) -> u32 {
        self.child()
    }
    fn child(self) -> u32 {
        self.block.owner.source.world().arguments(self.weak_type()).expect("sealed Weak child")[0]
    }
    /// New Shared owner is the success target's first parameter, with no expiry binding.
    #[must_use]
    pub fn success_binding(self) -> VerifiedValue<'p, 'a> {
        let raw::Terminator::WeakUpgradeBranch { success_binding, .. } = self.block.terminator()
        else {
            unreachable!("sealed upgrade")
        };
        VerifiedValue {
            owner: self.block.owner,
            function: self.block.function,
            id: success_binding.id,
        }
    }
    /// Exact checked count-failure cleanup, excluding the unpublished Shared result.
    #[must_use]
    pub fn outer_failure(self) -> VerifiedCleanupPlan<'p, 'a> {
        let raw::Terminator::WeakUpgradeBranch { failure, .. } = self.block.terminator() else {
            unreachable!("sealed upgrade")
        };
        VerifiedCleanupPlan { owner: self.block.owner, function: self.block.function, id: *failure }
    }
    /// Successful atomic acquisition edge.
    #[must_use]
    pub const fn success(self) -> VerifiedUpgradeEdge<'p, 'a> {
        VerifiedUpgradeEdge { upgrade: self, success: true }
    }
    /// Expired source edge; no Shared owner is published.
    #[must_use]
    pub const fn expired(self) -> VerifiedUpgradeEdge<'p, 'a> {
        VerifiedUpgradeEdge { upgrade: self, success: false }
    }
    fn source(self) -> raw::PlaceOrLoan {
        let raw::Terminator::WeakUpgradeBranch { weak, .. } = self.block.terminator() else {
            unreachable!("sealed upgrade")
        };
        *weak
    }
}
impl<'p, 'a> VerifiedUpgradeEdge<'p, 'a> {
    /// Exact successor in the same owning execution/function.
    #[must_use]
    pub fn target(self) -> VerifiedBlock<'p, 'a> {
        VerifiedBlock { block: self.raw().target as usize, ..self.upgrade.block }
    }
    /// Prepared ordinary arguments excluding any generated success binding.
    #[must_use]
    pub fn arguments(self) -> impl ExactSizeIterator<Item = VerifiedValue<'p, 'a>> {
        self.raw().arguments.iter().map(move |id| VerifiedValue {
            owner: self.upgrade.block.owner,
            function: self.upgrade.block.function,
            id: *id,
        })
    }
    fn raw(self) -> &'p raw::Edge {
        let raw::Terminator::WeakUpgradeBranch { success, expired, .. } =
            self.upgrade.block.terminator()
        else {
            unreachable!("sealed upgrade")
        };
        if self.success { success } else { expired }
    }
}
