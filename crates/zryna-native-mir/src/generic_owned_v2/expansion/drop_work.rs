//! Independent bounded postorder over sealed stored payloads, never emitted backend output.
use super::{budget, invariant, take};
use zryna_diagnostics::Diagnostic;
use zryna_layout::generic_v1::TypeView;

#[derive(Clone, Copy, Default)]
pub(super) struct DropWork {
    pub units: u64,
    pub contains_nominal: bool,
    pub contains_struct: bool,
}

pub(super) fn derive(types: &[TypeView<'_>], units: &mut u64) -> Result<Vec<DropWork>, Diagnostic> {
    take(units, u64::try_from(types.len()).map_err(|_| budget())?)?;
    let mut cache = Vec::new();
    cache.try_reserve(types.len()).map_err(|_| budget())?;
    cache.resize(types.len(), DropWork::default());
    let mut state = Vec::new();
    state.try_reserve(types.len()).map_err(|_| budget())?;
    state.resize(types.len(), 0u8);
    let mut pending = Vec::new();
    for root in 0..types.len() {
        pending.try_reserve(1).map_err(|_| budget())?;
        pending.push((root, false));
        while let Some((index, exit)) = pending.pop() {
            let ty = types.get(index).copied().ok_or_else(invariant)?;
            if state[index] == 2 {
                continue;
            }
            let structure = matches!(ty.key().first(), Some(0x10 | 0x12));
            let nominal = structure || matches!(ty.key().first(), Some(0x11 | 0x13));
            if structure && !exit {
                // Complete field traversal/retention credit precedes width and destructor fanout.
                take(
                    units,
                    u64::try_from(ty.fields().len())
                        .map_err(|_| budget())?
                        .checked_mul(3)
                        .ok_or_else(budget)?,
                )?;
            }
            if ty.drop_kind() == 0 {
                cache[index] =
                    DropWork { units: 0, contains_nominal: nominal, contains_struct: structure };
                state[index] = 2;
                continue;
            }
            if ty.key().first() == Some(&2) {
                cache[index] =
                    DropWork { units: 6, contains_nominal: false, contains_struct: false };
                state[index] = 2;
                continue;
            }
            if !matches!(ty.key().first(), Some(0x10..=0x15)) {
                return Err(invariant());
            }
            if structure {
                structure_work(ty, index, exit, &mut cache, &mut state, &mut pending)?;
                continue;
            }
            if exit {
                let mut work =
                    DropWork { units: 17, contains_nominal: nominal, contains_struct: false };
                for (_, payload) in ty.variants() {
                    work.units = work.units.checked_add(35).ok_or_else(budget)?;
                    if let Some(payload) = payload {
                        let child = payload.index() as usize;
                        if state.get(child) != Some(&2) {
                            return Err(invariant());
                        }
                        work.units =
                            work.units.checked_add(cache[child].units).ok_or_else(budget)?;
                        work.contains_nominal |= cache[child].contains_nominal;
                        work.contains_struct |= cache[child].contains_struct;
                    }
                }
                cache[index] = work;
                state[index] = 2;
            } else {
                if state[index] != 0 {
                    return Err(invariant());
                }
                // Debit the full known member census before iteration or pending-frame growth.
                let count = ty.variants().len();
                take(units, u64::try_from(count).map_err(|_| budget())?)?;
                pending
                    .try_reserve(count.checked_add(1).ok_or_else(budget)?)
                    .map_err(|_| budget())?;
                state[index] = 1;
                pending.push((index, true));
                for (_, payload) in ty.variants() {
                    if let Some(payload) = payload {
                        let child = payload.index() as usize;
                        match state.get(child) {
                            Some(0) => pending.push((child, false)),
                            Some(2) => {}
                            _ => return Err(invariant()),
                        }
                    }
                }
            }
        }
    }
    Ok(cache)
}

mod structures;
use structures::structure_work;

#[cfg(test)]
#[path = "../struct_tests/layouts.rs"]
mod struct_layout_tests;
