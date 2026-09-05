use std::collections::BTreeMap;

use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{
    self as syntax, RawDataDeclarationKind, RawExpressionKind, RawFieldInitializerKind,
};

use super::SemanticInput;
use super::diagnostics::Errors;
use super::layout_graph::{Decl, semantic_type};
use super::type_model::{
    RootBorrowInitializer, RootBorrowLiteral, RootBorrowPlacePlan, RootBorrowProjection,
    RootBorrowProjectionKey, Ty,
};
use crate::data_ownership_v1::diagnostics::span;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_root_borrow_initializer<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &syntax::RawFunctionSyntax,
    file: &syntax::SourceUnit,
    declarations: &[Decl],
    graph: &raw_layout::Graph,
    node_types: &[Option<Ty>],
    layouts: &layout::VerifiedLayouts,
    expression_id: u32,
    expected: Ty,
    errors: &mut Errors<'a>,
) -> Option<RootBorrowInitializer> {
    let expression = usize::try_from(expression_id)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    let at = span(input.sources(), expression.span);
    match (&expected.category, &expression.kind) {
        (TypeCategory::Bool, RawExpressionKind::BoolLiteral { value }) => {
            Some(RootBorrowInitializer::Literal {
                literal: RootBorrowLiteral::Bool(*value),
                ty: expected,
                at,
            })
        }
        (TypeCategory::I32, RawExpressionKind::I32Literal { spelling }) => {
            let Some(value) = spelling.parse::<i32>().ok() else {
                errors.at(
                    "ZRYNA-M3008",
                    at,
                    format!("integer literal '{spelling}' is outside i32"),
                    "use a decimal i32 literal",
                );
                return None;
            };
            Some(RootBorrowInitializer::Literal {
                literal: RootBorrowLiteral::I32(value),
                ty: expected,
                at,
            })
        }
        (TypeCategory::Struct, RawExpressionKind::StructConstruction { type_name, fields, .. }) => {
            let Some(declaration) = declarations.iter().find(|declaration| {
                declaration.module == module && declaration.name == type_name.text
            }) else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), type_name.span),
                    "projected borrowing requires one exact module-local Copy struct root",
                    "construct the declared Copy struct matching the root type",
                );
                return None;
            };
            let actual = node_types.get(declaration.node.0 as usize).and_then(|ty| *ty)?;
            let RawDataDeclarationKind::Struct { fields: declared, .. } =
                &file.data_declarations()[declaration.declaration].kind
            else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), type_name.span),
                    "projected borrowing does not admit an enum root constructor",
                    "use a Copy struct or fixed-array root with static projections",
                );
                return None;
            };
            if actual != expected || !actual.is_copy() {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), type_name.span),
                    "projected borrowing requires an exact recursively Copy struct root",
                    "remove String, Vec, enum, Shared, Weak, or other owned fields",
                );
                return None;
            }
            let mut supplied = BTreeMap::new();
            for field in fields {
                let (name, value) = match &field.kind {
                    RawFieldInitializerKind::Shorthand { name, value }
                    | RawFieldInitializerKind::Explicit { name, value, .. } => (&name.text, *value),
                };
                if declared.iter().all(|candidate| candidate.name.text != *name)
                    || supplied.insert(name.clone(), (value, field.span)).is_some()
                {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), field.span),
                        "projected-borrow struct initialization has an invalid or duplicate field",
                        "initialize every exact declared field once",
                    );
                    return None;
                }
            }
            let mut planned = Vec::with_capacity(declared.len());
            for field in declared {
                let Some((value, _)) = supplied.get(&field.name.text).copied() else {
                    errors.at(
                        "ZRYNA-M3017",
                        at,
                        "projected-borrow struct initialization omits a declared field",
                        "initialize every exact declared field once",
                    );
                    return None;
                };
                let ty = semantic_type(
                    file,
                    field.type_syntax,
                    module,
                    declarations,
                    graph,
                    node_types,
                    errors,
                )?;
                planned.push(plan_root_borrow_initializer(
                    input,
                    module,
                    function,
                    file,
                    declarations,
                    graph,
                    node_types,
                    layouts,
                    value,
                    ty,
                    errors,
                )?);
            }
            Some(RootBorrowInitializer::Struct { ty: expected, fields: planned, at })
        }
        (
            TypeCategory::FixedArray,
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. },
        ) => {
            let actual =
                semantic_type(file, *type_syntax, module, declarations, graph, node_types, errors)?;
            let record = layouts.type_by_id(expected.layout)?;
            let length = usize::try_from(record.array_length()?).ok()?;
            if actual != expected || !actual.is_copy() || elements.len() != length {
                errors.at(
                    "ZRYNA-M3017",
                    at,
                    "projected borrowing requires one exact recursively Copy fixed-array root",
                    "construct the exact fixed-array length from recursively Copy elements",
                );
                return None;
            }
            let element_layout = record.referenced_type()?;
            let element =
                node_types.iter().flatten().find(|ty| ty.layout == element_layout).copied()?;
            let planned = elements
                .iter()
                .map(|element_id| {
                    plan_root_borrow_initializer(
                        input,
                        module,
                        function,
                        file,
                        declarations,
                        graph,
                        node_types,
                        layouts,
                        *element_id,
                        element,
                        errors,
                    )
                })
                .collect::<Option<Vec<_>>>()?;
            Some(RootBorrowInitializer::FixedArray { ty: expected, elements: planned, at })
        }
        _ => {
            errors.at(
                "ZRYNA-M3017",
                at,
                "projected borrowing requires a literal recursively Copy root initializer",
                "initialize bool, i32, Copy struct, or Copy fixed-array storage directly",
            );
            None
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_root_borrow_place<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &syntax::RawFunctionSyntax,
    file: &syntax::SourceUnit,
    declarations: &[Decl],
    graph: &raw_layout::Graph,
    node_types: &[Option<Ty>],
    layouts: &layout::VerifiedLayouts,
    expression_id: u32,
    root_name: &str,
    root_ty: Ty,
    errors: &mut Errors<'a>,
) -> Option<RootBorrowPlacePlan> {
    let expression = usize::try_from(expression_id)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    match &expression.kind {
        RawExpressionKind::Reference { name } if name.text == root_name => {
            Some(RootBorrowPlacePlan { ty: root_ty, projections: Vec::new() })
        }
        RawExpressionKind::Reference { name } => {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), name.span),
                "borrow alias does not resolve to the projected root",
                "borrow the exact root, one static projection, or a preceding shared alias",
            );
            None
        }
        RawExpressionKind::FieldAccess { base, field, .. } => {
            let mut place = plan_root_borrow_place(
                input,
                module,
                function,
                file,
                declarations,
                graph,
                node_types,
                layouts,
                *base,
                root_name,
                root_ty,
                errors,
            )?;
            if place.ty.category == TypeCategory::Enum {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), expression.span),
                    "enum payload borrowing conservatively overlaps the complete root and is unavailable",
                    "borrow only a static Struct field or constant fixed-array element",
                );
                return None;
            }
            let Some(nominal) = layouts
                .type_by_id(place.ty.layout)
                .and_then(layout::VerifiedType::nominal_identity)
            else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), expression.span),
                    "borrow field projection does not have a Struct base",
                    "project one exact declared field from the Copy root",
                );
                return None;
            };
            let Some(declaration) = declarations.iter().find(|declaration| {
                (
                    u32::try_from(declaration.module).ok(),
                    u32::try_from(declaration.declaration).ok(),
                ) == (Some(nominal.0), Some(nominal.1))
            }) else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), field.span),
                    "borrow field projection has no authenticated declaration",
                    "project one exact declared field from the Copy root",
                );
                return None;
            };
            let RawDataDeclarationKind::Struct { fields, .. } =
                &file.data_declarations()[declaration.declaration].kind
            else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), expression.span),
                    "enum payload borrowing conservatively overlaps the complete root and is unavailable",
                    "borrow only a static Struct field or constant fixed-array element",
                );
                return None;
            };
            let Some((ordinal, declared)) =
                fields.iter().enumerate().find(|(_, candidate)| candidate.name.text == field.text)
            else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), field.span),
                    format!(
                        "struct '{}' has no borrowable field '{}'",
                        declaration.name, field.text
                    ),
                    "use one exact declared field name",
                );
                return None;
            };
            let ty = semantic_type(
                file,
                declared.type_syntax,
                module,
                declarations,
                graph,
                node_types,
                errors,
            )?;
            let ordinal = u32::try_from(ordinal).ok()?;
            place.projections.push(RootBorrowProjection {
                key: RootBorrowProjectionKey::StructField(ordinal),
                ty,
                at: span(input.sources(), expression.span),
            });
            place.ty = ty;
            Some(place)
        }
        RawExpressionKind::Index { base, index, .. } => {
            let mut place = plan_root_borrow_place(
                input,
                module,
                function,
                file,
                declarations,
                graph,
                node_types,
                layouts,
                *base,
                root_name,
                root_ty,
                errors,
            )?;
            if place.ty.category == TypeCategory::Vec {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), expression.span),
                    "Vec element borrowing conservatively overlaps the complete root and is unavailable",
                    "borrow only a static Struct field or constant fixed-array element",
                );
                return None;
            }
            if place.ty.category != TypeCategory::FixedArray {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), expression.span),
                    "borrow index projection does not have a fixed-array base",
                    "index one exact fixed array with an in-range constant",
                );
                return None;
            }
            let index_expression = usize::try_from(*index)
                .ok()
                .and_then(|index| function.body.expressions.get(index))?;
            let RawExpressionKind::I32Literal { spelling } = &index_expression.kind else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), index_expression.span),
                    "dynamic fixed-array borrowing conservatively overlaps the complete root and is unavailable",
                    "use one in-range nonnegative i32 literal index",
                );
                return None;
            };
            let Some(index) = spelling.parse::<u32>().ok() else {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), index_expression.span),
                    "borrow fixed-array index is negative or outside u32",
                    "use one in-range nonnegative i32 literal index",
                );
                return None;
            };
            let record = layouts.type_by_id(place.ty.layout)?;
            let length = record.array_length()?;
            if u64::from(index) >= length {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), index_expression.span),
                    format!("borrow fixed-array index {index} is outside length {length}"),
                    "use one constant index less than the exact fixed-array length",
                );
                return None;
            }
            let element_layout = record.referenced_type()?;
            let ty = node_types.iter().flatten().find(|ty| ty.layout == element_layout).copied()?;
            place.projections.push(RootBorrowProjection {
                key: RootBorrowProjectionKey::FixedArrayConstant(index),
                ty,
                at: span(input.sources(), expression.span),
            });
            place.ty = ty;
            Some(place)
        }
        _ => {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), expression.span),
                "borrow operand is not a static addressable place",
                "borrow the exact root, one Struct field, or one constant fixed-array element",
            );
            None
        }
    }
}
