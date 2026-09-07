use serde::{Deserialize, Serialize};

/// Stable trap categories compared by differential conformance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScalarTrapCode {
    /// Explicit language-level unreachable execution.
    Unreachable,
    /// Target engine reported an execution trap.
    TargetTrap,
    /// `DataOwnershipV1` bounds language trap.
    #[serde(rename = "zryna.trap.bounds-v1")]
    Bounds,
    /// `DataOwnershipV1` allocation language trap.
    #[serde(rename = "zryna.trap.allocation-v1")]
    Allocation,
    /// `DataOwnershipV1` capacity language trap.
    #[serde(rename = "zryna.trap.capacity-v1")]
    Capacity,
    /// `DataOwnershipV1` refcount language trap.
    #[serde(rename = "zryna.trap.refcount-v1")]
    Refcount,
    /// `DataOwnershipV1` utf8 language trap.
    #[serde(rename = "zryna.trap.utf8-v1")]
    Utf8,
}

impl ScalarTrapCode {
    /// Returns the canonical observable spelling, independent of the execution target.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unreachable => "unreachable",
            Self::TargetTrap => "target-trap",
            Self::Bounds => "zryna.trap.bounds-v1",
            Self::Allocation => "zryna.trap.allocation-v1",
            Self::Capacity => "zryna.trap.capacity-v1",
            Self::Refcount => "zryna.trap.refcount-v1",
            Self::Utf8 => "zryna.trap.utf8-v1",
        }
    }
}
