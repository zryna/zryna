//! Private v4 source authority, scoped to immutable Originals and an exact opaque owner.
use super::{Budget, Closed};
use crate::generic_v1::{
    Failure, budget, reject, reserve, source::Originals, source_types::Resolver,
};
use zryna_syntax::v5::RawTypeParameterList;
pub(super) mod clone_v5;
mod encoding;
mod members;
mod memo;
pub(super) mod queries;
mod syntax;
mod terms;
mod walk;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Frame {
    pub kind: u32,
    pub module: u32,
    pub index: u32,
}
impl Frame {
    fn parameters<'a>(
        &self,
        originals: &Originals<'a>,
    ) -> Result<Option<&'a RawTypeParameterList>, Failure> {
        let unit = originals.units.get(self.module as usize).ok_or(Failure::InternalFailure)?;
        match self.kind {
            0 => Ok(unit
                .functions
                .get(self.index as usize)
                .ok_or(Failure::InternalFailure)?
                .type_parameters
                .as_ref()),
            1 => Ok(unit
                .data_declarations
                .get(self.index as usize)
                .ok_or(Failure::InternalFailure)?
                .type_parameters
                .as_ref()),
            _ => Err(Failure::InternalFailure),
        }
    }
}
struct Engine<'a, 'b> {
    originals: &'a Originals<'b>,
    terms: Vec<terms::Term>,
    memo: memo::Memo,
    expanded: usize,
}
impl<'a, 'b> Engine<'a, 'b> {
    fn new(originals: &'a Originals<'b>) -> Self {
        Self { originals, terms: Vec::new(), memo: memo::Memo::default(), expanded: 0 }
    }
    fn arguments(
        &mut self,
        resolver: &Resolver<'_, '_>,
        owner: Frame,
        symbolic: bool,
        credit: &mut Budget,
    ) -> Result<([u32; 2], usize), Failure> {
        if resolver.arguments.len() > 2 {
            return Err(reject("contextual source arity exceeds two"));
        }
        let mut arguments = [0; 2];
        for (slot, key) in arguments.iter_mut().zip(&resolver.arguments) {
            *slot = self.decode(key, owner, symbolic, credit)?;
        }
        Ok((arguments, resolver.arguments.len()))
    }
    fn closed(&mut self, value: syntax::Resolved, credit: &mut Budget) -> Result<Closed, Failure> {
        match value {
            syntax::Resolved::Unit => Ok(Closed::Unit),
            syntax::Resolved::Stored(id) => {
                self.prove(id, credit)?;
                Ok(Closed::Stored(self.encode(id, credit)?))
            }
            syntax::Resolved::Borrow(id, exclusive) => {
                self.prove(id, credit)?;
                Ok(Closed::Borrow(self.encode(id, credit)?, exclusive))
            }
        }
    }
}
struct Cached {
    key: Vec<u8>,
    affine: bool,
    work: usize,
}
pub(super) struct Facts<'a, 'b> {
    originals: &'a Originals<'b>,
    owner: Frame,
    symbolic: bool,
    last: Option<Cached>,
}
impl<'a, 'b> Facts<'a, 'b> {
    pub(super) fn new(originals: &'a Originals<'b>, owner: Frame, symbolic: bool) -> Self {
        Self { originals, owner, symbolic, last: None }
    }
    pub(super) fn affine(&mut self, ty: &Closed, credit: &mut Budget) -> Result<bool, Failure> {
        let (Closed::Stored(key) | Closed::Borrow(key, _)) = ty else { return Ok(false) };
        if key.len() > 4096 {
            return Err(budget("contextual root key exceeds 4096 bytes"));
        }
        // The only externally visible debit of a failed miss is this completed lookup.
        credit.nominal(add(1, key.len())?)?;
        if let Some(last) = self.last.as_ref().filter(|last| last.key == *key) {
            credit.nominal(last.work)?;
            return Ok(matches!(ty, Closed::Stored(_)) && last.affine);
        }
        let mut checked = *credit;
        let mut engine = Engine::new(self.originals);
        let id = engine.decode(key, self.owner, self.symbolic, &mut checked)?;
        let fact = engine.prove(id, &mut checked)?;
        checked.nominal(key.len())?;
        let mut retained = reserve(key.len())?;
        retained.extend_from_slice(key);
        let work = checked
            .nominal_used()
            .checked_sub(credit.nominal_used())
            .ok_or(Failure::InternalFailure)?;
        self.last = Some(Cached { key: retained, affine: fact.affine, work });
        *credit = checked;
        Ok(matches!(ty, Closed::Stored(_)) && fact.affine)
    }
    pub(super) fn resolve(
        &self,
        resolver: &Resolver<'_, '_>,
        frame: Frame,
        at: u32,
        credit: &mut Budget,
    ) -> Result<Closed, Failure> {
        let mut checked = *credit;
        let mut engine = Engine::new(self.originals);
        let (arguments, count) =
            engine.arguments(resolver, self.owner, self.symbolic, &mut checked)?;
        let value = engine.syntax(frame, &arguments[..count], at, &mut checked)?;
        let result = engine.closed(value, &mut checked)?;
        *credit = checked;
        Ok(result)
    }
}
pub(super) fn declarations(originals: &Originals<'_>) -> Result<usize, Failure> {
    let mut credit = Budget::default();
    let mut total = 0;
    for unit in originals.units {
        for (index, original) in unit.data_declarations.iter().enumerate() {
            let mut engine = Engine::new(originals);
            let frame = Frame {
                kind: 1,
                module: unit.id,
                index: u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
            };
            let count = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
            if count > 2 {
                return Err(reject("contextual declaration arity exceeds two"));
            }
            let mut arguments = [0; 2];
            for (slot, argument) in arguments[..count].iter_mut().enumerate() {
                *argument = engine.opaque(
                    frame,
                    u32::try_from(slot).map_err(|_| Failure::InternalFailure)?,
                    &mut credit,
                )?;
            }
            let tag = match (&original.kind, count) {
                (zryna_syntax::v5::RawDataDeclarationKind::Struct { .. }, 0) => 0x10,
                (zryna_syntax::v5::RawDataDeclarationKind::Enum { .. }, 0) => 0x11,
                (zryna_syntax::v5::RawDataDeclarationKind::Struct { .. }, _) => 0x12,
                (zryna_syntax::v5::RawDataDeclarationKind::Enum { .. }, _) => 0x13,
            };
            let id =
                engine.application(tag, unit.id, frame.index, &arguments[..count], &mut credit)?;
            total = add(total, engine.member_count(id, &mut credit)?)?;
            if total > zryna_layout::MAX_MEMBERS {
                return Err(budget("contextual original member count exceeds 65536"));
            }
            engine.prove(id, &mut credit)?;
        }
    }
    Ok(credit.nominal_used())
}
fn add(a: usize, b: usize) -> Result<usize, Failure> {
    a.checked_add(b).ok_or_else(|| budget("contextual source work overflow"))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod contextual_extra;
