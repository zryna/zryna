//! Source/family enum identities, active payloads and canonical exhaustive match transfers.

use super::{Builder, Value};
use crate::generic_v1::{
    Failure, raw, reject, reserve,
    source_types::{Closed, type_id},
};
use zryna_syntax::v4::RawMatchArm;

pub(super) struct Description<'a> {
    pub ty: Closed,
    pub variants: Vec<(&'a str, Option<Closed>)>,
}

mod identity;
use identity::arguments;
pub(super) use identity::describe;

impl<'b> Builder<'_, 'b> {
    #[allow(
        clippy::too_many_lines,
        reason = "The original arm replay and edge-specific affine merge form one source authentication step."
    )]
    pub(super) fn match_expression(
        &mut self,
        expression: u32,
        arms: &'b [RawMatchArm],
        span: zryna_source::UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let scrutinee = self.expression(expression, depth + 1)?;
        self.budget.nominal_key(&scrutinee.ty, 2)?;
        let (key, mode) = match &scrutinee.ty {
            Closed::Stored(key) => (key.clone(), raw::MatchMode::Value),
            Closed::Borrow(key, false) => (key.clone(), raw::MatchMode::SharedBorrow),
            Closed::Borrow(key, true) => (key.clone(), raw::MatchMode::ExclusiveBorrow),
            Closed::Unit => return Err(reject("cannot match unit")),
        };
        let first = arms.first().ok_or_else(|| reject("empty match"))?;
        let decoded_arguments = arguments(&key)?;
        self.type_facts.description(
            &self.resolver,
            &first.type_name.text,
            &decoded_arguments,
            &mut self.budget,
        )?;
        let description =
            describe(&self.resolver, &first.type_name.text, &decoded_arguments, self.symbolic)?;
        if !matches!(&description.ty, Closed::Stored(expected) if expected == &key)
            || arms.len() != description.variants.len()
        {
            return Err(reject("inexact enum match coverage"));
        }
        let entry = self.block;
        // Match retains the same complete lexical/availability/loan state as a branch.
        // Charge before each copy, including full binding keys and expanded value IDs.
        self.charge_branch_state()?;
        let saved = self.locals.clone();
        let prior_scope = self.scope_start;
        if mode == raw::MatchMode::Value && self.affine[scrutinee.id as usize] {
            self.consume(scrutinee.id)?;
        }
        let saved_alive = self.alive.clone();
        let saved_loans = self.loans.clone();
        let saved_parents = self.loan_parents.clone();
        let mut successors = reserve(arms.len())?;
        let mut outputs = reserve(arms.len())?;
        for (ordinal, (name, payload)) in description.variants.iter().enumerate() {
            let mut selected = arms.iter().filter(|a| a.variant.text == *name);
            let arm = selected.next().ok_or_else(|| reject("missing enum ordinal"))?;
            self.type_facts.description(
                &self.resolver,
                &arm.type_name.text,
                &decoded_arguments,
                &mut self.budget,
            )?;
            if selected.next().is_some()
                || describe(&self.resolver, &arm.type_name.text, &decoded_arguments, self.symbolic)?
                    .ty
                    != description.ty
            {
                return Err(reject("duplicate/foreign enum match arm"));
            }
            self.charge_branch_state()?;
            self.locals = saved.clone();
            self.scope_start = saved.len();
            self.alive.clone_from(&saved_alive);
            self.alive.resize(self.next as usize, false);
            self.loans = saved_loans.clone();
            self.loan_parents = saved_parents.clone();
            self.block = self.new_block(arm.span)?;
            let target = self.block;
            let from = self.next as usize;
            let binding = match (&arm.binding, payload) {
                (Some(name), Some(ty)) => {
                    let ty = if mode == raw::MatchMode::Value {
                        ty.clone()
                    } else {
                        let Closed::Stored(key) = ty else {
                            return Err(Failure::InternalFailure);
                        };
                        Closed::Borrow(key.clone(), mode == raw::MatchMode::ExclusiveBorrow)
                    };
                    let value = self.value(ty)?;
                    let definition = self.definition(&value)?;
                    self.blocks[target].parameters.push(definition.clone());
                    if mode != raw::MatchMode::Value {
                        let root = *self.loans.get(&scrutinee.id).ok_or_else(|| {
                            super::owners::ownership("missing retained source loan")
                        })?;
                        self.loans.insert(value.id, root);
                        self.loan_parents.insert(value.id, scrutinee.id);
                    }
                    self.bind(&name.text, value)?;
                    Some(definition)
                }
                (None, None) => None,
                _ => return Err(reject("inexact active payload binding")),
            };
            let value = self.expression(arm.value, depth + 1)?;
            if matches!(value.ty, Closed::Borrow(..)) {
                return Err(super::owners::ownership("loan cannot escape match result"));
            }
            self.close_scope(from, Some(value.id), arm.span)?;
            self.charge_branch_state()?;
            outputs.push((self.block, value, self.alive.clone()));
            successors.push(raw::Arm {
                ordinal: u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                binding,
                edge: raw::Edge {
                    target: u32::try_from(target).map_err(|_| Failure::InternalFailure)?,
                    arguments: Vec::new(),
                },
            });
        }
        let result_type = outputs[0].1.ty.clone();
        if outputs.iter().any(|(_, v, _)| v.ty != result_type) {
            return Err(reject("different enum arm result types"));
        }
        self.charge_branch_state()?;
        let mut common = saved_alive.clone();
        for (id, available) in common.iter_mut().enumerate() {
            *available &= outputs.iter().all(|(_, _, s)| s.get(id).copied().unwrap_or(false));
        }
        for (block, value, state) in &outputs {
            self.block = *block;
            self.charge_branch_state()?;
            self.alive.clone_from(state);
            self.alive.resize(self.next as usize, false);
            self.loans = saved_loans.clone();
            self.loan_parents = saved_parents.clone();
            for id in (0..common.len()).rev() {
                if self.affine[id] && self.alive[id] && !common[id] {
                    self.ext(
                        Closed::Unit,
                        span,
                        super::Owned::Drop(
                            u32::try_from(id).map_err(|_| Failure::InternalFailure)?,
                        ),
                    )?;
                }
            }
            // The edge transfers each arm result into one new joined owner.
            if self.affine[value.id as usize] {
                self.consume(value.id)?;
            }
        }
        self.charge_branch_state()?;
        self.alive = common;
        self.alive.resize(self.next as usize, false);
        self.loans = saved_loans;
        self.loan_parents = saved_parents;
        self.block = self.new_block(span)?;
        let value = self.value(result_type)?;
        let definition = self.definition(&value)?;
        self.blocks[self.block].parameters.push(definition);
        for (block, out, _) in outputs {
            self.blocks[block].terminator = raw::Terminator::Jump(raw::Edge {
                target: u32::try_from(self.block).map_err(|_| Failure::InternalFailure)?,
                arguments: vec![out.id],
            });
        }
        let ty = if self.symbolic {
            0
        } else {
            let raw::Type::Stored(id) = type_id(self.program, Closed::Stored(key))? else {
                return Err(Failure::InternalFailure);
            };
            id
        };
        self.blocks[entry].span = span;
        self.blocks[entry].terminator = raw::Terminator::ClosedEnumMatch {
            ty,
            scrutinee: scrutinee.id,
            mode,
            arms: successors,
        };
        self.locals = saved;
        self.scope_start = prior_scope;
        if mode != raw::MatchMode::Value
            && matches!(
                self.original.body.expressions[expression as usize].kind,
                zryna_syntax::v5::RawExpressionKind::Borrow { .. }
                    | zryna_syntax::v5::RawExpressionKind::BorrowMut { .. }
            )
        {
            self.ext(Closed::Unit, span, super::Owned::EndLoan(scrutinee.id))?;
        }
        Ok(value)
    }
}
