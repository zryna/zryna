use super::OwnershipTarget;
use serde::{Deserialize, Serialize};
use zryna_abi::ScalarOutcome;

/// One normalized typed target observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipManifestResult {
    pub(super) target: OwnershipTarget,
    pub(super) outcome: ScalarOutcome,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) trace: Vec<OwnershipTraceEvent>,
}

impl OwnershipManifestResult {
    /// Creates one target result for canonical manifest rendering.
    #[must_use]
    pub(crate) const fn new(target: OwnershipTarget, outcome: ScalarOutcome) -> Self {
        Self { target, outcome, trace: Vec::new() }
    }
    pub(crate) fn with_trace(mut self, trace: Vec<OwnershipTraceEvent>) -> Self {
        self.trace = trace;
        self
    }
    /// Returns the executed logical cleanup trace in execution order.
    #[must_use]
    pub fn trace(&self) -> &[OwnershipTraceEvent] {
        &self.trace
    }

    /// Returns the observed target.
    #[must_use]
    pub const fn target(&self) -> OwnershipTarget {
        self.target
    }
    /// Returns the normalized typed outcome.
    #[must_use]
    pub const fn outcome(&self) -> ScalarOutcome {
        self.outcome
    }
}

/// Logical owned value category, independent of addresses and physical allocator bookkeeping.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OwnershipValueKind {
    /// String storage release.
    String,
    /// Struct, fixed array or Vec child sequence.
    Sequence,
    /// Active enum payload.
    Enum,
    /// Strong handle release.
    Shared,
    /// Weak handle release.
    Weak,
}

/// One executed cleanup or release step in the internal candidate observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum OwnershipTraceEvent {
    /// A sealed function cleanup plan drops this exact place.
    Cleanup {
        /// Source module identity.
        module: u32,
        /// Declaration identity within the module.
        function: u32,
        /// Verified place identity.
        place: u32,
    },
    /// Logical recursive value drop.
    Drop {
        /// The value's owned category.
        value: OwnershipValueKind,
    },
    /// Last strong owner releases the control block's implicit weak reference.
    ReleaseImplicitWeak,
    /// Last weak reference releases the control block.
    ReleaseControl,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ownership_manifest::{ManifestCommand, validation::validate_results};
    use zryna_abi::ScalarTrapCode;

    #[test]
    fn manifest_typed_trap_trace_boundaries_and_hostile_events_are_strict() {
        let target = OwnershipTarget::JavaScript;
        let check = |result| validate_results(ManifestCommand::Run, &[target], &[result]);
        for code in [
            ScalarTrapCode::Bounds,
            ScalarTrapCode::Allocation,
            ScalarTrapCode::Capacity,
            ScalarTrapCode::Refcount,
            ScalarTrapCode::Utf8,
        ] {
            let result = OwnershipManifestResult::new(target, ScalarOutcome::Trapped { code });
            let bytes = serde_json::to_vec(&result).unwrap();
            assert_eq!(serde_json::from_slice::<OwnershipManifestResult>(&bytes).unwrap(), result);
            check(result).unwrap();
        }
        let event = OwnershipTraceEvent::Drop { value: OwnershipValueKind::String };
        let result = OwnershipManifestResult::new(
            target,
            ScalarOutcome::Trapped { code: ScalarTrapCode::Allocation },
        );
        check(result.clone().with_trace(vec![event.clone(); 4096])).unwrap();
        assert!(check(result.clone().with_trace(vec![event; 4097])).is_err());
        for event in [
            OwnershipTraceEvent::Cleanup { module: 65536, function: 0, place: 0 },
            OwnershipTraceEvent::Cleanup { module: 0, function: 65536, place: 0 },
            OwnershipTraceEvent::Cleanup { module: 0, function: 0, place: 1048577 },
        ] {
            assert!(check(result.clone().with_trace(vec![event])).is_err());
        }
        for code in [ScalarTrapCode::TargetTrap, ScalarTrapCode::Unreachable] {
            assert!(
                check(OwnershipManifestResult::new(target, ScalarOutcome::Trapped { code }))
                    .is_err()
            );
        }
        for event in [
            serde_json::json!({"kind":"host-error"}),
            serde_json::json!({"kind":"drop","value":"string","unknown":true}),
            serde_json::json!({"kind":"drop","value":"pointer"}),
        ] {
            assert!(serde_json::from_value::<OwnershipTraceEvent>(event).is_err());
        }
    }
}
