//! Actual stored members are resolved in their declaring module and exact environment.
use super::syntax::Resolved;
use super::{Budget, Engine, Failure, Frame, reject};
use zryna_syntax::v5::RawDataDeclarationKind;

impl Engine<'_, '_> {
    pub(super) fn member_count(&self, id: u32, credit: &mut Budget) -> Result<usize, Failure> {
        credit.nominal(1)?;
        let key = self.terms[id as usize].key;
        if !matches!(key[0], 0x10..=0x13) {
            return Ok(if key[0] >= 0x14 && key[0] <= 0x15 { key[3] as usize } else { 0 });
        }
        let original = self
            .originals
            .units
            .get(key[1] as usize)
            .and_then(|u| u.data_declarations.get(key[2] as usize))
            .ok_or_else(|| reject("contextual term names an unknown original"))?;
        let arity = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
        let (tag, count) = match &original.kind {
            RawDataDeclarationKind::Struct { fields, .. } => {
                (if arity == 0 { 0x10 } else { 0x12 }, fields.len())
            }
            RawDataDeclarationKind::Enum { variants, .. } => {
                (if arity == 0 { 0x11 } else { 0x13 }, variants.len())
            }
        };
        if arity != key[3] as usize || arity > 2 || tag != key[0] {
            return Err(reject("contextual nominal kind/arity differs from its original"));
        }
        if count == 0 || count > zryna_layout::MAX_MEMBERS_PER_DECLARATION {
            return Err(reject("contextual original requires one to 1024 members"));
        }
        Ok(count)
    }
    pub(super) fn member(
        &mut self,
        id: u32,
        cursor: usize,
        credit: &mut Budget,
    ) -> Result<Option<u32>, Failure> {
        credit.nominal(1)?;
        let key = self.terms[id as usize].key;
        if matches!(key[0], 0x14 | 0x15) {
            return Ok(Some(key[4 + cursor]));
        }
        let original = &self.originals.units[key[1] as usize].data_declarations[key[2] as usize];
        let occurrence = match &original.kind {
            RawDataDeclarationKind::Struct { fields, .. } => Some(fields[cursor].type_syntax),
            RawDataDeclarationKind::Enum { variants, .. } => variants[cursor].payload_type,
        };
        let Some(occurrence) = occurrence else { return Ok(None) };
        let frame = Frame { kind: 1, module: key[1], index: key[2] };
        let value = self.syntax(frame, &key[4..4 + key[3] as usize], occurrence, credit)?;
        let Resolved::Stored(value) = value else {
            return Err(reject("nominal member is not a stored source value"));
        };
        Ok(Some(value))
    }
}
