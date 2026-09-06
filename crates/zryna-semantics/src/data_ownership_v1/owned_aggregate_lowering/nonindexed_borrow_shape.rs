use super::super::{Decl, Ty};
use zryna_syntax::v4::{
    RawExpressionKind, RawFunctionSyntax, RawStatementKind, RawTypeSyntaxKind, SourceUnit,
};

pub(in crate::data_ownership_v1) fn has_nonindexed_owned_borrow(
    function: &RawFunctionSyntax,
    file: &SourceUnit,
    module: usize,
    declarations: &[Decl],
    node_types: &[Option<Ty>],
) -> bool {
    // Preserve established locally constructed root and projected-borrow routes.
    if function.export_span.is_some()
        || function.parameters.is_empty()
        || function.body.statements.iter().any(|statement| {
            !matches!(
                statement.kind,
                RawStatementKind::LocalDeclaration { .. }
                    | RawStatementKind::Assignment { .. }
                    | RawStatementKind::Block { .. }
                    | RawStatementKind::ExpressionStatement { .. }
                    | RawStatementKind::Return { .. }
            )
        })
    {
        return false;
    }
    function.body.statements.iter().any(|statement| {
        let RawStatementKind::LocalDeclaration { type_syntax, initializer, .. } = statement.kind
        else {
            return false;
        };
        let Some(ty) = file.type_syntax().get(type_syntax as usize) else { return false };
        let (RawTypeSyntaxKind::Borrow { argument, .. }
        | RawTypeSyntaxKind::BorrowMut { argument, .. }) = ty.kind
        else {
            return false;
        };
        if !owned_type(file, argument, module, declarations, node_types) {
            return false;
        }
        let Some(expression) = function.body.expressions.get(initializer as usize) else {
            return false;
        };
        let (RawExpressionKind::Borrow { value, .. } | RawExpressionKind::BorrowMut { value, .. }) =
            expression.kind
        else {
            return false;
        };
        parameter_fed_place(function, value)
    })
}

fn owned_type(
    file: &SourceUnit,
    id: u32,
    module: usize,
    declarations: &[Decl],
    node_types: &[Option<Ty>],
) -> bool {
    file.type_syntax().get(id as usize).is_some_and(|ty| match &ty.kind {
        RawTypeSyntaxKind::String { .. } | RawTypeSyntaxKind::Vec { .. } => true,
        RawTypeSyntaxKind::FixedArray { element, .. } => {
            owned_type(file, *element, module, declarations, node_types)
        }
        RawTypeSyntaxKind::Named { name } => declarations
            .iter()
            .find(|declaration| declaration.module == module && declaration.name == name.text)
            .and_then(|declaration| node_types.get(declaration.node.0 as usize))
            .copied()
            .flatten()
            .is_some_and(|ty| !ty.is_copy() && ty.category == zryna_layout::TypeCategory::Struct),
        _ => false,
    })
}

fn parameter_fed_place(function: &RawFunctionSyntax, id: u32) -> bool {
    let Some(expression) = function.body.expressions.get(id as usize) else { return false };
    match &expression.kind {
        RawExpressionKind::FieldAccess { base, .. } => parameter_fed_place(function, *base),
        RawExpressionKind::Index { base, index, .. }
            if function.body.expressions.get(*index as usize).is_some_and(|index| {
                matches!(index.kind, RawExpressionKind::I32Literal { .. })
            }) =>
        {
            parameter_fed_place(function, *base)
        }
        RawExpressionKind::Reference { name } => {
            let parameter = |name: &str| {
                function.parameters.iter().any(|parameter| parameter.name.text == name)
            };
            if parameter(&name.text) {
                return true;
            }
            function.body.statements.iter().any(|statement| {
                let RawStatementKind::LocalDeclaration {
                    name: local,
                    initializer,
                    ..
                } = &statement.kind
                else {
                    return false;
                };
                if local.text != name.text {
                    return false;
                }
                function.body.expressions.get(*initializer as usize).is_some_and(|initializer| {
                    matches!(&initializer.kind, RawExpressionKind::Reference { name } if parameter(&name.text))
                })
            })
        }
        _ => false,
    }
}
