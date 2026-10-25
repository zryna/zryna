//! Closed source-origin `String` effects stop at nonempty physical acquisitions, never counts.
use super::super::{
    Context, Diagnostic, TypeCategory, charge, emission_budget, filled, internal, raw,
};
pub(super) struct Types {
    seen: Vec<bool>,
    queue: Vec<u32>,
}
impl Types {
    pub(super) fn new(context: &Context<'_, '_>) -> Result<Self, Diagnostic> {
        let seen = filled(context, context.types.len(), false)?;
        let mut queue = filled(context, context.types.len(), 0u32)?;
        queue.clear();
        Ok(Self { seen, queue })
    }
    pub(super) fn request(&mut self, context: &Context<'_, '_>, id: u32) -> Result<(), Diagnostic> {
        charge(&context.work, 1)?;
        let seen = self.seen.get_mut(id as usize).ok_or_else(internal)?;
        if !*seen {
            *seen = true;
            self.queue.push(id);
        }
        Ok(())
    }
    pub(super) fn value(
        &mut self,
        context: &Context<'_, '_>,
        ty: raw::ValueType,
    ) -> Result<(), Diagnostic> {
        if let raw::ValueType::Stored(id) = ty {
            self.request(context, id)?;
        }
        Ok(())
    }
    pub(super) fn check(&mut self, context: &Context<'_, '_>) -> Result<(), Diagnostic> {
        while let Some(id) = self.queue.pop() {
            let ty = context.ty(id)?;
            match ty.category() {
                TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
                TypeCategory::Struct => {
                    let fields = ty.fields();
                    charge(&context.work, fields.len())?;
                    for (_, child, _) in fields {
                        self.request(context, child.index())?;
                    }
                }
                TypeCategory::Enum => {
                    let variants = ty.variants();
                    charge(&context.work, variants.len())?;
                    for (_, payload) in variants {
                        if let Some(child) = payload {
                            self.request(context, child.index())?;
                        }
                    }
                }
                TypeCategory::FixedArray => {
                    self.request(context, ty.referenced_type().ok_or_else(internal)?.index())?;
                }
                TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => {
                    return Err(emission_budget());
                }
            }
        }
        Ok(())
    }
}
