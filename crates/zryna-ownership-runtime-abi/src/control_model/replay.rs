use super::{
    Control, ControlEventKind, ControlState, Drop, Location, LogicalOperation,
    MAX_ALLOCATION_OPERATIONS, MAX_DYNAMIC_ALLOCATION_BYTES, MAX_LIVE_ALLOCATIONS, Model, Owner,
    RuntimeAbiViolation, RuntimeStatus, StorageTarget, TransitionClaim, budget,
    validate_transition, violation,
};

impl Model<'_> {
    pub(super) fn replay(&mut self, event: &ControlEventKind) -> Result<(), RuntimeAbiViolation> {
        match event {
            ControlEventKind::Construct {
                payload,
                nodes,
                status,
                base,
                size,
                alignment,
                control,
                owner,
            } => {
                self.require_publication_phase()?;
                let layout = self
                    .abi
                    .control_layouts()
                    .find(|layout| {
                        layout.payload() == *payload && layout.target() == self.layouts.target()
                    })
                    .ok_or_else(|| violation("construction has no exact sealed control layout"))?;
                if *size != layout.size() || *alignment != layout.alignment() {
                    return Err(violation(
                        "construction size or alignment differs from sealed control layout",
                    ));
                }
                let drops = self.payload(*payload, nodes)?;
                self.allocations = self.allocations.checked_add(1).ok_or_else(budget)?;
                let capacity = *size > MAX_DYNAMIC_ALLOCATION_BYTES
                    || self.allocations > MAX_ALLOCATION_OPERATIONS
                    || self.intervals.len() as u64 >= MAX_LIVE_ALLOCATIONS;
                if *status != RuntimeStatus::Ok {
                    if *base != 0
                        || control.is_some()
                        || owner.is_some()
                        || (*status != RuntimeStatus::Capacity || !capacity)
                            && (*status != RuntimeStatus::Allocation || capacity)
                    {
                        return Err(violation(
                            "construction failure has wrong status or nonzero publication",
                        ));
                    }
                    return Ok(());
                }
                if capacity {
                    return Err(budget());
                }
                let end = base
                    .checked_add(*size)
                    .ok_or_else(|| violation("control allocation interval overflow"))?;
                if *base == 0
                    || *size == 0
                    || *alignment == 0
                    || base % alignment != 0
                    || self.layouts.target() == StorageTarget::Linear32V1 && end > 1_u64 << 32
                    || self
                        .intervals
                        .range(..=*base)
                        .next_back()
                        .is_some_and(|(_, &end)| end > *base)
                    || self.intervals.range(*base..).next().is_some_and(|(&start, _)| start < end)
                {
                    return Err(violation(
                        "control allocation is null, misaligned, overlapping or nonrepresentable",
                    ));
                }
                let id = u32::try_from(self.controls.len()).map_err(|_| budget())?;
                if *control != Some(id) || *owner != Some(self.next_owner()?) {
                    return Err(violation(
                        "construction control or owner identity is not fresh and dense",
                    ));
                }
                for drop in &drops {
                    if let Drop::Handle(owner) = drop {
                        self.owners[*owner as usize].location = Location::Payload(id);
                    }
                }
                self.intervals.insert(*base, end);
                self.controls.push(Control {
                    payload: *payload,
                    base: *base,
                    drops,
                    next: 0,
                    state: ControlState {
                        strong_count: 1,
                        weak_count: 1,
                        pending_last_strong: false,
                        payload_initialized: true,
                        allocated: true,
                    },
                });
                self.owners.push(Owner { control: id, weak: false, location: Location::External });
                Ok(())
            }
            ControlEventKind::Handle { operation, owner, status, result, boolean, after } => {
                self.handle(*operation, *owner, *status, *result, *boolean, *after)
            }
            ControlEventKind::DropPayloadNode { control, node } => {
                self.next_drop(*control, Drop::Node(*node))
            }
            ControlEventKind::Finish { control, after } => self.finish(*control, *after),
        }
    }

    fn require_publication_phase(&self) -> Result<(), RuntimeAbiViolation> {
        if !self.pending.is_empty() {
            return Err(violation("payload destruction cannot allocate or publish"));
        }
        Ok(())
    }

    fn next_owner(&self) -> Result<u32, RuntimeAbiViolation> {
        u32::try_from(self.owners.len()).map_err(|_| budget())
    }

    fn next_drop(&mut self, id: u32, expected: Drop) -> Result<(), RuntimeAbiViolation> {
        if self.pending.last() != Some(&id) {
            return Err(violation("cleanup targets a foreign or nonpending control"));
        }
        let control = &mut self.controls[id as usize];
        if control.drops.get(control.next) != Some(&expected) {
            return Err(violation(
                "payload cleanup is omitted, duplicated or out of reverse order",
            ));
        }
        control.next += 1;
        Ok(())
    }

    fn handle(
        &mut self,
        operation: LogicalOperation,
        id: u32,
        status: RuntimeStatus,
        result: Option<u32>,
        boolean: Option<bool>,
        after: ControlState,
    ) -> Result<(), RuntimeAbiViolation> {
        let owner =
            *self.owners.get(id as usize).ok_or_else(|| violation("handle was never issued"))?;
        let release = matches!(
            operation,
            LogicalOperation::StrongReleaseBegin | LogicalOperation::WeakRelease
        );
        if owner.location == Location::Dropped {
            return Err(violation("handle owner is moved or already released"));
        }
        if let Some(&pending) = self.pending.last() {
            if !release || owner.location != Location::Payload(pending) {
                return Err(violation(
                    "payload cleanup cannot invoke unrelated or retained handle operations",
                ));
            }
        } else if owner.location != Location::External {
            return Err(violation("payload-owned handle is not an independently available owner"));
        }
        let weak_operation = matches!(
            operation,
            LogicalOperation::WeakClone
                | LogicalOperation::WeakUpgrade
                | LogicalOperation::WeakRelease
        );
        if weak_operation != owner.weak
            || !matches!(
                operation,
                LogicalOperation::StrongClone
                    | LogicalOperation::WeakDowngrade
                    | LogicalOperation::WeakClone
                    | LogicalOperation::WeakUpgrade
                    | LogicalOperation::StrongReleaseBegin
                    | LogicalOperation::WeakRelease
            )
        {
            return Err(violation("handle mode does not match the exact operation"));
        }
        let before = self.controls[owner.control as usize].state;
        if before.pending_last_strong || !before.allocated || status == RuntimeStatus::AbiViolation
        {
            return Err(violation("operation reenters a pending or deallocated control"));
        }
        validate_transition(TransitionClaim::Control {
            operation,
            before,
            status,
            bool_result: boolean,
            after,
        })?;
        let issues = !release && status == RuntimeStatus::Ok;
        if result != if issues { Some(self.next_owner()?) } else { None } {
            return Err(violation("only successful clone or upgrade may issue one fresh owner"));
        }
        if release {
            if status != RuntimeStatus::Ok {
                return Err(violation("verified release is infallible"));
            }
            if let Location::Payload(pending) = owner.location {
                self.next_drop(pending, Drop::Handle(id))?;
            }
            self.owners[id as usize].location = Location::Dropped;
        }
        self.controls[owner.control as usize].state = after;
        if issues {
            self.owners.push(Owner {
                control: owner.control,
                weak: matches!(
                    operation,
                    LogicalOperation::WeakClone | LogicalOperation::WeakDowngrade
                ),
                location: Location::External,
            });
        }
        if after.pending_last_strong {
            self.pending.push(owner.control);
        }
        if !after.allocated {
            self.intervals.remove(&self.controls[owner.control as usize].base);
        }
        Ok(())
    }

    fn finish(&mut self, id: u32, after: ControlState) -> Result<(), RuntimeAbiViolation> {
        if self.pending.last() != Some(&id) {
            return Err(violation("last-release receipt is foreign or replayed"));
        }
        let control = &mut self.controls[id as usize];
        if control.next != control.drops.len() {
            return Err(violation("last-release finish precedes complete payload destruction"));
        }
        let before = ControlState { payload_initialized: false, ..control.state };
        validate_transition(TransitionClaim::Control {
            operation: LogicalOperation::StrongReleaseFinish,
            before,
            status: RuntimeStatus::Ok,
            bool_result: None,
            after,
        })?;
        control.state = after;
        self.pending.pop();
        if !after.allocated {
            self.intervals.remove(&control.base);
        }
        Ok(())
    }
}
