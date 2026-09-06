use zryna_syntax::v4::{RawExpressionKind, RawFunctionSyntax, RawStatementKind};

pub(in crate::data_ownership_v1) fn requires_structured_cfg(function: &RawFunctionSyntax) -> bool {
    let root_has_structured_statement = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
        .is_some_and(|root| {
            root.statements.iter().any(|id| {
                usize::try_from(*id)
                    .ok()
                    .and_then(|index| function.body.statements.get(index))
                    .is_some_and(|statement| {
                        matches!(
                            statement.kind,
                            RawStatementKind::Block { .. }
                                | RawStatementKind::If { .. }
                                | RawStatementKind::While { .. }
                                | RawStatementKind::WeakUpgrade { .. }
                        )
                    })
            })
        });
    root_has_structured_statement
        || function
            .body
            .expressions
            .iter()
            .any(|expression| matches!(expression.kind, RawExpressionKind::Match { .. }))
}
