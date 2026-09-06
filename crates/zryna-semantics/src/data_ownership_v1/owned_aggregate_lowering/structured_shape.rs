use zryna_syntax::v4::{RawExpressionKind, RawFunctionSyntax, RawStatementKind};

pub(in crate::data_ownership_v1) fn requires_structured_cfg(function: &RawFunctionSyntax) -> bool {
    function.body.statements.iter().any(|statement| {
        matches!(
            statement.kind,
            RawStatementKind::Block { .. }
                | RawStatementKind::If { .. }
                | RawStatementKind::While { .. }
                | RawStatementKind::WeakUpgrade { .. }
        )
    }) || function
        .body
        .expressions
        .iter()
        .any(|expression| matches!(expression.kind, RawExpressionKind::Match { .. }))
}
