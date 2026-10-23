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
struct Analysis<'c, 'p, 'a> {
    c: &'c Context<'p, 'a>,
    active: Vec<bool>,
}
pub(in crate::generic_execution_v1) fn prove(c: &Context<'_, '_>) -> Result<Quota, Diagnostic> {
    charge(&c.work, c.functions.len())?;
    let mut active = Vec::new();
    active.try_reserve_exact(c.functions.len()).map_err(|_| emission_budget())?;
    active.resize(c.functions.len(), false);
    let mut analysis = Analysis { c, active };
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
        let args = filled(c, f.parameters.len(), Cost::default())?;
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
    fn function(&mut self, id: usize, args: &[Cost], depth: usize) -> Result<Summary, Diagnostic> {
        if depth > zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH
            || self.active.get(id).copied().unwrap_or(true)
        {
            return Err(emission_budget());
        }
        self.active[id] = true;
        let c = self.c;
        let f = c.functions.get(id).ok_or_else(internal)?;
        if !f.loans.is_empty() || args.len() != f.parameters.len() {
            return Err(internal());
        }
        let mut values = filled(c, f.values.len(), Cost::default())?;
        let mut places = filled(c, f.places.len(), Cost::default())?;
        charge(
            &c.work,
            f.parameters.len().checked_add(f.places.len()).ok_or_else(emission_budget)?,
        )?;
        for (p, arg) in f.parameters.iter().zip(args) {
            let VerifiedParameter::Value(v) = p else {
                return Err(internal());
            };
            values[v.id() as usize] = normalized(c, v.ty(), *arg)?;
        }
        for place in &f.places {
            if let raw::PlaceKind::Parameter(ordinal) = place.kind() {
                let VerifiedParameter::Value(v) = f.parameters[ordinal as usize] else {
                    return Err(internal());
                };
                places[place.id() as usize] = values[v.id() as usize];
            }
        }
        let mut summary = Summary { result: Cost::default(), cost: Cost::default() };
        for bi in cfg::order(c, f)? {
            let block = f.blocks[bi];
            charge(&c.work, block.operations().len().checked_add(1).ok_or_else(emission_budget)?)?;
            for op in block.operations() {
                let (result, cost) = self.operation(f, op, &values, &mut places, depth)?;
                summary.cost = summary.cost.add(cost)?;
                if let Some(v) = op.result() {
                    values[v.id() as usize] = result;
                }
            }
            self.terminator(f, block, &mut values, &places, &mut summary)?;
        }
        self.active[id] = false;
        Ok(summary)
    }
}
fn source(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    places: &[Cost],
    id: u32,
) -> Result<Cost, Diagnostic> {
    let p = f.place(id)?;
    normalized(c, raw::ValueType::Stored(p.ty()), places[p.root().id() as usize])
}
