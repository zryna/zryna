//! Ordered census and private-value frame allocation.
use super::{
    CopySite, Diagnostic, MAX_PARAMETERS, MAX_VALUE_LANES, Record, Shape, allocate, budget,
    copy_cost, cost, invariant, reserve, shape, take,
};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedFunction, VerifiedOperation, VerifiedParameter, raw,
};
pub(super) struct Values {
    pub records: Vec<Record>,
    pub parameters: Vec<Option<u32>>,
    pub end: u64,
}
fn census(f: VerifiedFunction<'_, '_>, work: &mut u64) -> Result<usize, Diagnostic> {
    take(
        work,
        (f.blocks().len() as u64)
            .checked_mul(128)
            .and_then(|n| n.checked_add((f.parameters().len() as u64).checked_mul(128)?))
            .ok_or_else(budget)?,
    )?;
    let mut count = f.parameters().filter(|p| matches!(p, VerifiedParameter::Value(_))).count();
    for b in f.blocks() {
        take(
            work,
            (b.operations().len() as u64)
                .checked_mul(256)
                .and_then(|n| n.checked_add((b.value_parameters().len() as u64).checked_mul(128)?))
                .ok_or_else(budget)?,
        )?;
        count = count.checked_add(b.value_parameters().len()).ok_or_else(budget)?;
        for op in b.operations() {
            count = count.checked_add(usize::from(op.result().is_some())).ok_or_else(budget)?;
        }
    }
    take(
        work,
        (count as u64)
            .checked_mul(16)
            .and_then(|n| n.checked_add((f.places().len() as u64).checked_mul(128)?))
            .and_then(|n| n.checked_add((f.loans().len() as u64).checked_mul(128)?))
            .ok_or_else(budget)?,
    )?;
    Ok(count)
}
pub(super) fn derive(
    f: VerifiedFunction<'_, '_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<Values, Diagnostic> {
    let count = census(f, work)?;
    let mut values = reserve(count)?;
    values.resize(count, Record { offset: 0, ty: raw::ValueType::Unit });
    let mut end = 0;
    let mut lanes = 0u64;
    let mut parameter_lanes = 1u64;
    let mut parameter_values = reserve(f.parameters().len())?;
    for parameter in f.parameters() {
        match parameter {
            VerifiedParameter::Value(value) => {
                let width = match value.ty() {
                    raw::ValueType::Unit => 0,
                    raw::ValueType::Stored(id) => shape(types, id)?.lanes,
                };
                parameter_lanes = parameter_lanes.checked_add(width).ok_or_else(budget)?;
                lanes = lanes.checked_add(width).ok_or_else(budget)?;
                *values.get_mut(value.id() as usize).ok_or_else(invariant)? =
                    allocate(types, value.ty(), &mut end)?;
                copy_cost(CopySite::Formal, types, value.ty(), work)?;
                parameter_values.push(Some(value.id()));
            }
            VerifiedParameter::Loan(_) => {
                parameter_lanes = parameter_lanes.checked_add(1).ok_or_else(budget)?;
                parameter_values.push(None);
            }
        }
    }
    if parameter_lanes > u64::from(MAX_PARAMETERS)
        || f.parameters().len().checked_add(1).is_none_or(|count| count > MAX_PARAMETERS as usize)
    {
        return Err(budget());
    }
    cost(types, f.result_type(), work)?;
    for b in f.blocks() {
        for value in
            b.value_parameters().chain(b.operations().filter_map(VerifiedOperation::result))
        {
            let width = match value.ty() {
                raw::ValueType::Unit => 0,
                raw::ValueType::Stored(id) => shape(types, id)?.lanes,
            };
            lanes = lanes.checked_add(width).ok_or_else(budget)?;
            *values.get_mut(value.id() as usize).ok_or_else(invariant)? =
                allocate(types, value.ty(), &mut end)?;
            cost(types, value.ty(), work)?;
        }
    }
    if lanes > u64::from(MAX_VALUE_LANES) {
        return Err(budget());
    }
    Ok(Values { records: values, parameters: parameter_values, end })
}
