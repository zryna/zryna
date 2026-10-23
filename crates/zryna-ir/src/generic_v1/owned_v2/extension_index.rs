//! Private typed-v4 result index, prepaid on the actual ownership replay ledger.
use super::{Failure, graph, plan, raw};
use crate::generic_v1::{budget, reject, reserve};

const ABSENT: u32 = u32::MAX;

#[derive(Debug)]
pub(super) struct ExtensionIndex {
    rows: Vec<Vec<u32>>,
}

fn dimensions(function: &graph::Function) -> Result<usize, Failure> {
    function.blocks.iter().try_fold(0usize, |count, block| {
        count
            .checked_add(block.parameters.len())
            .and_then(|count| count.checked_add(block.instructions.len()))
            .ok_or_else(|| budget("owned extension definition census overflow"))
    })
}

/// Debit all dimensions before any table reserve. Raw producers use the same debit
/// without allocating an index they cannot retain as executable authority.
pub(super) fn preflight(
    program: &graph::Program,
    extensions: &[Vec<raw::Extension>],
    used: &mut usize,
) -> Result<(), Failure> {
    if extensions.len() != program.functions.len() {
        return Err(reject("extension index requires one effect row per function"));
    }
    let mut checked = *used;
    plan::charge(
        &mut checked,
        program
            .functions
            .len()
            .checked_mul(2)
            .ok_or_else(|| budget("owned extension function census overflow"))?,
    )?;
    for (function, effects) in program.functions.iter().zip(extensions) {
        // The first B census is prepaid before observing block lengths.
        plan::charge(&mut checked, function.blocks.len())?;
        let definitions = dimensions(function)?;
        let remaining = function
            .blocks
            .len()
            .checked_mul(2)
            .and_then(|count| definitions.checked_mul(2).and_then(|d| count.checked_add(d)))
            .and_then(|count| count.checked_add(effects.len()))
            .ok_or_else(|| budget("owned extension index credit overflow"))?;
        plan::charge(&mut checked, remaining)?;
    }
    *used = checked;
    Ok(())
}

impl ExtensionIndex {
    pub(super) fn build(
        program: &graph::Program,
        extensions: &[Vec<raw::Extension>],
        used: &mut usize,
    ) -> Result<Self, Failure> {
        #[cfg(test)]
        statistics::begin(*used);
        let mut checked = *used;
        preflight(program, extensions, &mut checked)?;
        let mut rows = table(program.functions.len())?;
        for (function, effects) in program.functions.iter().zip(extensions) {
            // Entire-program debit already covers this second block census, the
            // sentinel initialization, dense-definition pass and each effect fill.
            let count = dimensions(function)?;
            let mut row = table(count)?;
            row.resize(count, ABSENT);
            dense(function)?;
            let mut previous = None;
            for (ordinal, effect) in effects.iter().enumerate() {
                let ordinal = u32::try_from(ordinal)
                    .map_err(|_| budget("owned extension ordinal overflow"))?;
                if ordinal == ABSENT || previous.is_some_and(|id| id >= effect.result) {
                    return Err(reject("extension index effects are duplicate or unordered"));
                }
                let result = usize::try_from(effect.result)
                    .map_err(|_| reject("extension index result is outside the platform"))?;
                let slot = row
                    .get_mut(result)
                    .ok_or_else(|| reject("extension index effect has no dense definition"))?;
                if *slot != ABSENT {
                    return Err(reject("extension index repeats a definition"));
                }
                *slot = ordinal;
                previous = Some(effect.result);
            }
            rows.push(row);
        }
        *used = checked;
        #[cfg(test)]
        statistics::commit(checked);
        Ok(Self { rows })
    }

    pub(super) fn ordinal(&self, function: usize, result: u32) -> Option<u32> {
        let result = usize::try_from(result).ok()?;
        let ordinal = *self.rows.get(function)?.get(result)?;
        (ordinal != ABSENT).then_some(ordinal)
    }
}

fn dense(function: &graph::Function) -> Result<(), Failure> {
    for (expected, definition) in function
        .blocks
        .iter()
        .flat_map(|block| {
            block.parameters.iter().chain(block.instructions.iter().map(|i| &i.result))
        })
        .enumerate()
    {
        if usize::try_from(definition.id).ok() != Some(expected) {
            return Err(reject("extension index definitions are not exact dense IDs"));
        }
    }
    Ok(())
}

fn table<T>(count: usize) -> Result<Vec<T>, Failure> {
    #[cfg(test)]
    statistics::reserve_attempt();
    reserve(count)
}

#[cfg(test)]
pub(super) mod statistics;
#[cfg(test)]
mod tests;
