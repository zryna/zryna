use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::{RawDataDeclarationKind, RawExpressionKind};

use super::super::type_model::{OwnedAggregatePlace, OwnedAggregatePlacePreflight, Ty};
use super::PrivateOwnedAggregateLowerer;
use super::projection_resolution::ProjectionResolver;
use super::projection_topology::MaterializedProjectionTopology;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
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

    pub(super) fn owned_place_preflight(&self, id: u32) -> Option<OwnedAggregatePlacePreflight> {
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

    pub(super) fn preflight_projection_available(
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
        let reserved_places = self.reserved_constructor_places();
        let mut resolver = ProjectionResolver {
            input: self.input,
            file: self.file,
            function: self.function,
            module: self.module,
            declarations: self.declarations,
            graph: self.graph,
            node_types: self.node_types,
            layouts: self.layouts,
            bindings: &self.bindings,
            errors: self.errors,
        };
        let mut topology = MaterializedProjectionTopology {
            projections: &mut self.projections,
            places: &mut self.places,
            reserved_places,
        };
        resolver.resolve(id, &mut topology)
    }
}
