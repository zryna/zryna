//! Cached actual referent addresses; dynamic cells never masquerade as static Places.
use super::{
    Diagnostic, LoanAddress, Record, Shape, VerifiedFunction, VerifiedLoan, VerifiedLoanOrigin,
    budget, displacement, invariant, raw, reserve, shape, take,
};
pub(in crate::generic_execution_v1::planning) fn derive(
    f: VerifiedFunction<'_, '_>,
    places: &[Record],
    types: &[Shape<'_>],
    end: &mut u64,
    work: &mut u64,
) -> Result<(Vec<LoanAddress>, Vec<u32>), Diagnostic> {
    let mut loan_views = reserve(f.loans().len())?;
    loan_views.extend(f.loans());
    let mut loan_cache = reserve(loan_views.len())?;
    loan_cache.resize(loan_views.len(), None);
    let mut loans = reserve(loan_views.len())?;
    let mut loan_types = reserve(loan_views.len())?;
    loan_types.extend(loan_views.iter().map(|loan| loan.referent()));
    let mut context =
        Loans { views: &loan_views, cache: &mut loan_cache, places, types, end, work };
    for loan in &loan_views {
        loans.push(context.resolve(*loan, 0)?);
    }
    Ok((loans, loan_types))
}

struct Loans<'c, 'p, 'a, 't> {
    views: &'c [VerifiedLoan<'p, 'a>],
    cache: &'c mut [Option<LoanAddress>],
    places: &'c [Record],
    types: &'c [Shape<'t>],
    end: &'c mut u64,
    work: &'c mut u64,
}
impl Loans<'_, '_, '_, '_> {
    fn resolve(
        &mut self,
        loan: VerifiedLoan<'_, '_>,
        depth: usize,
    ) -> Result<LoanAddress, Diagnostic> {
        take(self.work, 1)?;
        if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
            return Err(budget());
        }
        if let Some(r) = self.cache[loan.id() as usize] {
            return Ok(r);
        }
        let result = if let Some(indexed) = loan.indexed() {
            self.indexed(indexed)?
        } else {
            match loan.origin().ok_or_else(invariant)? {
                VerifiedLoanOrigin::Formal(parameter) => {
                    LoanAddress::Formal { parameter, offset: 0 }
                }
                VerifiedLoanOrigin::Place(place) => {
                    LoanAddress::Frame(*self.places.get(place.id() as usize).ok_or_else(invariant)?)
                }
                VerifiedLoanOrigin::Parent(parent) => self.resolve(
                    *self.views.get(parent.id() as usize).ok_or_else(invariant)?,
                    depth + 1,
                )?,
                VerifiedLoanOrigin::StaticProjection { parent, projection } => {
                    let base = self.resolve(
                        *self.views.get(parent.id() as usize).ok_or_else(invariant)?,
                        depth + 1,
                    )?;
                    let delta = displacement(
                        self.types,
                        raw::ValueType::Stored(parent.referent()),
                        projection,
                        loan.referent(),
                        self.work,
                    )?;
                    match base {
                        LoanAddress::Formal { parameter, offset } => LoanAddress::Formal {
                            parameter,
                            offset: offset.checked_add(delta).ok_or_else(budget)?,
                        },
                        LoanAddress::Frame(record) => LoanAddress::Frame(Record {
                            offset: record.offset.checked_add(delta).ok_or_else(budget)?,
                            ty: raw::ValueType::Stored(loan.referent()),
                        }),
                        LoanAddress::Prepared { slot, offset } => LoanAddress::Prepared {
                            slot,
                            offset: offset.checked_add(delta).ok_or_else(budget)?,
                        },
                    }
                }
            }
        };
        self.cache[loan.id() as usize] = Some(result);
        Ok(result)
    }
    fn indexed(
        &mut self,
        indexed: zryna_ir::generic_v1::execution_v1::VerifiedIndexedLoan<'_, '_>,
    ) -> Result<LoanAddress, Diagnostic> {
        take(self.work, 2048)?;
        let container = shape(self.types, indexed.container_type())?.view;
        if !matches!(
            container.category(),
            zryna_layout::TypeCategory::FixedArray | zryna_layout::TypeCategory::Vec
        ) || container.referenced_type().ok_or_else(invariant)?.index()
            != indexed.loan().referent()
        {
            return Err(invariant());
        }
        *self.end = self.end.checked_add(7).map(|n| n & !7).ok_or_else(budget)?;
        let slot = u32::try_from(*self.end).map_err(|_| budget())?;
        *self.end = self.end.checked_add(8).ok_or_else(budget)?;
        Ok(LoanAddress::Prepared { slot, offset: 0 })
    }
}
