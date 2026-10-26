//! Actual runtime guard predicates, scoped to their retained canonical operation role.
use super::{Diagnostic, Model, invariant};
use wasmparser::Operator;
mod allocation;
mod atoms;
mod counts;
mod release;
use atoms::A;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Guard {
    AllocateArguments,
    AllocateZero,
    AllocationFault,
    AllocationCapacity,
    AllocationAddress,
    ReleaseNull,
    ReleaseSize,
    PointerValidity,
    HeaderLive,
    HeaderSize,
    HeaderAlignment,
    ControlFlags(bool),
    CountZero(u32),
    CountMax(u32),
    FinishCounts,
    WeakReleaseCounts,
}

pub(super) struct Window {
    role: u32,
    recent: [A; 32],
    next: usize,
    seen: usize,
}
impl Window {
    pub(super) fn new(role: u32) -> Self {
        Self { role, recent: [A(255, 0); 32], next: 0, seen: 0 }
    }
    pub(super) fn push(
        &mut self,
        op: &Operator<'_>,
        m: &Model<'_, '_, '_>,
    ) -> Result<(), Diagnostic> {
        m.take(1)?;
        self.recent[self.next] = atoms::decode(op);
        self.next = (self.next + 1) % self.recent.len();
        self.seen = (self.seen + 1).min(self.recent.len());
        Ok(())
    }
    pub(super) fn guard(&self, m: &Model<'_, '_, '_>) -> Result<Guard, Diagnostic> {
        use crate::generic_execution_v1::model::containers as C;
        let guard = match self.role {
            C::ALLOCATE_RAW => allocation::classify(self, m)?,
            C::RELEASE_RAW => release::classify(self, m)?,
            C::STRONG_CLONE
            | C::DOWNGRADE
            | C::WEAK_CLONE
            | C::STRONG_BEGIN
            | C::STRONG_FINISH
            | C::WEAK_RELEASE => counts::classify(self, m)?,
            _ => None,
        };
        guard.ok_or_else(invariant)
    }
    fn matches(&self, pattern: &[A], m: &Model<'_, '_, '_>) -> Result<bool, Diagnostic> {
        if pattern.is_empty() || pattern.len() > self.recent.len() {
            return Err(invariant());
        }
        m.take(pattern.len())?;
        if self.seen < pattern.len() {
            return Ok(false);
        }
        let start = self.next + self.recent.len() - pattern.len();
        Ok(pattern
            .iter()
            .enumerate()
            .all(|(index, atom)| self.recent[(start + index) % self.recent.len()] == *atom))
    }
}
