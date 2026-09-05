//! Bounded, invocation-bound symbolic control traces; never runtime execution receipts.

use super::{
    LogicalOperation, MAX_ALLOCATION_OPERATIONS, MAX_DYNAMIC_ALLOCATION_BYTES,
    MAX_LIVE_ALLOCATIONS, MAX_STATUS_TRANSITIONS, OwnershipRuntimeAbiIdentity, RuntimeAbiViolation,
    RuntimeAbiViolationKind, RuntimeStatus, TransitionClaim, VerifiedOwnershipRuntimeAbi,
    validate_transition,
};
use std::collections::BTreeMap;
use zryna_layout::{StorageTarget, TypeId, VerifiedLayouts};

mod payload;
mod replay;
#[cfg(test)]
mod tests;
pub use payload::{PayloadKind, PayloadNode};

/// Complete logical state of one Shared/Weak control allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlState {
    /// Strong-owner count.
    pub strong_count: u32,
    /// Explicit weak handles plus the implicit weak owner while strong owners exist or release is pending.
    pub weak_count: u32,
    /// Whether last-strong release has begun but not finished.
    pub pending_last_strong: bool,
    /// Whether the payload remains initialized.
    pub payload_initialized: bool,
    /// Whether the control allocation remains allocated.
    pub allocated: bool,
}

/// One untrusted site-bound operation in a complete symbolic invocation trace.
#[derive(Clone, Debug)]
pub struct ControlEvent {
    /// Must equal the event's dense ordinal; prevents site substitution/replay.
    pub site: u64,
    /// Closed operation claim.
    pub kind: ControlEventKind,
}

/// Untrusted construction or handle-operation claims. IDs are dense and invocation-local.
#[derive(Clone, Debug)]
pub enum ControlEventKind {
    /// Prepare a fully initialized typed payload, then publish only on successful allocation.
    Construct {
        /// Exact layout-branded payload type.
        payload: TypeId,
        /// Exact initialized topology, children preceding parents; last node is the root.
        nodes: Vec<PayloadNode>,
        /// Claimed allocation status.
        status: RuntimeStatus,
        /// Nonzero aligned control base on success, zero otherwise.
        base: u64,
        /// Exact sealed control size.
        size: u64,
        /// Exact sealed control alignment.
        alignment: u64,
        /// Next dense control ID on success, none otherwise.
        control: Option<u32>,
        /// Next dense strong-owner ID on success, none otherwise.
        owner: Option<u32>,
    },
    /// Retain or consume one previously issued external/payload handle.
    Handle {
        /// Exact frozen ABI count operation, never a replacement count model.
        operation: LogicalOperation,
        /// Issued owner, not an implicit Weak count.
        owner: u32,
        /// Exact operation status.
        status: RuntimeStatus,
        /// Next dense owner on successful clone/downgrade/upgrade only.
        result: Option<u32>,
        /// Exact operation-specific boolean shape.
        boolean: Option<bool>,
        /// Claimed resulting state checked against the existing ABI transition validator.
        after: ControlState,
    },
    /// Complete the next non-handle payload leaf/storage drop, derived from sealed topology.
    DropPayloadNode {
        /// Pending control whose exact payload is being destroyed.
        control: u32,
        /// Exact next node in reverse recursive cleanup, Vec storage last.
        node: u32,
    },
    /// Finish only the current last-release receipt after its complete payload cleanup.
    Finish {
        /// Pending control, never an arbitrary count-state claim.
        control: u32,
        /// Existing ABI finish transition's exact resulting state.
        after: ControlState,
    },
}

/// Exact model invocation input. Reverification proves the same trace, not another execution.
#[derive(Clone, Debug)]
pub struct ControlTrace {
    /// Must match the independently supplied expected invocation identity.
    pub invocation: u64,
    /// Must match the independently verified ABI declaration authority.
    pub abi: OwnershipRuntimeAbiIdentity,
    /// Exact storage target of the independently supplied layouts.
    pub target: StorageTarget,
    /// Bounded source-ordered transition sites.
    pub events: Vec<ControlEvent>,
}

/// Opaque complete symbolic proof. It cannot authorize concrete allocator or runtime effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedControlTrace {
    invocation: u64,
    states: Vec<ControlState>,
    live_owners: usize,
}

/// Independently expected surviving external owner at the invocation boundary.
/// Empty expectations require complete release; payload-owned children are retained by their root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpectedOwner {
    /// Exact issued owner identity.
    pub owner: u32,
    /// Exact issued control identity.
    pub control: u32,
    /// True only for explicit Weak ownership, never the implicit Weak count.
    pub weak: bool,
}
impl VerifiedControlTrace {
    /// Returns the independently bound model invocation.
    #[must_use]
    pub const fn invocation(&self) -> u64 {
        self.invocation
    }
    /// Returns final count states of issued controls in canonical order.
    #[must_use]
    pub fn states(&self) -> impl ExactSizeIterator<Item = ControlState> + '_ {
        self.states.iter().copied()
    }
    /// Returns unconsumed explicit owners; never includes the implicit Weak count.
    #[must_use]
    pub const fn live_owners(&self) -> usize {
        self.live_owners
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Location {
    External,
    Payload(u32),
    Dropped,
}
#[derive(Clone, Copy)]
struct Owner {
    control: u32,
    weak: bool,
    location: Location,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Drop {
    Handle(u32),
    Node(u32),
}
struct Control {
    payload: TypeId,
    state: ControlState,
    base: u64,
    drops: Vec<Drop>,
    next: usize,
}
struct Model<'a> {
    abi: &'a VerifiedOwnershipRuntimeAbi,
    layouts: &'a VerifiedLayouts,
    controls: Vec<Control>,
    owners: Vec<Owner>,
    intervals: BTreeMap<u64, u64>,
    pending: Vec<u32>,
    allocations: u64,
    nodes: u64,
}

fn violation(message: &str) -> RuntimeAbiViolation {
    RuntimeAbiViolation {
        kind: RuntimeAbiViolationKind::Evidence,
        declaration_index: None,
        message: message.into(),
    }
}
fn budget() -> RuntimeAbiViolation {
    RuntimeAbiViolation {
        kind: RuntimeAbiViolationKind::Budget,
        declaration_index: None,
        message: "control trace exceeds the checked invocation budget".into(),
    }
}

fn checked_model_count(before: u64, extra: u64, maximum: u64) -> Result<u64, RuntimeAbiViolation> {
    before.checked_add(extra).filter(|&total| total <= maximum).ok_or_else(budget)
}

/// Verifies SW1–SW5 model provenance and exact transitions without executing an allocator.
///
/// # Errors
/// Rejects foreign ABI/layout/invocation, malformed topology, stale owners, allocation overlap,
/// invalid count/status claims, reordered/replayed drops and all incomplete last-release receipts.
pub fn verify_control_trace(
    abi: &VerifiedOwnershipRuntimeAbi,
    layouts: &VerifiedLayouts,
    expected_invocation: u64,
    expected_owners: &[ExpectedOwner],
    trace: &ControlTrace,
) -> Result<VerifiedControlTrace, RuntimeAbiViolation> {
    if trace.invocation != expected_invocation
        || trace.abi != abi.identity()
        || trace.target != layouts.target()
        || layouts.universe_identity() != abi.universe
        || *layouts.fingerprint()
            != match trace.target {
                StorageTarget::Linear32V1 => abi.linear32_fingerprint,
                StorageTarget::LinuxX8664V1 => abi.linux_x86_64_fingerprint,
            }
    {
        return Err(violation(
            "control trace has foreign ABI, layout, target or invocation authority",
        ));
    }
    checked_model_count(0, trace.events.len() as u64, MAX_STATUS_TRANSITIONS)?;
    let mut model = Model {
        abi,
        layouts,
        controls: vec![],
        owners: vec![],
        intervals: BTreeMap::new(),
        pending: vec![],
        allocations: 0,
        nodes: 0,
    };
    for (index, event) in trace.events.iter().enumerate() {
        if event.site != index as u64 {
            return Err(violation("control transition site is not dense or was replayed"));
        }
        model.replay(&event.kind)?;
    }
    if !model.pending.is_empty() {
        return Err(violation("last-strong payload cleanup is incomplete"));
    }
    let surviving: Vec<_> = model
        .owners
        .iter()
        .enumerate()
        .filter(|(_, owner)| owner.location == Location::External)
        .map(|(id, owner)| {
            Ok(ExpectedOwner {
                owner: u32::try_from(id).map_err(|_| budget())?,
                control: owner.control,
                weak: owner.weak,
            })
        })
        .collect::<Result<Vec<_>, RuntimeAbiViolation>>()?;
    if surviving != expected_owners {
        return Err(violation(
            "surviving external owners differ from the independent exit contract",
        ));
    }
    Ok(VerifiedControlTrace {
        invocation: expected_invocation,
        states: model.controls.iter().map(|control| control.state).collect(),
        live_owners: model
            .owners
            .iter()
            .filter(|owner| owner.location != Location::Dropped)
            .count(),
    })
}
