use super::super::{
    RawDataDeclarationKind, RawExpressionKind, Span, Ty, TypeCategory, semantic_type, span,
};
use super::FunctionLowerer;

impl FunctionLowerer<'_, '_, '_> {
    pub(in crate::data_ownership_v1) fn primitive(&self, category: TypeCategory) -> Option<Ty> {
        self.node_types.iter().flatten().find(|ty| ty.category == category).copied()
    }
    pub(in crate::data_ownership_v1) fn decl_ty(&self, name: &str) -> Option<Ty> {
        let decl = self.declarations.iter().find(|d| d.module == self.module && d.name == name)?;
        self.node_types[usize::try_from(decl.node.0).ok()?]
    }
    pub(in crate::data_ownership_v1) fn field(
        &mut self,
        base: Ty,
        name: &str,
        use_span: Span,
    ) -> Option<(u32, Ty)> {
        let nominal = self.layouts.type_by_id(base.layout)?.nominal_identity()?;
        let decl = self.declarations.iter().find(|d| {
            (u32::try_from(d.module).ok(), u32::try_from(d.declaration).ok())
                == (Some(nominal.0), Some(nominal.1))
        })?;
        let raw_decl =
            &self.input.syntax().files()[decl.module].data_declarations()[decl.declaration];
        let RawDataDeclarationKind::Struct { fields, .. } = &raw_decl.kind else {
            self.errors.at(
                "ZRYNA-M3006",
                decl.span,
                "field access requires a struct",
                "project fields only from a struct place",
            );
            return None;
        };
        fields
            .iter()
            .enumerate()
            .find(|(_, f)| f.name.text == name)
            .and_then(|(ordinal, f)| {
                semantic_type(
                    self.file,
                    f.type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )
                .map(|ty| (u32::try_from(ordinal).unwrap_or(u32::MAX), ty))
            })
            .or_else(|| {
                self.errors.at(
                    "ZRYNA-M3006",
                    use_span,
                    format!("struct '{}' has no field '{name}'", decl.name),
                    "use one exact declared field name",
                );
                None
            })
    }
    pub(in crate::data_ownership_v1) fn constant_index(
        &mut self,
        base: Ty,
        index_expr: u32,
    ) -> Option<(u32, Ty)> {
        let expr =
            usize::try_from(index_expr).ok().and_then(|i| self.function.body.expressions.get(i))?;
        let RawExpressionKind::I32Literal { spelling } = &expr.kind else {
            self.errors.at(
                "ZRYNA-M3006",
                span(self.input.sources(), expr.span),
                "fixed-array indices must be compile-time i32 literals",
                "use a nonnegative literal within the fixed-array length",
            );
            return None;
        };
        let index = spelling.parse::<u32>().ok().or_else(|| {
            self.errors.at(
                "ZRYNA-M3006",
                span(self.input.sources(), expr.span),
                "fixed-array index is negative or outside u32",
                "use a nonnegative constant index",
            );
            None
        })?;
        let record = self.layouts.type_by_id(base.layout)?;
        let length = record.array_length()?;
        if u64::from(index) >= length {
            self.errors.at(
                "ZRYNA-M3006",
                span(self.input.sources(), expr.span),
                format!("fixed-array index {index} is outside length {length}"),
                "use an index less than the exact fixed-array length",
            );
            return None;
        }
        let element = record.referenced_type()?;
        let ty = self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        Some((index, ty))
    }
}
