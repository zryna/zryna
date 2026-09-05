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
        parameter_fed_root(function, value)
    })
}

fn owned_type(
    file: &SourceUnit,
    id: u32,
    _module: usize,
    _declarations: &[Decl],
    _node_types: &[Option<Ty>],
) -> bool {
    file.type_syntax().get(id as usize).is_some_and(|ty| {
        matches!(ty.kind, RawTypeSyntaxKind::String { .. } | RawTypeSyntaxKind::Vec { .. })
    })
}

fn parameter_fed_root(function: &RawFunctionSyntax, id: u32) -> bool {
    let Some(expression) = function.body.expressions.get(id as usize) else { return false };
    let RawExpressionKind::Reference { name } = &expression.kind else { return false };
    let parameter =
        |name: &str| function.parameters.iter().any(|parameter| parameter.name.text == name);
    if parameter(&name.text) {
        return true;
    }
    function.body.statements.iter().any(|statement| {
        let RawStatementKind::LocalDeclaration { name: local, initializer, .. } = &statement.kind else { return false };
        if local.text != name.text { return false; }
        function.body.expressions.get(*initializer as usize).is_some_and(|initializer| {
            matches!(&initializer.kind, RawExpressionKind::Reference { name } if parameter(&name.text))
        })
    })
}
