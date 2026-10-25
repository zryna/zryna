//! Counted in-place heapsort: no quadratic source-key insertion path.
use super::{Failure, add, charge, reject};
use std::cmp::Ordering;
pub(super) fn keys(keys: &mut [Vec<u8>], spent: &mut usize) -> Result<(), Failure> {
    for parent in (0..keys.len() / 2).rev() {
        sift(keys, parent, keys.len(), spent)?;
    }
    for end in (1..keys.len()).rev() {
        charge(spent, 1)?;
        keys.swap(0, end);
        sift(keys, 0, end, spent)?;
    }
    for pair in keys.windows(2) {
        if compare(&pair[0], &pair[1], spent)? != Ordering::Less {
            return Err(reject("duplicate source function instance key"));
        }
    }
    Ok(())
}
fn compare(a: &[u8], b: &[u8], spent: &mut usize) -> Result<Ordering, Failure> {
    charge(spent, add(a.len().min(b.len()), 1)?)?;
    Ok(a.cmp(b))
}
fn sift(keys: &mut [Vec<u8>], mut at: usize, end: usize, spent: &mut usize) -> Result<(), Failure> {
    while let Some(left) = at.checked_mul(2).and_then(|n| n.checked_add(1)).filter(|n| *n < end) {
        let mut child = left;
        if left + 1 < end && compare(&keys[left], &keys[left + 1], spent)? == Ordering::Less {
            child += 1;
        }
        if compare(&keys[at], &keys[child], spent)? != Ordering::Less {
            break;
        }
        charge(spent, 1)?;
        keys.swap(at, child);
        at = child;
    }
    Ok(())
}
