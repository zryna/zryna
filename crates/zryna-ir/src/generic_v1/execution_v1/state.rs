//! Complete-root initialization, pending values and live loans in the new execution domain.
use super::{Failure, budget, plan, raw, reject, reserve, resource_failure, typed};
mod access;
#[cfg(test)]
mod capacity_tests;
mod census;
#[cfg(test)]
mod container_tests;
mod instructions;
mod loans;
mod storage;
#[cfg(test)]
mod tests;
mod transfers;

#[derive(Debug, Eq, PartialEq)]
struct State {
    initialized: Vec<bool>,
    partial: Vec<bool>,
    live_values: Vec<bool>,
    live_loans: Vec<bool>,
    owners: Vec<u32>,
    loan_order: Vec<u32>,
    variants: Vec<Option<u32>>,
    tags: Vec<Option<u32>>,
}
struct Context<'a> {
    program: &'a raw::Program,
    typed: &'a typed::Typed,
    function: usize,
}
impl Context<'_> {
    fn raw(&self) -> &raw::Function {
        &self.program.functions[self.function]
    }
    fn checked(&self) -> &typed::Function {
        &self.typed.functions[self.function]
    }
    fn affine(&self, ty: raw::ValueType) -> Result<bool, Failure> {
        match ty {
            raw::ValueType::Unit => Ok(false),
            raw::ValueType::Stored(id) => self
                .typed
                .facts
                .get(id as usize)
                .map(|f| !f.copy)
                .ok_or_else(|| reject("state value type is foreign")),
        }
    }
}

pub(super) fn replay(
    program: &raw::Program,
    typed: &typed::Typed,
    fi: usize,
    plan: &mut plan::Plan,
    spent: &mut usize,
) -> Result<(), Failure> {
    let context = Context { program, typed, function: fi };
    let f = context.raw();
    let checked = context.checked();
    let b = f.blocks.len();
    let mut work = *spent;
    census::precharge(f, checked.values.len(), &mut work)?;
    let initial = State::new(&context)?;
    let mut incoming = reserve(b)?;
    incoming.resize_with(b, || None);
    incoming[0] = Some(initial);
    let mut queue = reserve(b)?;
    queue.push(0usize);
    while let Some(bi) = queue.pop() {
        let mut state = incoming[bi]
            .as_ref()
            .ok_or_else(|| reject("missing incoming execution state"))?
            .copy()?;
        for (ii, i) in f.blocks[bi].instructions.iter().enumerate() {
            instructions::execute(&context, &mut state, bi, ii, i, plan, &mut work)?;
        }
        transfers::terminator(&context, &state, bi, plan, &mut incoming, &mut queue, &mut work)?;
    }
    if incoming.iter().any(Option::is_none) {
        return Err(reject("execution state did not visit every claimed block"));
    }
    *spent = work;
    Ok(())
}

impl State {
    fn new(c: &Context<'_>) -> Result<Self, Failure> {
        let f = c.raw();
        let checked = c.checked();
        let mut state = Self {
            initialized: filled(f.places.len(), false)?,
            partial: filled(f.places.len(), false)?,
            live_values: filled(checked.values.len(), false)?,
            live_loans: filled(f.loans.len(), false)?,
            owners: reserve(f.places.len())?,
            loan_order: reserve(f.loans.len())?,
            variants: filled(f.places.len(), None)?,
            tags: filled(checked.values.len(), None)?,
        };
        for parameter in &f.parameters {
            match parameter {
                raw::Parameter::Value(d) => state.publish(c, d, None)?,
                raw::Parameter::Loan(d) => {
                    let row = checked
                        .places
                        .loans
                        .get(d.id as usize)
                        .ok_or_else(|| reject("formal loan row absent"))?;
                    if !row.formal {
                        return Err(reject("signature loan lacks exact formal origin"));
                    }
                    state.loan_room()?;
                    state.live_loans[d.id as usize] = true;
                    state.loan_order.push(d.id);
                }
            }
        }
        if state.loan_order.len() > crate::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION {
            return Err(resource_failure("execution active formal loans exceed inherited limit"));
        }
        Ok(state)
    }
    fn copy(&self) -> Result<Self, Failure> {
        Ok(Self {
            initialized: copied(&self.initialized)?,
            partial: copied(&self.partial)?,
            live_values: copied(&self.live_values)?,
            live_loans: copied(&self.live_loans)?,
            owners: copied_capacity(&self.owners, self.initialized.len())?,
            loan_order: copied_capacity(&self.loan_order, self.live_loans.len())?,
            variants: copied(&self.variants)?,
            tags: copied(&self.tags)?,
        })
    }
    fn whole_actions(&self, work: &mut usize) -> Result<Vec<raw::CleanupAction>, Failure> {
        let n = self
            .owners
            .len()
            .checked_add(self.loan_order.len())
            .ok_or_else(|| resource_failure("execution cleanup snapshot overflow"))?;
        budget::charge(work, n)?;
        let mut actions = reserve(n)?;
        actions.extend(self.loan_order.iter().rev().copied().map(raw::CleanupAction::EndLoan));
        actions.extend(self.owners.iter().rev().copied().map(raw::CleanupAction::DropPlace));
        Ok(actions)
    }
    fn same(&self, other: &Self, c: &Context<'_>) -> Result<bool, Failure> {
        if self.initialized != other.initialized
            || self.partial != other.partial
            || self.live_loans != other.live_loans
            || self.owners != other.owners
            || self.loan_order != other.loan_order
            || self.variants != other.variants
        {
            return Ok(false);
        }
        for (id, ty) in c.checked().values.iter().enumerate() {
            if c.affine(*ty)? && self.live_values[id] != other.live_values[id] {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
fn filled<T: Clone>(n: usize, value: T) -> Result<Vec<T>, Failure> {
    let mut result = reserve(n)?;
    result.resize(n, value);
    Ok(result)
}
fn copied<T: Clone>(rows: &[T]) -> Result<Vec<T>, Failure> {
    let mut result = reserve(rows.len())?;
    result.extend_from_slice(rows);
    Ok(result)
}

// Owner/loan arenas are source-verified bounds, not the previous allocation's spare capacity.
// Each state copy retains the full future slot inventory paid by census::precharge.
fn copied_capacity<T: Clone>(rows: &[T], capacity: usize) -> Result<Vec<T>, Failure> {
    if rows.len() > capacity {
        return Err(reject("state order exceeds its retained authority arena"));
    }
    let mut result = reserve(capacity)?;
    result.extend_from_slice(rows);
    Ok(result)
}

#[cfg(test)]
mod continuation_tests;

#[cfg(test)]
mod partial_tests;

#[cfg(test)]
mod static_loan_tests;

#[cfg(test)]
mod move_order_tests;

#[cfg(test)]
mod edge_order_tests;

#[cfg(test)]
mod loop_tests;

#[cfg(test)]
mod loop_boundary_tests;

#[cfg(test)]
mod loop_variant_tests;

#[cfg(test)]
mod borrowed_match_tests;

#[cfg(test)]
mod indexed_tests;
