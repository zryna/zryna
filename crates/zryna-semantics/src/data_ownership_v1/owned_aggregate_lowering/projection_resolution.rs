use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawDataDeclarationKind, RawExpressionKind};

use super::super::layout_graph::{Decl, semantic_type};
use super::super::type_model::{Binding, OwnedAggregatePlace, Ty};
use super::super::{Errors, SemanticInput};
use super::projection_topology::{ProjectionDescriptor, ProjectionTopology, project};
use crate::data_ownership_v1::diagnostics::span;

pub(super) struct ProjectionResolver<'a, 'f, 'b, 'e> {
    pub(super) input: SemanticInput<'a>,
    pub(super) file: &'a syntax::SourceUnit,
    pub(super) function: &'f syntax::RawFunctionSyntax,
    pub(super) module: usize,
    pub(super) declarations: &'a [Decl],
    pub(super) graph: &'a raw_layout::Graph,
    pub(super) node_types: &'a [Option<Ty>],
    pub(super) layouts: &'a layout::VerifiedLayouts,
    pub(super) bindings: &'b BTreeMap<String, Binding>,
    pub(super) errors: &'e mut Errors<'a>,
}

impl ProjectionResolver<'_, '_, '_, '_> {
    fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

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

    pub(super) fn resolve(
        &mut self,
        id: u32,
        topology: &mut impl ProjectionTopology,
    ) -> Option<OwnedAggregatePlace> {
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
                let base = self.resolve(base, topology)?;
                let (ordinal, ty) = self.field_projection_type(base.ty, &field)?;
                let key = (base.place.0, 0, ordinal);
                let place = project(
                    topology,
                    ProjectionDescriptor {
                        ty,
                        at,
                        key,
                        kind: raw::PlaceKind::StructField { base: base.place, ordinal },
                    },
                    self.errors,
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
                let base = self.resolve(base, topology)?;
                let (index, ty) = self.constant_projection_type(base.ty, index)?;
                let key = (base.place.0, 1, index);
                let place = project(
                    topology,
                    ProjectionDescriptor {
                        ty,
                        at,
                        key,
                        kind: raw::PlaceKind::FixedArrayConstant { base: base.place, index },
                    },
                    self.errors,
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
