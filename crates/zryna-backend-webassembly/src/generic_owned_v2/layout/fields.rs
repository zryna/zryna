//! Flat field boundaries come from sealed lane widths, never physical byte offsets.
use super::{Layout, Value};
use crate::generic_owned_v2::{budget, invariant};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::raw;
use zryna_layout::{StorageTarget, TypeCategory};

impl Layout<'_, '_> {
    pub(in crate::generic_owned_v2) fn fields(
        &self,
        ty: raw::Type,
        value: Value,
    ) -> Result<Vec<(raw::Type, Value)>, Diagnostic> {
        let raw::Type::Stored(index) = ty else {
            return Err(invariant());
        };
        let view = self
            .program
            .layouts(StorageTarget::Linear32V1)
            .types()
            .nth(index as usize)
            .ok_or_else(invariant)?;
        if view.category() != TypeCategory::Struct || self.width(ty)? != value.width {
            return Err(invariant());
        }
        // Complete-program preflight charges every retained descriptor before this reserve.
        let mut fields = Vec::new();
        fields.try_reserve(view.fields().len()).map_err(|_| budget())?;
        let mut prefix = 0u32;
        for (_, field, _) in view.fields() {
            let ty = raw::Type::Stored(field.index());
            let width = self.width(ty)?;
            let end =
                prefix.checked_add(width).filter(|n| *n <= value.width).ok_or_else(invariant)?;
            let start = value.start.checked_add(prefix).ok_or_else(budget)?;
            fields.push((ty, Value { start, width }));
            prefix = end;
        }
        if prefix != value.width {
            return Err(invariant());
        }
        Ok(fields)
    }
}
