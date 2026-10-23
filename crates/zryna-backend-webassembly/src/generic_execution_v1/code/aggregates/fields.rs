//! Fallible, prepaid field prefix retention for reverse cleanup emission.
use super::{Diagnostic, Model};
use zryna_layout::generic_v1::TypeId;
pub(super) fn collect(
    m: &Model<'_, '_, '_>,
    ty: u32,
    count: usize,
) -> Result<Vec<(u32, TypeId, u64)>, Diagnostic> {
    m.take(
        count
            .checked_mul(size_of::<(u32, TypeId, u64)>())
            .ok_or_else(crate::generic_execution_v1::budget)?,
    )?;
    m.take(count)?;
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| crate::generic_execution_v1::budget())?;
    result.extend(m.types[ty as usize].fields().take(count));
    Ok(result)
}
