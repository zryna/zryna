//! Static source paths retain exact original field/constant-index identity and root mutability.
use super::{Binding, Body, Expr, Failure, Kind, UntrustedSpan, charge, raw, reject};
use zryna_syntax::v4::RawIdentifierSyntax;
impl<'s> Body<'_, 's> {
    pub(super) fn source_place(&mut self, at: u32, depth: usize) -> Result<Binding<'s>, Failure> {
        self.source_place_in(at, depth, Some(self.original.body.expressions[at as usize].span))
    }
    pub(super) fn source_place_in(
        &mut self,
        at: u32,
        depth: usize,
        region: Option<UntrustedSpan>,
    ) -> Result<Binding<'s>, Failure> {
        self.source_place_mode(at, depth, region, (raw::Access::Shared, raw::RegionRole::Lexical))
    }
    pub(super) fn source_place_mode(
        &mut self,
        at: u32,
        depth: usize,
        region: Option<UntrustedSpan>,
        mode: (raw::Access, raw::RegionRole),
    ) -> Result<Binding<'s>, Failure> {
        if !u32::try_from(depth).is_ok_and(|d| d <= zryna_syntax::v4::MAX_NESTING_DEPTH) {
            return Err(super::exhausted("source static place exceeds inherited nesting depth"));
        }
        charge(self.spent, 1)?;
        let e = &self.original.body.expressions[at as usize];
        match &e.kind {
            Expr::Reference { .. } => self.reference(at),
            Expr::FieldAccess { base, field, .. } => {
                self.source_field(*base, field, e.span, depth, region, mode)
            }
            Expr::Index { base, index, .. } => {
                self.source_index(*base, *index, e.span, depth, region, mode)
            }
            _ => Err(self.error(e.span, "ZRYNA-M7006", "operation requires an exact source place")),
        }
    }
    fn source_field(
        &mut self,
        base: u32,
        field: &RawIdentifierSyntax,
        span: UntrustedSpan,
        depth: usize,
        region: Option<UntrustedSpan>,
        mode: (raw::Access, raw::RegionRole),
    ) -> Result<Binding<'s>, Failure> {
        let mut b = self.source_place_mode(base, depth + 1, region, mode)?;
        let Kind::Struct { module, declaration, .. } =
            self.world.kind(b.ty).ok_or(Failure::InternalFailure)?
        else {
            return Err(reject("static source field requires exact original struct"));
        };
        let original =
            &self.originals.units[module as usize].data_declarations[declaration as usize];
        let zryna_syntax::v5::RawDataDeclarationKind::Struct { fields, .. } = &original.kind else {
            return Err(Failure::InternalFailure);
        };
        let mut ordinal = None;
        for (i, member) in fields.iter().enumerate() {
            charge(self.spent, member.name.text.len().min(field.text.len()) + 1)?;
            if member.name.text == field.text {
                ordinal = Some(i);
                break;
            }
        }
        let ordinal = ordinal
            .ok_or_else(|| self.error(span, "ZRYNA-M7006", "source static field is absent"))?;
        let ty = self
            .world
            .members(b.ty)
            .and_then(|m| m.get(ordinal))
            .copied()
            .flatten()
            .ok_or(Failure::InternalFailure)?;
        b.source = self.project_source(
            &b,
            ty,
            raw::StaticProjection::StructField(
                u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
            ),
            region.unwrap_or(span),
            span,
        )?;
        if let raw::PlaceOrLoan::Loan(id) = b.source {
            self.output.loans[id as usize].region.role = mode.1;
        }
        b.ty = ty;
        Ok(b)
    }
    fn source_index(
        &mut self,
        base: u32,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
        region: Option<UntrustedSpan>,
        mode: (raw::Access, raw::RegionRole),
    ) -> Result<Binding<'s>, Failure> {
        let b = self.source_place_mode(base, depth + 1, region, mode)?;
        self.indexed_source(b, index, span, depth, region.unwrap_or(span), mode)
    }
    pub(super) fn available_target(
        &mut self,
        b: &Binding<'_>,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        match b.source {
            raw::PlaceOrLoan::Place(id) => self.complete_place(id, span),
            raw::PlaceOrLoan::Loan(id) => self.complete_loan_source(id, span),
        }
    }
}
