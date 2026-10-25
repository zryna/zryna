//! Exact old-value drop frontier; only owning replay can retain this operation receipt.
use super::{VerifiedLoan, VerifiedOperation, VerifiedPlace, raw};
/// Checked reverse old-value drop frontier bound to one exact source operation.
#[derive(Clone, Copy)]
pub struct VerifiedDropTargets<'p, 'a> {
    operation: VerifiedOperation<'p, 'a>,
    targets: &'p [raw::PlaceOrLoan],
    whole: bool,
}
/// One exact complete referent or static subobject from the checked operation's old value.
#[derive(Clone, Copy)]
pub enum VerifiedDropTarget<'p, 'a> {
    /// Complete initialized static subobject; inactive/uninitialized siblings are absent.
    Place(VerifiedPlace<'p, 'a>),
    /// Complete exact loan referent; no local place is fabricated for formal loans.
    Loan(VerifiedLoan<'p, 'a>),
}
impl<'p, 'a> VerifiedOperation<'p, 'a> {
    /// Source/type/state verified old-drop receipt for a dropping or replacing operation.
    #[must_use]
    pub fn drop_targets(self) -> Option<VerifiedDropTargets<'p, 'a>> {
        let range = self.owner.plan.operations.get(self.index)?.drops?;
        let targets = self.owner.plan.drop_targets.get(range.start..range.end)?;
        Some(VerifiedDropTargets { operation: self, targets, whole: range.whole })
    }
}
impl<'p, 'a> VerifiedDropTargets<'p, 'a> {
    /// Whether the original selected object was complete, preserving whole-value behavior.
    #[must_use]
    pub fn is_whole(self) -> bool {
        self.whole
    }
    /// Reverse declaration/index ordered maximal initialized objects, bound to this same site.
    #[must_use]
    pub fn targets(self) -> impl ExactSizeIterator<Item = VerifiedDropTarget<'p, 'a>> {
        let op = self.operation;
        self.targets.iter().map(move |source| match *source {
            raw::PlaceOrLoan::Place(id) => VerifiedDropTarget::Place(VerifiedPlace {
                owner: op.owner,
                function: op.site().function,
                id,
            }),
            raw::PlaceOrLoan::Loan(id) => VerifiedDropTarget::Loan(VerifiedLoan {
                owner: op.owner,
                function: op.site().function,
                id,
            }),
        })
    }
}
