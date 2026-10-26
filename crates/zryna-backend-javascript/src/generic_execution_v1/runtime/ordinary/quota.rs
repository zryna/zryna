//! Conservative per-entry acquisition/release proof; no runtime status is repurposed.
use super::super::super::{
    Context,
    context::{Function, charge},
    emission_budget, internal,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedParameter, raw};
use zryna_layout::TypeCategory;
mod cfg;
mod cyclic;
mod frame;
mod operations;
#[derive(Clone, Copy, Default)]
pub(super) struct Cost {
    pub(super) handles: usize,
    pub(super) allocations: usize,
    pub(super) releases: usize,
}
impl Cost {
    fn add(self, other: Self) -> Result<Self, Diagnostic> {
        let result = Self {
            handles: self.handles.checked_add(other.handles).ok_or_else(emission_budget)?,
            allocations: self
                .allocations
                .checked_add(other.allocations)
                .ok_or_else(emission_budget)?,
            releases: self.releases.checked_add(other.releases).ok_or_else(emission_budget)?,
        };
        let transitions = limit(zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS)?;
        if result.allocations > limit(zryna_ownership_runtime_abi::MAX_ALLOCATION_OPERATIONS)?
            || result.allocations > limit(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)?
            || result.handles.checked_add(result.releases).is_none_or(|n| n > transitions)
        {
            return Err(emission_budget());
        }
        Ok(result)
    }
    fn maximum(self, other: Self) -> Self {
        Self {
            handles: self.handles.max(other.handles),
            allocations: self.allocations.max(other.allocations),
            releases: self.releases.max(other.releases),
        }
    }
}
fn limit(value: u64) -> Result<usize, Diagnostic> {
    usize::try_from(value).map_err(|_| emission_budget())
}
#[derive(Clone, Copy, Default)]
pub(in crate::generic_execution_v1) struct Quota {
    pub(in crate::generic_execution_v1) allocations: usize,
    pub(in crate::generic_execution_v1) transitions: usize,
}
struct Summary {
    result: Cost,
    cost: Cost,
}
#[derive(Clone, Copy, Default)]
struct Potential {
    cell: usize,
    projected: bool,
}
#[derive(Clone, Copy)]
enum Argument {
    Value(Cost),
    Loan(Potential),
}
impl Default for Argument {
    fn default() -> Self {
        Self::Value(Cost::default())
    }
}
struct Analysis<'c, 'p, 'a> {
    c: &'c Context<'p, 'a>,
    active: Vec<bool>,
    cells: Vec<Cost>,
}
pub(in crate::generic_execution_v1) fn prove(c: &Context<'_, '_>) -> Result<Quota, Diagnostic> {
    charge(&c.work, c.functions.len())?;
    let mut active = Vec::new();
    active.try_reserve_exact(c.functions.len()).map_err(|_| emission_budget())?;
    active.resize(c.functions.len(), false);
    let mut analysis = Analysis { c, active, cells: Vec::new() };
    let mut quota = Quota::default();
    charge(&c.work, c.program.export_functions().len())?;
    if c.program.scalar_abi().exports().len() != c.program.export_functions().len() {
        return Err(internal());
    }
    for &entry in c.program.export_functions() {
        let f = c.functions.get(entry).ok_or_else(internal)?;
        charge(&c.work, f.parameters.len())?;
        for parameter in &f.parameters {
            let VerifiedParameter::Value(value) = parameter else {
                return Err(internal());
            };
            scalar(c, value.ty())?;
        }
        scalar(c, f.view.result_type())?;
        if let Some(cyclic) = cyclic::prove(c, entry)? {
            quota.allocations = quota.allocations.max(cyclic.allocations);
            quota.transitions = quota.transitions.max(cyclic.transitions);
            continue;
        }
        let args = filled(c, f.parameters.len(), Argument::default())?;
        let summary = analysis.function(entry, &args, 1)?;
        quota.allocations = quota.allocations.max(summary.cost.allocations);
        quota.transitions = quota.transitions.max(
            summary.cost.handles.checked_add(summary.cost.releases).ok_or_else(emission_budget)?,
        );
    }
    Ok(quota)
}
fn scalar(c: &Context<'_, '_>, ty: raw::ValueType) -> Result<(), Diagnostic> {
    let raw::ValueType::Stored(id) = ty else {
        return Err(internal());
    };
    if !matches!(c.ty(id)?.category(), TypeCategory::Bool | TypeCategory::I32) {
        return Err(internal());
    }
    Ok(())
}
fn filled<T: Clone>(c: &Context<'_, '_>, len: usize, value: T) -> Result<Vec<T>, Diagnostic> {
    charge(&c.work, len.checked_mul(size_of::<T>().max(1)).ok_or_else(emission_budget)?)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(len).map_err(|_| emission_budget())?;
    rows.resize(len, value);
    Ok(rows)
}
fn normalized(c: &Context<'_, '_>, ty: raw::ValueType, upper: Cost) -> Result<Cost, Diagnostic> {
    let raw::ValueType::Stored(id) = ty else {
        return Ok(Cost::default());
    };
    Ok(match c.ty(id)?.category() {
        TypeCategory::Bool | TypeCategory::I32 => Cost::default(),
        TypeCategory::String => {
            Cost { handles: 1, allocations: upper.allocations.min(1), releases: 1 }
        }
        TypeCategory::Shared | TypeCategory::Weak => {
            Cost { handles: 1, allocations: 0, releases: 1 }
        }
        _ => upper,
    })
}
impl Analysis<'_, '_, '_> {
    fn function(
        &mut self,
        id: usize,
        args: &[Argument],
        depth: usize,
    ) -> Result<Summary, Diagnostic> {
        if depth > zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH
            || self.active.get(id).copied().unwrap_or(true)
        {
            return Err(emission_budget());
        }
        self.active[id] = true;
        let context = self.c;
        let function = context.functions.get(id).ok_or_else(internal)?;
        if args.len() != function.parameters.len() {
            return Err(internal());
        }
        let mut values = filled(context, function.values.len(), Cost::default())?;
        let frame = self.frame(function, args, &mut values)?;
        let mut summary = Summary { result: Cost::default(), cost: Cost::default() };
        for index in cfg::order(context, function)? {
            let block = function.blocks[index];
            charge(
                &context.work,
                block
                    .operations()
                    .len()
                    .checked_add(block.value_parameters().len())
                    .and_then(|n| n.checked_add(1))
                    .ok_or_else(emission_budget)?,
            )?;
            for parameter in block.value_parameters() {
                if let Some(cell) = frame.temporary[parameter.id() as usize] {
                    self.cells[cell] = values[parameter.id() as usize];
                }
            }
            for operation in block.operations() {
                let (result, cost) = self.operation(function, operation, &values, &frame, depth)?;
                summary.cost = summary.cost.add(cost)?;
                if let Some(value) = operation.result() {
                    values[value.id() as usize] = result;
                    if let Some(cell) = frame.temporary[value.id() as usize] {
                        self.cells[cell] = result;
                    }
                }
            }
            self.terminator(function, block, &mut values, &frame, &mut summary)?;
        }
        self.cells.truncate(frame.marker);
        self.active[id] = false;
        Ok(summary)
    }
}
