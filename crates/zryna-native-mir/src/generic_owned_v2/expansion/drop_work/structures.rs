//! Exact Struct field work: no enum tag dispatch, synthetic blocks, or inferred Copy.
use super::{DropWork, budget, invariant};
use zryna_diagnostics::Diagnostic;
use zryna_layout::generic_v1::TypeView;

pub(super) fn structure_work(
    ty: TypeView<'_>,
    index: usize,
    exit: bool,
    cache: &mut [DropWork],
    state: &mut [u8],
    pending: &mut Vec<(usize, bool)>,
) -> Result<(), Diagnostic> {
    let count = ty.fields().len();
    if exit {
        // One forward visit, one retained range, one reverse visit per emitted field.
        // Actual emitted Cranelift work comes exclusively from each field destructor.
        let mut work = DropWork {
            units: u64::try_from(count).map_err(|_| budget())?.checked_mul(3).ok_or_else(budget)?,
            contains_nominal: true,
            contains_struct: true,
        };
        for (_, field, _) in ty.fields() {
            let child = field.index() as usize;
            if state.get(child) != Some(&2) {
                return Err(invariant());
            }
            work.units = work.units.checked_add(cache[child].units).ok_or_else(budget)?;
        }
        cache[index] = work;
        state[index] = 2;
    } else {
        if state[index] != 0 {
            return Err(invariant());
        }
        pending.try_reserve(count.checked_add(1).ok_or_else(budget)?).map_err(|_| budget())?;
        state[index] = 1;
        pending.push((index, true));
        for (_, field, _) in ty.fields() {
            let child = field.index() as usize;
            match state.get(child) {
                Some(0) => pending.push((child, false)),
                Some(2) => {}
                _ => return Err(invariant()),
            }
        }
    }
    Ok(())
}
