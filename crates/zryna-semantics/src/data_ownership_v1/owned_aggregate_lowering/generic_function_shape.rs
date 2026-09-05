use zryna_layout::{TypeCategory, VerifiedLayouts};
use zryna_syntax::v4::{
    RawExpressionKind, RawFunctionSyntax, RawStatementKind, RawTypeSyntaxKind, SourceUnit,
};

use super::super::function_catalog::{FunctionCatalog, FunctionResolution, FunctionSignature};
use super::super::{Decl, Ty};

// Select a new route from already authenticated shape only. This probe emits no diagnostics
// and never evaluates an expression; the selected lowerer still validates each operation.
pub(in crate::data_ownership_v1) fn requires_generic_function(
    function: &RawFunctionSyntax,
    (signature, catalog): (&FunctionSignature, &FunctionCatalog),
    file: &SourceUnit,
    declarations: &[Decl],
    node_types: &[Option<Ty>],
    layouts: &VerifiedLayouts,
) -> bool {
    if !signature.private {
        return false;
    }
    if super::has_nonindexed_owned_borrow(
        function,
        file,
        signature.id.module.0 as usize,
        declarations,
        node_types,
    ) {
        return true;
    }
    if signature.has_borrow_parameters() {
        return super::has_indexed_borrow(function)
            || signature.borrow_parameters.iter().any(|parameter| !parameter.referent.is_copy())
            || signature.parameters.iter().any(|ty| !ty.is_copy())
            || !signature.result.is_copy();
    }
    if super::has_indexed_borrow(function) {
        return true;
    }
    if super::mixed_shape::requires_summary(signature.result, layouts)
        || signature.parameters.iter().any(|ty| !ty.is_copy() && *ty != signature.result)
        || (signature.parameters.len() > 1 && signature.parameters.iter().any(|ty| !ty.is_copy()))
        || signature.parameters.iter().any(|ty| {
            super::mixed_shape::requires_summary(*ty, layouts)
                || (!ty.is_copy()
                    && matches!(
                        ty.category,
                        TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
                    ))
        })
    {
        return true;
    }
    let has_index = function
        .body
        .expressions
        .iter()
        .any(|expression| matches!(expression.kind, RawExpressionKind::Index { .. }));
    if has_index && signature.parameters.iter().any(|ty| ty.category == TypeCategory::Vec) {
        return true;
    }
    if checked_array_shape(function, signature, file, layouts) {
        return true;
    }
    if indexed_container_call_base(function, signature.id.module.0 as usize, catalog) {
        return true;
    }
    if function.body.expressions.iter().any(|expression| {
        let RawExpressionKind::Index { base, .. } = expression.kind else { return false };
        function.body.expressions.get(base as usize).is_some_and(|base| {
            matches!(
                base.kind,
                RawExpressionKind::FixedArrayConstruction { .. }
                    | RawExpressionKind::VecConstruction { .. }
            )
        })
    }) {
        return true;
    }
    if generic_call(function, signature.id.module.0 as usize, catalog, layouts) {
        return true;
    }
    let legacy_owned_result = !signature.result.is_copy()
        && matches!(
            signature.result.category,
            TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
        );
    function.body.statements.iter().any(|statement| {
        let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind else {
            return false;
        };
        let mut current = type_syntax;
        let mut array = false;
        for _ in 0..file.type_syntax().len() {
            let Some(ty) = file.type_syntax().get(current as usize) else { return false };
            return match &ty.kind {
                RawTypeSyntaxKind::FixedArray { element, .. } => {
                    array = true;
                    current = *element;
                    continue;
                }
                RawTypeSyntaxKind::Named { name } => declarations
                    .iter()
                    .find(|decl| {
                        decl.module == signature.id.module.0 as usize && decl.name == name.text
                    })
                    .and_then(|decl| node_types.get(decl.node.0 as usize).copied().flatten())
                    .is_some_and(|ty| {
                        super::mixed_shape::requires_summary(ty, layouts)
                            || (array && !ty.is_copy() && ty.category == TypeCategory::Enum)
                            || (!legacy_owned_result && !ty.is_copy())
                    }),
                RawTypeSyntaxKind::Vec { argument, .. } => {
                    array || has_index || !simple_vec_element(file, *argument)
                }
                RawTypeSyntaxKind::String { .. } => array && !legacy_owned_result,
                _ => false,
            };
        }
        false
    })
}

fn indexed_container_call_base(
    function: &RawFunctionSyntax,
    module: usize,
    catalog: &FunctionCatalog,
) -> bool {
    function.body.expressions.iter().any(|expression| {
        let RawExpressionKind::Index { base, .. } = expression.kind else { return false };
        let Some(base) = function.body.expressions.get(base as usize) else { return false };
        let RawExpressionKind::Call { callee, .. } = &base.kind else { return false };
        let FunctionResolution::Exact(callee) = catalog.resolve(module, &callee.text) else {
            return false;
        };
        callee.private
            && matches!(callee.result.category, TypeCategory::FixedArray | TypeCategory::Vec)
    })
}

fn generic_call(
    function: &RawFunctionSyntax,
    module: usize,
    catalog: &FunctionCatalog,
    layouts: &VerifiedLayouts,
) -> bool {
    function.body.expressions.iter().any(|expression| {
        let RawExpressionKind::Call { callee, .. } = &expression.kind else { return false };
        let FunctionResolution::Exact(signature) = catalog.resolve(module, &callee.text) else {
            return false;
        };
        let types = std::iter::once(signature.result).chain(signature.parameters.iter().copied());
        signature.private
            && !signature.has_borrow_parameters()
            && types.clone().all(|ty| super::mixed_shape::supported(ty, layouts))
            && types.clone().any(|ty| !ty.is_copy())
            && (signature.parameters.len() > 1
                || signature.parameters.iter().any(|ty| *ty != signature.result)
                || types.clone().any(|ty| super::mixed_shape::requires_summary(ty, layouts))
                || signature.parameters.iter().any(|ty| {
                    !ty.is_copy()
                        && matches!(
                            ty.category,
                            TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
                        )
                }))
    })
}

fn simple_vec_element(file: &SourceUnit, id: u32) -> bool {
    file.type_syntax().get(id as usize).is_some_and(|ty| match &ty.kind {
        RawTypeSyntaxKind::Named { name } => matches!(name.text.as_str(), "bool" | "i32"),
        RawTypeSyntaxKind::String { .. } => true,
        _ => false,
    })
}

fn checked_array_shape(
    function: &RawFunctionSyntax,
    signature: &FunctionSignature,
    file: &SourceUnit,
    layouts: &VerifiedLayouts,
) -> bool {
    use super::super::function_catalog::FunctionParameterOrder;
    function.body.expressions.iter().any(|expression| {
        let RawExpressionKind::Index { base, index, .. } = expression.kind else { return false };
        let Some(base) = function.body.expressions.get(base as usize) else { return false };
        let cloned = matches!(base.kind, RawExpressionKind::Clone { .. });
        let mut base = if let RawExpressionKind::Clone { value, .. } = base.kind {
            let Some(base) = function.body.expressions.get(value as usize) else { return false };
            base
        } else {
            base
        };
        while cloned {
            let parent = match base.kind {
                RawExpressionKind::FieldAccess { base, .. } => base,
                RawExpressionKind::Index { base, .. } => base,
                _ => break,
            };
            let Some(parent) = function.body.expressions.get(parent as usize) else { return false };
            base = parent;
        }
        let RawExpressionKind::Reference { name } = &base.kind else { return false };
        let parameter_type = function
            .parameters
            .iter()
            .zip(&signature.parameter_order)
            .find(|(parameter, _)| parameter.name.text == name.text)
            .and_then(|(_, order)| match *order {
                FunctionParameterOrder::Value(index) => signature.parameters.get(index as usize),
                FunctionParameterOrder::Borrow(_) => None,
            });
        if cloned
            && parameter_type.is_some_and(|ty| {
                matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            })
        {
            return true;
        }
        let parameter_length = parameter_type
            .filter(|ty| ty.category == TypeCategory::FixedArray)
            .and_then(|ty| layouts.type_by_id(ty.layout))
            .and_then(zryna_layout::VerifiedType::array_length);
        let local_type = function.body.statements.iter().find_map(|statement| {
            let RawStatementKind::LocalDeclaration { name: local, type_syntax, .. } =
                &statement.kind
            else {
                return None;
            };
            if local.text != name.text {
                return None;
            }
            file.type_syntax().get(*type_syntax as usize)
        });
        if cloned
            && local_type.is_some_and(|ty| match &ty.kind {
                RawTypeSyntaxKind::FixedArray { .. } => true,
                RawTypeSyntaxKind::Named { name } => !matches!(name.text.as_str(), "bool" | "i32"),
                _ => false,
            })
        {
            return true;
        }
        let local_length = local_type.and_then(|ty| match ty.kind {
            RawTypeSyntaxKind::FixedArray { length, .. } => Some(u64::from(length)),
            _ => None,
        });
        let Some(length) = parameter_length.or(local_length) else { return false };
        let Some(index) = function.body.expressions.get(index as usize) else { return false };
        cloned || super::ordinary_indexed_array_preparation::checked_index(&index.kind, length)
    })
}
