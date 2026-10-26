//! Declaration-ordered selected Loan arms retain their source parent and end children before joins.
use super::borrowed_source::Source;
use super::{
    Body, Failure, RawMatchArm, State, UntrustedSpan, Value, charge, originals, raw, reject,
    reserve,
};
use crate::generic_v1::execution_v1::source::proof::matches::{Arm, Use};
impl<'s> Body<'_, 's> {
    pub(super) fn borrowed_match(
        &mut self,
        source: Source,
        arms: &'s [RawMatchArm],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let ty = raw::ValueType::Stored(source.ty);
        let (args, desc) = self.match_description(source.ty, ty, arms)?;
        let access = self.output.loans[source.parent as usize].access;
        let mode = if access == raw::Access::Shared {
            raw::MatchMode::SharedLoan
        } else {
            raw::MatchMode::ExclusiveLoan
        };
        let entry = self.block;
        let saved = self.snapshot()?;
        let locals = saved.locals.len();
        let start = self.output.places.len();
        charge(
            self.spent,
            arms.len().checked_mul(size_of::<Arm>() + 3).ok_or(Failure::InternalFailure)?,
        )?;
        let mut successors = reserve(arms.len())?;
        let mut outputs = reserve(arms.len())?;
        let mut facts = reserve(arms.len())?;
        let mut result_type = None;
        for (ordinal, (name, payload)) in desc.members.iter().enumerate() {
            let arm = self.borrowed_arm(&args, ty, name, arms)?;
            let region = self.original.body.expressions[arm.value as usize].span;
            let state = self.copy_state(&saved)?;
            self.restore(state)?;
            self.block = self.new_block()?;
            let target = self.block;
            let first = self.output.loans.len();
            let ordinal = u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?;
            let binding = self.borrowed_binding(source.parent, ordinal, arm, payload.as_ref())?;
            let output = self.borrowed_output(arm, &saved, (locals, start), first, depth)?;
            if result_type.is_some_and(|t| t != output.ty) {
                return Err(reject("borrowed match arm result types differ"));
            }
            result_type = Some(output.ty);
            outputs.push((self.block, output));
            facts.push(Arm {
                ordinal,
                region,
                binding: binding.clone(),
                blocks: target..self.output.blocks.len(),
            });
            successors.push(raw::EnumArm {
                ordinal,
                binding: binding.map(raw::MatchBinding::Loan),
                edge: raw::Edge {
                    target: u32::try_from(target).map_err(|_| Failure::InternalFailure)?,
                    arguments: Vec::new(),
                },
            });
        }
        self.restore(saved)?;
        let result =
            self.match_join(result_type.ok_or(Failure::InternalFailure)?, outputs, span)?;
        self.output.blocks[entry].terminator = raw::Terminator::ClosedEnumMatch {
            ty: source.ty,
            scrutinee: raw::PlaceOrLoan::Loan(source.parent),
            mode,
            arms: successors,
        };
        self.retain_borrowed_use(entry, span, &source, mode, facts)?;
        self.activate(result.id)?;
        self.end_match_loans(source.first)?;
        Ok(Value { id: result.id, ty: result.ty })
    }
    fn borrowed_output(
        &mut self,
        arm: &'s RawMatchArm,
        saved: &State<'s>,
        scope: (usize, usize),
        first: usize,
        depth: usize,
    ) -> Result<Value, Failure> {
        let region = self.original.body.expressions[arm.value as usize].span;
        let output = self.expression(arm.value, depth + 1)?;
        self.consume(output, region)?;
        self.end_match_loans(first)?;
        self.close_scope_storage(scope.0, scope.1, region)?;
        self.same_state(saved)?;
        Ok(output)
    }
    fn retain_borrowed_use(
        &mut self,
        entry: usize,
        span: UntrustedSpan,
        source: &Source,
        mode: raw::MatchMode,
        arms: Vec<Arm>,
    ) -> Result<(), Failure> {
        charge(
            self.spent,
            self.match_uses
                .len()
                .checked_add(1)
                .and_then(|n| n.checked_mul(size_of::<Use>() + 1))
                .ok_or(Failure::InternalFailure)?,
        )?;
        self.match_uses.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.match_uses.push(Use {
            block: entry,
            span,
            ty: source.ty,
            parent: source.parent,
            mode,
            arms,
            join: self.block,
        });
        Ok(())
    }
    fn borrowed_arm(
        &mut self,
        args: &[Vec<u8>],
        ty: raw::ValueType,
        name: &str,
        arms: &'s [RawMatchArm],
    ) -> Result<&'s RawMatchArm, Failure> {
        let mut selected = None;
        for arm in arms {
            charge(self.spent, arm.variant.text.len().min(name.len()) + 1)?;
            if arm.variant.text == name {
                if selected.is_some() {
                    return Err(reject("duplicate original borrowed match arm"));
                }
                selected = Some(arm);
            }
        }
        let arm = selected.ok_or_else(|| reject("missing exact borrowed match arm"))?;
        let other = originals::describe(&self.resolver, &arm.type_name.text, args, self.spent)?;
        if self.closed_borrowed(&other.ty)? != ty {
            return Err(reject("borrowed match arm changes nominal identity"));
        }
        Ok(arm)
    }
    fn borrowed_binding(
        &mut self,
        parent: u32,
        ordinal: u32,
        arm: &'s RawMatchArm,
        payload: Option<&crate::generic_v1::source_types::Closed>,
    ) -> Result<Option<raw::LoanDefinition>, Failure> {
        match (&arm.binding, payload) {
            (None, None) => Ok(None),
            (Some(name), Some(payload)) => {
                let raw::ValueType::Stored(referent) = self.closed_borrowed(payload)? else {
                    return Err(reject("borrowed payload must have stored referent"));
                };
                let access = self.output.loans[parent as usize].access;
                let region = self.original.body.expressions[arm.value as usize].span;
                let id = self.add_loan(
                    referent,
                    access,
                    raw::LoanOrigin::StaticProjection {
                        parent,
                        projection: raw::StaticProjection::EnumPayload(ordinal),
                    },
                    region,
                    region,
                    false,
                )?;
                charge(self.spent, 1)?;
                self.output.loans[id as usize].region.role = raw::RegionRole::SelectedMatchArm;
                self.bind_loan(&name.text, id, referent, access == raw::Access::Exclusive)?;
                Ok(Some(raw::LoanDefinition { id, referent, access }))
            }
            _ => Err(reject("borrowed match binding differs from exact active payload")),
        }
    }
}
