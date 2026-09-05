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
    if !signature.private || signature.has_borrow_parameters() {
        return false;
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
