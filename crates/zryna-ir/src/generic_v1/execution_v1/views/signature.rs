//! Signature, incoming CFG values and exit cleanup from their exact owning views.
use super::{
    VerifiedBlock, VerifiedCleanupPlan, VerifiedFunction, VerifiedLoan, VerifiedValue, raw,
};

/// One source-ordered private parameter from one independently verified function.
#[derive(Clone, Copy)]
pub enum VerifiedParameter<'p, 'a> {
    /// Ordinary value; affine ownership follows the verified private call convention.
    Value(VerifiedValue<'p, 'a>),
    /// Nonstored formal referent, never a fabricated local place.
    Loan(VerifiedLoan<'p, 'a>),
}

impl<'p, 'a> VerifiedFunction<'p, 'a> {
    /// Exact private parameters in source order, retaining this function's seal.
    #[must_use]
    pub fn parameters(self) -> impl ExactSizeIterator<Item = VerifiedParameter<'p, 'a>> {
        self.owner.program.functions[self.index].parameters.iter().map(move |parameter| {
            match parameter {
                raw::Parameter::Value(value) => VerifiedParameter::Value(VerifiedValue {
                    owner: self.owner,
                    function: self.index,
                    id: value.id,
                }),
                raw::Parameter::Loan(loan) => VerifiedParameter::Loan(VerifiedLoan {
                    owner: self.owner,
                    function: self.index,
                    id: loan.id,
                }),
            }
        })
    }
}

impl<'p, 'a> VerifiedBlock<'p, 'a> {
    /// Exact incoming ordinary values in the independently checked CFG order.
    #[must_use]
    pub fn value_parameters(self) -> impl ExactSizeIterator<Item = VerifiedValue<'p, 'a>> {
        self.owner.program.functions[self.function].blocks[self.block].value_parameters.iter().map(
            move |value| VerifiedValue { owner: self.owner, function: self.function, id: value.id },
        )
    }
    /// Exact retained cleanup for this block's own Return or Trap exit only.
    /// No caller-supplied plan identity or foreign function can select this view.
    #[must_use]
    pub fn exit_cleanup(self) -> Option<VerifiedCleanupPlan<'p, 'a>> {
        match self.terminator() {
            raw::Terminator::Return { cleanup, .. } | raw::Terminator::Trap { cleanup, .. } => {
                Some(VerifiedCleanupPlan {
                    owner: self.owner,
                    function: self.function,
                    id: *cleanup,
                })
            }
            _ => None,
        }
    }
}
