//! Only proved in-range fixed constants become static places; other indexes keep checked access.
use super::{Binding, Body, Expr, Failure, Kind, UntrustedSpan, charge, raw};
impl<'s> Body<'_, 's> {
    pub(super) fn static_index_binding(
        &mut self,
        base: &Binding<'s>,
        index: u32,
        span: UntrustedSpan,
    ) -> Result<Option<Binding<'s>>, Failure> {
        charge(self.spent, 1)?;
        let Some(Kind::FixedArray { length }) = self.world.kind(base.ty) else { return Ok(None) };
        let Expr::I32Literal { spelling } = &self.original.body.expressions[index as usize].kind
        else {
            return Ok(None);
        };
        charge(self.spent, spelling.len())?;
        let Ok(index) = spelling.parse::<u32>() else { return Ok(None) };
        if index >= length {
            return Ok(None);
        }
        let ty = self
            .world
            .arguments(base.ty)
            .and_then(|a| a.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        // Existing Copy indexing keeps its checked IndexCopy encoding. This new
        // route supplies only the static affine move that IndexCopy cannot own.
        if self.copy(ty)? {
            return Ok(None);
        }
        let mut binding = base.clone();
        binding.source = self.project_source(
            base,
            ty,
            raw::StaticProjection::FixedArrayConstant(index),
            span,
            span,
        )?;
        binding.ty = ty;
        Ok(Some(binding))
    }
}
