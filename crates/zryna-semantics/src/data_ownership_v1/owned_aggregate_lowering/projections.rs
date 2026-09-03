use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory};
use zryna_syntax::v4::{self as syntax, RawDataDeclarationKind, RawExpressionKind};

use super::super::layout_graph::semantic_type;
use super::super::span;
use super::super::type_model::{OwnedAggregatePlace, OwnedAggregatePlacePreflight, Ty};
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn field_projection_type(
        &mut self,
        base: Ty,
        name: &syntax::RawIdentifierSyntax,
    ) -> Option<(u32, Ty)> {
        let use_span = span(self.input.sources(), name.span);
        let Some(nominal) =
            self.layouts.type_by_id(base.layout).and_then(layout::VerifiedType::nominal_identity)
        else {
            self.errors.at(
                "ZRYNA-M3006",
                use_span,
                "owned field projection requires an exact struct place",
                "project one declared field from a supported private struct",
            );
            return None;
        };
        let Some(decl) = self.declarations.iter().find(|decl| {
            (u32::try_from(decl.module).ok(), u32::try_from(decl.declaration).ok())
                == (Some(nominal.0), Some(nominal.1))
        }) else {
            self.errors.at(
                "ZRYNA-M3006",
                use_span,
                "owned field projection has no authenticated declaration",
                "project one declared field from a supported private struct",
            );
            return None;
        };
        let RawDataDeclarationKind::Struct { fields, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3006",
                use_span,
                "owned field projection requires a struct, not an enum",
                "project one declared field from a supported private struct",
            );
            return None;
        };
        fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name.text == name.text)
            .and_then(|(ordinal, field)| {
                u32::try_from(ordinal).ok().zip(semantic_type(
                    self.file,
                    field.type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                ))
            })
            .or_else(|| {
                self.errors.at(
                    "ZRYNA-M3006",
                    use_span,
                    format!("struct '{}' has no field '{}'", decl.name, name.text),
                    "use one exact declared field name",
                );
                None
            })
    }

    fn constant_projection_type(&mut self, base: Ty, index_id: u32) -> Option<(u32, Ty)> {
        let expression = self.expression(index_id)?.clone();
        let at = span(self.input.sources(), expression.span);
        if base.category != TypeCategory::FixedArray {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                "owned indexing currently admits only fixed-array projections",
                "use one constant index into a supported private fixed array",
            );
            return None;
        }
        let RawExpressionKind::I32Literal { spelling } = expression.kind else {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                "owned fixed-array indices must be compile-time i32 literals",
                "use a nonnegative literal within the fixed-array length",
            );
            return None;
        };
        let index = spelling.parse::<u32>().ok().or_else(|| {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                "owned fixed-array index is negative or outside u32",
                "use a nonnegative constant index",
            );
            None
        })?;
        let record = self.layouts.type_by_id(base.layout)?;
        let length = record.array_length()?;
        if u64::from(index) >= length {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                format!("owned fixed-array index {index} is outside length {length}"),
                "use an index less than the exact fixed-array length",
            );
            return None;
        }
        let element = record.referenced_type()?;
        let ty = self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        Some((index, ty))
    }

    pub(super) fn projection_expression_type(&self, id: u32) -> Option<Ty> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).map(|binding| binding.ty)
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let base = self.projection_expression_type(*base)?;
                let nominal = self.layouts.type_by_id(base.layout)?.nominal_identity()?;
                let declaration = self.declarations.iter().find(|declaration| {
                    (
                        u32::try_from(declaration.module).ok(),
                        u32::try_from(declaration.declaration).ok(),
                    ) == (Some(nominal.0), Some(nominal.1))
                })?;
                let RawDataDeclarationKind::Struct { fields, .. } =
                    &self.file.data_declarations()[declaration.declaration].kind
                else {
                    return None;
                };
                let ordinal =
                    fields.iter().position(|candidate| candidate.name.text == field.text)?;
                let field_ty = self.layouts.type_by_id(base.layout)?.fields().get(ordinal)?.ty();
                self.node_types.iter().flatten().find(|ty| ty.layout == field_ty).copied()
            }
            RawExpressionKind::Index { base, index, .. } => {
                let base = self.projection_expression_type(*base)?;
                if base.category != TypeCategory::FixedArray {
                    return None;
                }
                let index_expression = self.expression(*index)?;
                let RawExpressionKind::I32Literal { spelling } = &index_expression.kind else {
                    return None;
                };
                let index = spelling.parse::<u32>().ok()?;
                let record = self.layouts.type_by_id(base.layout)?;
                if u64::from(index) >= record.array_length()? {
                    return None;
                }
                let element = record.referenced_type()?;
                self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()
            }
            _ => None,
        }
    }

    pub(in crate::data_ownership_v1) fn owned_place_preflight(
        &self,
        id: u32,
    ) -> Option<OwnedAggregatePlacePreflight> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name } => {
                let binding = self.bindings.get(&name.text)?;
                Some(OwnedAggregatePlacePreflight {
                    place: OwnedAggregatePlace {
                        ty: binding.ty,
                        place: binding.place,
                        root: binding.place,
                        mutable: binding.mutable,
                        is_root: true,
                    },
                    missing: 0,
                    lineage: vec![binding.place],
                })
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let mut base = self.owned_place_preflight(*base)?;
                let nominal = self.layouts.type_by_id(base.place.ty.layout)?.nominal_identity()?;
                let declaration = self.declarations.iter().find(|declaration| {
                    (
                        u32::try_from(declaration.module).ok(),
                        u32::try_from(declaration.declaration).ok(),
                    ) == (Some(nominal.0), Some(nominal.1))
                })?;
                let RawDataDeclarationKind::Struct { fields, .. } =
                    &self.file.data_declarations()[declaration.declaration].kind
                else {
                    return None;
                };
                let ordinal = u32::try_from(
                    fields.iter().position(|candidate| candidate.name.text == field.text)?,
                )
                .ok()?;
                let ty = self.projection_expression_type(id)?;
                let key = (base.place.place.0, 0, ordinal);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let index = self.places.len().checked_add(base.missing)?;
                    base.missing = base.missing.checked_add(1)?;
                    raw::PlaceId(u32::try_from(index).ok()?)
                };
                base.lineage.push(place);
                base.place = OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.place.root,
                    mutable: base.place.mutable,
                    is_root: false,
                };
                Some(base)
            }
            RawExpressionKind::Index { base, index, .. } => {
                let mut base = self.owned_place_preflight(*base)?;
                if base.place.ty.category != TypeCategory::FixedArray {
                    return None;
                }
                let RawExpressionKind::I32Literal { spelling } = &self.expression(*index)?.kind
                else {
                    return None;
                };
                let index = spelling.parse::<u32>().ok()?;
                let record = self.layouts.type_by_id(base.place.ty.layout)?;
                if u64::from(index) >= record.array_length()? {
                    return None;
                }
                let ty = self.projection_expression_type(id)?;
                let key = (base.place.place.0, 1, index);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let place_index = self.places.len().checked_add(base.missing)?;
                    base.missing = base.missing.checked_add(1)?;
                    raw::PlaceId(u32::try_from(place_index).ok()?)
                };
                base.lineage.push(place);
                base.place = OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.place.root,
                    mutable: base.place.mutable,
                    is_root: false,
                };
                Some(base)
            }
            _ => None,
        }
    }

    pub(in crate::data_ownership_v1) fn preflight_projection_available(
        &self,
        place: &OwnedAggregatePlacePreflight,
    ) -> bool {
        self.owners.contains(place.place.root)
            && !self.moved_projections.iter().any(|moved| {
                place.lineage.contains(moved)
                    || (self.places.get(place.place.place.0 as usize).is_some()
                        && self.place_is_at_or_below(*moved, place.place.place))
            })
    }

    pub(super) fn owned_place(&mut self, id: u32) -> Option<OwnedAggregatePlace> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        match expression.kind {
            RawExpressionKind::Reference { name } => self
                .bindings
                .get(&name.text)
                .cloned()
                .map(|binding| OwnedAggregatePlace {
                    ty: binding.ty,
                    place: binding.place,
                    root: binding.place,
                    mutable: binding.mutable,
                    is_root: true,
                })
                .or_else(|| {
                    let wrong_case =
                        self.bindings.keys().any(|key| key.eq_ignore_ascii_case(&name.text));
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        if wrong_case {
                            format!(
                                "aggregate value '{}' has the wrong portable ASCII case",
                                name.text
                            )
                        } else {
                            format!("aggregate value '{}' is not declared", name.text)
                        },
                        "reference one exact preceding local using its declared spelling",
                    );
                    None
                }),
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let base = self.owned_place(base)?;
                let (ordinal, ty) = self.field_projection_type(base.ty, &field)?;
                let key = (base.place.0, 0, ordinal);
                let place = self.push_projection(
                    ty,
                    at,
                    key,
                    raw::PlaceKind::StructField { base: base.place, ordinal },
                )?;
                Some(OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.root,
                    mutable: base.mutable,
                    is_root: false,
                })
            }
            RawExpressionKind::Index { base, index, .. } => {
                let base = self.owned_place(base)?;
                let (index, ty) = self.constant_projection_type(base.ty, index)?;
                let key = (base.place.0, 1, index);
                let place = self.push_projection(
                    ty,
                    at,
                    key,
                    raw::PlaceKind::FixedArrayConstant { base: base.place, index },
                )?;
                Some(OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.root,
                    mutable: base.mutable,
                    is_root: false,
                })
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned projection base is outside the static place checkpoint",
                    "project from a named private Struct or fixed-array local",
                );
                None
            }
        }
    }
}
