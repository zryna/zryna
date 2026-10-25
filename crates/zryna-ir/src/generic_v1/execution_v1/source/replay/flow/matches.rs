use super::{
    Body, Domain, Failure, State, UntrustedSpan, Value, charge, copy_key, keys, originals, raw,
    reject, reserve,
};
use zryna_syntax::v4::RawMatchArm;
mod borrowed;
mod borrowed_source;
impl<'s> Body<'_, 's> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn match_expression(
        &mut self,
        index: u32,
        arms: &'s [RawMatchArm],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        if let Some(source) = self.borrowed_scrutinee(index, span, depth)? {
            return self.borrowed_match(source, arms, span, depth);
        }
        let value = self.expression(index, depth + 1)?;
        let raw::ValueType::Stored(ty) = value.ty else {
            return Err(reject("match requires a stored enum"));
        };
        let (args, desc) = self.match_description(ty, value.ty, arms)?;
        // Copy values need an addressable source just like affine pending values.
        let source = if let Some(p) = self.owners[value.id as usize] {
            p
        } else {
            let p = self.place(ty, raw::PlaceKind::Temporary(value.id), span)?;
            self.instruction(
                None,
                raw::Operation::InitializePlace { place: p, value: value.id },
                span,
            )?;
            self.initialize(p)?;
            p
        };
        let copy = self.copy(ty)?;
        self.consume_place(source, span)?;
        let entry = self.block;
        let mut saved = self.snapshot()?;
        let locals = saved.locals.len();
        let start = self.output.places.len();
        charge(self.spent, arms.len() * 2)?;
        let mut successors = reserve(arms.len())?;
        let mut outputs = reserve(arms.len())?;
        let mut result_type = None;
        for (ordinal, (name, payload)) in desc.members.iter().enumerate() {
            let mut selected = None;
            for arm in arms {
                charge(self.spent, arm.variant.text.len().min(name.len()) + 1)?;
                if arm.variant.text == *name {
                    if selected.is_some() {
                        return Err(reject("duplicate original match arm"));
                    }
                    selected = Some(arm);
                }
            }
            let arm = selected.ok_or_else(|| reject("missing exact enum arm"))?;
            // Every arm's type name must authenticate the same complete nominal.
            let other =
                originals::describe(&self.resolver, &arm.type_name.text, &args, self.spent)?;
            if self.closed_borrowed(&other.ty)? != value.ty {
                return Err(reject("match arm changes nominal identity"));
            }
            let state = self.copy_state(&saved)?;
            self.restore(state)?;
            self.block = self.new_block()?;
            let target = self.block;
            let binding = self.match_binding(arm, payload.as_ref())?;
            let output = self.expression(arm.value, depth + 1)?;
            if let Some(t) = result_type {
                if t != output.ty {
                    return Err(reject("match branches have different result types"));
                }
            } else {
                result_type = Some(output.ty);
            }
            self.consume(output, arm.span)?;
            self.close_match_arm(&mut saved, source, copy, (locals, start), arm.span)?;
            outputs.push((self.block, output));
            successors.push(raw::EnumArm {
                ordinal: u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                binding,
                edge: raw::Edge {
                    target: u32::try_from(target).map_err(|_| Failure::InternalFailure)?,
                    arguments: Vec::new(),
                },
            });
        }
        let result_type = result_type.ok_or(Failure::InternalFailure)?;
        if copy {
            charge(self.spent, 1)?;
            saved.live[source as usize] = false;
        }
        self.restore(saved)?;
        let result = self.match_join(result_type, outputs, span)?;
        self.output.blocks[entry].terminator = raw::Terminator::ClosedEnumMatch {
            ty,
            scrutinee: raw::PlaceOrLoan::Place(source),
            mode: raw::MatchMode::Value,
            arms: successors,
        };
        self.activate(result.id)?;
        Ok(Value { id: result.id, ty: result.ty })
    }
    fn close_match_arm(
        &mut self,
        saved: &mut State<'s>,
        source: u32,
        copy: bool,
        scope: (usize, usize),
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        self.end_created_loans(saved.loans.len(), span)?;
        self.close_scope_storage(scope.0, scope.1, span)?;
        if copy {
            self.drop_place(source, span)?;
            charge(self.spent, 2)?;
            saved.live[source as usize] = false;
            self.same_state(saved)?;
            saved.live[source as usize] = true;
        } else {
            self.same_state(saved)?;
        }
        Ok(())
    }
    fn match_join(
        &mut self,
        result_type: raw::ValueType,
        outputs: Vec<(usize, Value)>,
        span: UntrustedSpan,
    ) -> Result<raw::ValueDefinition, Failure> {
        let join = self.new_block()?;
        self.block = join;
        let result = self.definition(result_type, true, span)?;
        charge(self.spent, 1)?;
        self.output.blocks[join]
            .value_parameters
            .try_reserve(1)
            .map_err(|_| Failure::AllocationFailure)?;
        self.output.blocks[join].value_parameters.push(result.clone());
        for (block, value) in outputs {
            charge(self.spent, 1)?;
            let mut arguments = reserve(1)?;
            arguments.push(value.id);
            self.output.blocks[block].terminator = raw::Terminator::Jump(raw::Edge {
                target: u32::try_from(join).map_err(|_| Failure::InternalFailure)?,
                arguments,
            });
        }
        Ok(result)
    }
    fn match_description(
        &mut self,
        ty: u32,
        value_type: raw::ValueType,
        arms: &'s [RawMatchArm],
    ) -> Result<(Vec<Vec<u8>>, originals::Description<'s>), Failure> {
        charge(self.spent, 1)?;
        let key = &self.program.type_keys[ty as usize];
        charge(
            self.spent,
            key.len()
                .checked_mul(4)
                .and_then(|n| n.checked_add(67))
                .ok_or(Failure::InternalFailure)?,
        )?;
        let decoded = keys::decode(key, Domain::Type)?;
        charge(self.spent, decoded.arguments().len())?;
        let mut args = reserve(decoded.arguments().len())?;
        for key in decoded.arguments() {
            args.push(copy_key(key, self.spent)?);
        }
        let first = arms.first().ok_or_else(|| reject("empty enum match"))?;
        let desc = originals::describe(&self.resolver, &first.type_name.text, &args, self.spent)?;
        if desc.structure
            || self.closed_borrowed(&desc.ty)? != value_type
            || arms.len() != desc.members.len()
        {
            return Err(reject("match identity/coverage differs from original enum"));
        }
        Ok((args, desc))
    }
    fn match_binding(
        &mut self,
        arm: &'s RawMatchArm,
        payload: Option<&crate::generic_v1::source_types::Closed>,
    ) -> Result<Option<raw::MatchBinding>, Failure> {
        let target = self.block;
        let binding = match (&arm.binding, payload) {
            (Some(name), Some(t)) => {
                let t = self.closed_borrowed(t)?;
                let d = self.definition(t, true, arm.span)?;
                charge(self.spent, 1)?;
                self.output.blocks[target]
                    .value_parameters
                    .try_reserve(1)
                    .map_err(|_| Failure::AllocationFailure)?;
                self.output.blocks[target].value_parameters.push(d.clone());
                self.activate(d.id)?;
                let raw::ValueType::Stored(t) = t else {
                    return Err(reject("payload cannot be unit"));
                };
                let p = if let Some(p) = self.owners[d.id as usize] {
                    p
                } else {
                    let p = self.place(t, raw::PlaceKind::Temporary(d.id), arm.span)?;
                    self.instruction(
                        None,
                        raw::Operation::InitializePlace { place: p, value: d.id },
                        arm.span,
                    )?;
                    self.initialize(p)?;
                    p
                };
                self.bind(&name.text, p, t)?;
                Some(raw::MatchBinding::Value(d))
            }
            (None, None) => None,
            _ => return Err(reject("match payload binding differs from original variant")),
        };
        Ok(binding)
    }
    fn copy_state(&mut self, s: &State<'s>) -> Result<State<'s>, Failure> {
        charge(
            self.spent,
            s.locals.len()
                + s.live.len()
                + s.partial.len() * 2
                + s.order.len()
                + s.loans.len()
                + s.loan_order.len(),
        )?;
        let mut locals = reserve(s.locals.len())?;
        locals.extend(s.locals.iter().cloned());
        let mut live = reserve(s.live.len())?;
        live.extend_from_slice(&s.live);
        let mut partial = reserve(s.partial.len())?;
        partial.extend_from_slice(&s.partial);
        let mut order = reserve(s.order.len())?;
        order.extend_from_slice(&s.order);
        let mut loans = reserve(s.loans.len())?;
        loans.extend_from_slice(&s.loans);
        let mut loan_order = reserve(s.loan_order.len())?;
        loan_order.extend_from_slice(&s.loan_order);
        Ok(State { locals, live, partial, order, loans, loan_order })
    }
}
