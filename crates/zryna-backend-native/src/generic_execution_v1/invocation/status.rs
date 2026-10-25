//! Source-bound failure roles; canonical and source status namespaces remain distinct.
use crate::generic_execution_v1::invariant;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, invocation::SourceStatus, raw};
#[derive(Clone, Copy, Default)]
pub(super) struct Cases(u8);
impl Cases {
    pub fn allocation() -> Self {
        Self(3)
    }
    pub fn count() -> Self {
        Self(6)
    }
    pub fn utf8() -> Self {
        Self(11)
    }
    pub fn capacity() -> Self {
        Self(2)
    }
    pub fn bounds() -> Self {
        Self(16)
    }
    pub fn union(&mut self, other: Self) {
        self.0 |= other.0;
    }
    pub fn len(self) -> u64 {
        u64::from(self.0.count_ones())
    }
    pub fn entries(self) -> impl Iterator<Item = (u32, SourceStatus)> {
        [
            (1, raw::TrapIdentity::Allocation, 1),
            (2, raw::TrapIdentity::Capacity, 2),
            (3, raw::TrapIdentity::Refcount, 4),
            (4, raw::TrapIdentity::Utf8, 8),
            (1, raw::TrapIdentity::Bounds, 16),
        ]
        .into_iter()
        .filter(move |(_, _, bit)| self.0 & bit != 0)
        .map(|(canonical, trap, _)| (canonical, SourceStatus::Trap(trap)))
    }
}
pub(super) fn operation(op: VerifiedOperation<'_, '_>, clone: Cases) -> Result<Cases, Diagnostic> {
    Ok(match op.kind() {
        raw::Operation::StringFromUtf8 { .. } => Cases::utf8(),
        raw::Operation::StringConcat { .. }
        | raw::Operation::VecConstruct { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::VecPush { .. } => Cases::allocation(),
        raw::Operation::WeakDowngrade { .. } => Cases::count(),
        raw::Operation::Clone { .. } => clone,
        raw::Operation::IndexCopy { .. } | raw::Operation::BeginLoan { .. } => Cases::bounds(),
        raw::Operation::ClosedGenericCall { .. } | raw::Operation::SourceCall { .. } => {
            Cases::default()
        }
        _ if op.failure().is_none() => Cases::default(),
        _ => return Err(invariant()),
    })
}
