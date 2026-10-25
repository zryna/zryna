//! Admission facade keeps the legacy implementation and its exact charges intact.
use super::{Admission, Budget, Closed, contextual_facts, enums, structs, type_facts};
use crate::generic_v1::{Failure, reject, source::Originals, source_types::Resolver};
pub(super) use contextual_facts::Frame;
pub(super) mod comparisons;

pub(super) enum Facts<'a, 'b> {
    Legacy(type_facts::SourceTypeFacts),
    V4(contextual_facts::Facts<'a, 'b>),
    V5 {
        ownership: contextual_facts::Facts<'a, 'b>,
        clone: contextual_facts::clone_v5::CloneFacts<'a, 'b>,
    },
}
impl<'a, 'b> Facts<'a, 'b> {
    pub(super) fn new(
        originals: &'a Originals<'b>,
        admission: Admission,
        frame: Frame,
        symbolic: bool,
    ) -> Self {
        match admission {
            Admission::LegacyOwned => Self::Legacy(type_facts::SourceTypeFacts::new(admission)),
            Admission::StructCloneV5 => Self::V5 {
                ownership: contextual_facts::Facts::new(originals, frame, symbolic),
                clone: contextual_facts::clone_v5::CloneFacts::new(originals, frame, symbolic),
            },
            Admission::StructV4 => {
                Self::V4(contextual_facts::Facts::new(originals, frame, symbolic))
            }
        }
    }
    pub(super) const fn admission(&self) -> Admission {
        match self {
            Self::Legacy(facts) => facts.admission(),
            Self::V4(_) => Admission::StructV4,
            Self::V5 { .. } => Admission::StructCloneV5,
        }
    }
    pub(super) fn clone_available(
        &mut self,
        ty: &Closed,
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        let Self::V5 { clone, .. } = self else {
            return Err(reject("Struct Clone requires private V5 admission"));
        };
        clone.available(ty, credit)
    }
    pub(super) fn affine(
        &mut self,
        originals: &Originals<'_>,
        ty: &Closed,
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        match self {
            Self::Legacy(facts) => facts.affine(originals, ty, credit),
            Self::V4(facts) | Self::V5 { ownership: facts, .. } => facts.affine(ty, credit),
        }
    }
    pub(super) fn key_copy(
        &self,
        ty: &Closed,
        copies: usize,
        credit: &mut Budget,
    ) -> Result<(), Failure> {
        if matches!(self, Self::Legacy(_)) {
            return credit.nominal_key(ty, copies);
        }
        if let Closed::Stored(key) | Closed::Borrow(key, _) = ty {
            credit.nominal(key.len().checked_mul(copies).ok_or(Failure::InternalFailure)?)?;
        }
        Ok(())
    }
    pub(super) fn v4_copy(
        &self,
        ty: &Closed,
        copies: usize,
        credit: &mut Budget,
    ) -> Result<(), Failure> {
        if matches!(self, Self::V4(_) | Self::V5 { .. }) {
            self.key_copy(ty, copies, credit)?;
        }
        Ok(())
    }
    pub(super) fn resolve(
        &self,
        resolver: &Resolver<'_, '_>,
        frame: Frame,
        at: u32,
        symbolic: bool,
        credit: &mut Budget,
    ) -> Result<Closed, Failure> {
        match self {
            Self::Legacy(_) => {
                if symbolic {
                    resolver.resolve_symbolic(at)
                } else {
                    resolver.resolve(at)
                }
            }
            Self::V4(facts) | Self::V5 { ownership: facts, .. } => {
                facts.resolve(resolver, frame, at, credit)
            }
        }
    }
    pub(super) fn enum_description(
        &mut self,
        resolver: &Resolver<'_, 'b>,
        name: &str,
        arguments: &[Vec<u8>],
        symbolic: bool,
        credit: &mut Budget,
    ) -> Result<enums::Description<'b>, Failure> {
        match self {
            Self::Legacy(facts) => {
                facts.description(resolver, name, arguments, credit)?;
                enums::describe(resolver, name, arguments, symbolic)
            }
            Self::V4(facts) | Self::V5 { ownership: facts, .. } => {
                let description = facts.named(resolver.module, name, arguments, credit)?;
                if description.structure {
                    return Err(reject("enum constructor names a struct"));
                }
                Ok(enums::Description { ty: description.ty, variants: description.members })
            }
        }
    }
    pub(super) fn structure(
        &mut self,
        resolver: &Resolver<'_, 'b>,
        name: &str,
        arguments: &[Vec<u8>],
        symbolic: bool,
        credit: &mut Budget,
    ) -> Result<structs::identity::Description<'b>, Failure> {
        match self {
            Self::Legacy(facts) => {
                facts.description(resolver, name, arguments, credit)?;
                structs::identity::describe(
                    resolver,
                    name,
                    arguments,
                    symbolic,
                    Admission::LegacyOwned,
                )
            }
            Self::V4(facts) | Self::V5 { ownership: facts, .. } => {
                let mut checked = *credit;
                let result = structure(
                    facts.named(resolver.module, name, arguments, &mut checked)?,
                    &mut checked,
                )?;
                *credit = checked;
                Ok(result)
            }
        }
    }
    pub(super) fn stored_structure(
        &self,
        resolver: &Resolver<'_, 'b>,
        key: &[u8],
        symbolic: bool,
        credit: &mut Budget,
    ) -> Result<structs::identity::Description<'b>, Failure> {
        match self {
            Self::Legacy(_) => {
                structs::identity::stored(resolver, key, symbolic, Admission::LegacyOwned, credit)
            }
            Self::V4(facts) | Self::V5 { ownership: facts, .. } => {
                let mut checked = *credit;
                let result = structure(facts.stored(key, &mut checked)?, &mut checked)?;
                *credit = checked;
                Ok(result)
            }
        }
    }
}
fn structure<'b>(
    description: contextual_facts::queries::Description<'b>,
    credit: &mut Budget,
) -> Result<structs::identity::Description<'b>, Failure> {
    if !description.structure {
        return Err(reject("Struct operation requires an original Struct declaration"));
    }
    let mut fields = Vec::new();
    for (name, ty) in description.members {
        let ty = ty.ok_or(Failure::InternalFailure)?;
        credit.nominal(1)?;
        fields.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        fields.push((name, ty));
    }
    Ok(structs::identity::Description { ty: description.ty, fields })
}
