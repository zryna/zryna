//! Exact loan-cell inventory: pointer cells have no stored Borrow `TypeId` or local referent Place.
use super::{Diagnostic, VerifiedFunction, add, align, budget, charge, raw, retained};
pub(super) fn inventory(
    f: VerifiedFunction<'_, '_>,
    cursor: &mut u32,
    work: &mut usize,
) -> Result<Vec<raw::LoanCell>, Diagnostic> {
    charge(work, f.loans().len())?;
    retained::<raw::LoanCell>(work, f.loans().len())?;
    let mut cells = Vec::new();
    #[cfg(test)]
    crate::generic_execution_v1::work::reserve_seen(f.loans().len());
    cells.try_reserve_exact(f.loans().len()).map_err(|_| budget())?;
    for loan in f.loans() {
        if loan.origin().is_none() {
            // One extra fixed metadata query, paid before inspecting the indexed view.
            charge(work, 1)?;
            if loan.indexed().is_none() {
                return Err(super::invariant());
            }
        }
        *cursor = align(*cursor, 4)?;
        cells.push(raw::LoanCell {
            referent: loan.referent(),
            access: loan.access(),
            offset: *cursor,
        });
        *cursor = add(*cursor, 4)?;
    }
    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn genuine_formal_loan_cell_inventory_prepays_before_its_first_reserve() {
        crate::generic_execution_v1::tests::loan_program("formal-copy", |program, _| {
            let function = program
                .functions()
                .find(|f| f.loans().len() != 0)
                .expect("actual formal/lexical loans");
            let cost = function.loans().len() * (1 + size_of::<raw::LoanCell>());
            let mut spent = crate::generic_execution_v1::MAX_BYTES - cost + 1;
            let mut cursor = 8;
            crate::generic_execution_v1::work::reset_seen();
            assert!(inventory(function, &mut cursor, &mut spent).is_err());
            assert_eq!(crate::generic_execution_v1::work::seen(), 0);
            assert_eq!(cursor, 8);
            let mut spent = crate::generic_execution_v1::MAX_BYTES - cost;
            let cells = inventory(function, &mut cursor, &mut spent).expect("exact metadata debit");
            assert_eq!(spent, crate::generic_execution_v1::MAX_BYTES);
            assert_eq!(cells.len(), function.loans().len());
        });
    }
}
