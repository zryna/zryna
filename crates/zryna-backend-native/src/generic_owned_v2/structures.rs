//! Exact ordinal field lane ranges; physical byte padding never becomes private carrier lanes.
use super::{body::get, effects::value_type, invariant};
use cranelift_codegen::ir::Value;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::raw;
use zryna_native_mir::generic_owned_v2::VerifiedProgram;

pub(super) struct Field {
    pub ty: raw::Type,
    pub begin: usize,
    pub end: usize,
}
pub(super) fn fields(p: &VerifiedProgram<'_, '_>, index: u32) -> Result<Vec<Field>, Diagnostic> {
    let view = p.stored_type(index).ok_or_else(invariant)?;
    if !matches!(view.key().first(), Some(0x10 | 0x12)) {
        return Err(invariant());
    }
    let mut fields = Vec::new();
    fields.try_reserve(view.fields().len()).map_err(|_| invariant())?;
    let mut begin = 0usize;
    for (ordinal, field, _) in view.fields() {
        if usize::try_from(ordinal).map_err(|_| invariant())? != fields.len() {
            return Err(invariant());
        }
        let ty = raw::Type::Stored(field.index());
        let width = usize::try_from(p.width(ty)?).map_err(|_| invariant())?;
        let end = begin.checked_add(width).ok_or_else(invariant)?;
        fields.push(Field { ty, begin, end });
        begin = end;
    }
    if begin != usize::try_from(p.width(raw::Type::Stored(index))?).map_err(|_| invariant())? {
        return Err(invariant());
    }
    Ok(fields)
}
pub(super) fn construct(
    p: &VerifiedProgram<'_, '_>,
    values: &[Option<Vec<Value>>],
    index: u32,
    operands: &[u32],
) -> Result<Vec<Value>, Diagnostic> {
    let fields = fields(p, index)?;
    if fields.len() != operands.len() {
        return Err(invariant());
    }
    let width = usize::try_from(p.width(raw::Type::Stored(index))?).map_err(|_| invariant())?;
    let mut lanes = Vec::new();
    lanes.try_reserve(width).map_err(|_| invariant())?;
    for (field, operand) in fields.iter().zip(operands) {
        let source = get(values, *operand)?;
        if source.len() != field.end - field.begin {
            return Err(invariant());
        }
        lanes.extend_from_slice(source);
    }
    if lanes.len() != width {
        return Err(invariant());
    }
    Ok(lanes)
}
pub(super) fn copy_field(
    p: &VerifiedProgram<'_, '_>,
    function: &raw::Function,
    values: &[Option<Vec<Value>>],
    value: u32,
    ordinal: u32,
) -> Result<Vec<Value>, Diagnostic> {
    let raw::Type::Stored(index) = value_type(function, value)? else {
        return Err(invariant());
    };
    let fields = fields(p, index)?;
    let field =
        fields.get(usize::try_from(ordinal).map_err(|_| invariant())?).ok_or_else(invariant)?;
    let raw::Type::Stored(ty) = field.ty else {
        return Err(invariant());
    };
    if p.stored_type(ty).ok_or_else(invariant)?.drop_kind() != 0 {
        return Err(invariant());
    }
    let source = get(values, value)?.get(field.begin..field.end).ok_or_else(invariant)?;
    let mut lanes = Vec::new();
    lanes.try_reserve(source.len()).map_err(|_| invariant())?;
    lanes.extend_from_slice(source);
    Ok(lanes)
}
