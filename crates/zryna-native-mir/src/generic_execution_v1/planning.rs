//! Upfront native work and exact physical static-place projection.
use super::containers::Extensions;
use super::{
    Diagnostic, FunctionPlan, LoanAddress, MAX_CODEGEN_UNITS, MAX_PARAMETERS, MAX_VALUE_LANES,
    Record, budget, invariant, shapes::Shape,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedFunction, raw};
mod emission;
mod projections;
mod values;
pub(super) fn take(work: &mut u64, amount: u64) -> Result<(), Diagnostic> {
    *work = work.checked_add(amount).filter(|n| *n <= MAX_CODEGEN_UNITS).ok_or_else(budget)?;
    Ok(())
}
pub(super) fn reserve<T>(count: usize) -> Result<Vec<T>, Diagnostic> {
    #[cfg(test)]
    super::resource_tests::trace::reserve(count);
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(|_| budget())?;
    Ok(rows)
}
fn shape<'a>(types: &[Shape<'a>], ty: u32) -> Result<Shape<'a>, Diagnostic> {
    types.get(ty as usize).copied().ok_or_else(invariant)
}
fn cost(types: &[Shape<'_>], ty: raw::ValueType, work: &mut u64) -> Result<(), Diagnostic> {
    if let raw::ValueType::Stored(id) = ty {
        let s = shape(types, id)?;
        take(
            work,
            s.expanded
                .checked_mul(256)
                .and_then(|n| n.checked_add(s.view.size().div_ceil(8).checked_mul(64)?))
                .ok_or_else(budget)?,
        )?;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CopySite {
    Formal,
    Initialize,
    Return,
}
fn copy_cost(
    site: CopySite,
    types: &[Shape<'_>],
    ty: raw::ValueType,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    #[cfg(test)]
    let before = *work;
    cost(types, ty, work)?;
    #[cfg(test)]
    super::resource_tests::trace::copy(site, ty, *work - before);
    #[cfg(not(test))]
    let _ = site;
    Ok(())
}

fn allocate(types: &[Shape<'_>], ty: raw::ValueType, end: &mut u64) -> Result<Record, Diagnostic> {
    let (size, align) = match ty {
        raw::ValueType::Unit => (0, 1),
        raw::ValueType::Stored(id) => {
            let s = shape(types, id)?;
            (s.view.size(), s.view.alignment())
        }
    };
    let start = end
        .checked_add(align.checked_sub(1).ok_or_else(invariant)?)
        .map(|n| n & !(align - 1))
        .ok_or_else(budget)?;
    *end =
        start.checked_add(size.max(1)).filter(|n| i32::try_from(*n).is_ok()).ok_or_else(budget)?;
    Ok(Record { offset: u32::try_from(start).map_err(|_| budget())?, ty })
}
pub(super) fn function<'p, 'a>(
    f: VerifiedFunction<'p, 'a>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(FunctionPlan<'p, 'a>, Extensions), Diagnostic> {
    let mut frame = values::derive(f, types, work)?;
    let places =
        projections::places(f, &frame.records, &frame.parameters, types, &mut frame.end, work)?;
    let (loans, loan_types) = projections::loans(f, &places, types, work)?;
    let extensions = emission::check(f, types, &frame.records, &places, work)?;
    take(work, frame.end.div_ceil(8).checked_mul(64).ok_or_else(budget)?)?;
    Ok((
        FunctionPlan {
            source: f,
            symbol: format!("zryna_gexecution_v1_{}", f.index()),
            frame_bytes: u32::try_from(frame.end.max(1)).map_err(|_| budget())?,
            values: frame.records,
            places,
            loans,
            loan_types,
        },
        extensions,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn inherited_codegen_ceiling_and_first_extra_are_exact() {
        let mut work = 0;
        super::take(&mut work, super::MAX_CODEGEN_UNITS).expect("exact");
        assert_eq!(super::take(&mut work, 1).expect_err("extra").code, "ZRYNA-N7001");
        let mut overflow = u64::MAX;
        assert!(super::take(&mut overflow, 1).is_err());
    }
}
