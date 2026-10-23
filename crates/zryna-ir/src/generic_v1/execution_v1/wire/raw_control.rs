//! Explicit CFG and cleanup vocabulary, independent of runtime authority.
use super::{LoanDefinition, PlaceOrLoan, ValueDefinition};
use zryna_source::UntrustedSpan;
/// Static-place observation mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadMode {
    /// Proven original Copy observation.
    Copy,
    /// Ownership transfer.
    Move,
}
/// Exact enum observation role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchMode {
    /// Copy observation or ownership transfer.
    Value,
    /// Active shared payload loan.
    SharedLoan,
    /// Active exclusive payload loan.
    ExclusiveLoan,
}
/// One generated active binding; never stored loan data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchBinding {
    /// Ordinary value binding.
    Value(ValueDefinition),
    /// Nonstored loan binding.
    Loan(LoanDefinition),
}
/// One ordinary CFG edge carrying values only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    /// Exact target block.
    pub target: u32,
    /// Prepared values excluding any generated binding.
    pub arguments: Vec<u32>,
}
/// One canonical fixed ordinal successor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumArm {
    /// Fixed active variant ordinal.
    pub ordinal: u32,
    /// Generated binding equal to target's first parameter, if present.
    pub binding: Option<MatchBinding>,
    /// Ordinary arguments excluding the generated binding.
    pub edge: Edge,
}
/// Exact controlled runtime failure identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrapIdentity {
    /// Bounds failure.
    Bounds,
    /// Allocation failure.
    Allocation,
    /// Capacity overflow.
    Capacity,
    /// Reference count overflow.
    Refcount,
    /// Invalid runtime UTF-8.
    Utf8,
}
/// Exactly one explicit block transfer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Terminator {
    /// Exact result and reverse exit cleanup.
    Return {
        /// Absent for unstored unit.
        value: Option<u32>,
        /// Exact retained cleanup plan.
        cleanup: u32,
    },
    /// Ordinary unconditional edge.
    Jump(Edge),
    /// Only the selected successor executes.
    Branch {
        /// Exact bool value.
        condition: u32,
        /// True successor.
        yes: Edge,
        /// False successor.
        no: Edge,
    },
    /// Exact exhaustive fixed active-tag selection.
    ClosedEnumMatch {
        /// Exact enum type.
        ty: u32,
        /// Evaluated source.
        scrutinee: PlaceOrLoan,
        /// Observation role.
        mode: MatchMode,
        /// Canonical exhaustive ordinal arms.
        arms: Vec<EnumArm>,
    },
    /// Atomic checked weak upgrade with distinct expiry edge.
    WeakUpgradeBranch {
        /// Exact Weak source.
        weak: PlaceOrLoan,
        /// Exact Shared binding on success only.
        success_binding: ValueDefinition,
        /// Successful acquisition edge.
        success: Edge,
        /// Expired edge, without generated binding.
        expired: Edge,
        /// Refcount failure cleanup.
        failure: u32,
    },
    /// Controlled failure after exact cleanup.
    Trap {
        /// Exact trap kind.
        identity: TrapIdentity,
        /// Exact retained cleanup plan.
        cleanup: u32,
    },
}
/// Symbolic preparation identity; never a caller-supplied progress count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparationSite {
    /// Canonical function inventory index.
    pub function: u32,
    /// Dense block identity.
    pub block: u32,
    /// Source-ordered instruction identity.
    pub instruction: u32,
}
/// Exact independent cleanup claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupAction {
    /// End one live loan before releasing its root.
    EndLoan(u32),
    /// Drop current initialized place state.
    DropPlace(u32),
    /// Roll back only the operation-bound initialized prefix.
    DropInitializedPrefix {
        /// Exact operation destination.
        destination: u32,
        /// Exact retained preparation.
        preparation_site: PreparationSite,
    },
}
/// One claimed reverse cleanup plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupPlan {
    /// Dense plan identity.
    pub id: u32,
    /// Exact original failure or exit site.
    pub span: UntrustedSpan,
    /// Exact ordered cleanup sequence.
    pub actions: Vec<CleanupAction>,
}
