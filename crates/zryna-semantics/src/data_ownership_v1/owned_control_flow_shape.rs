use zryna_layout::TypeCategory;
use zryna_source::SourceMap;
use zryna_syntax::v4::{self as syntax, RawStatementKind};

use super::diagnostics::Errors;
use super::type_model::TerminalOwnedIf;
use crate::data_ownership_v1::diagnostics::span;

pub(super) fn root_is_terminal_if(function: &syntax::RawFunctionSyntax) -> bool {
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    let [statement] = root.statements.as_slice() else { return false };
    usize::try_from(*statement)
        .ok()
        .and_then(|index| function.body.statements.get(index))
        .is_some_and(|statement| matches!(statement.kind, RawStatementKind::If { .. }))
}

pub(super) fn is_terminal_owned_phi_candidate(
    function: &syntax::RawFunctionSyntax,
    result: TypeCategory,
    has_vec_operation: bool,
) -> bool {
    function.export_span.is_none()
        && root_is_terminal_if(function)
        && ((result == TypeCategory::String && !has_vec_operation) || result == TypeCategory::Vec)
}

pub(super) fn preflight_owned_loop_body(
    function: &syntax::RawFunctionSyntax,
    body_block: u32,
    allow_vec_effects: bool,
    sources: &SourceMap,
    errors: &mut Errors<'_>,
) -> bool {
    let Some(block) =
        usize::try_from(body_block).ok().and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    for statement_id in &block.statements {
        let Some(statement) = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))
        else {
            return false;
        };
        let supported = matches!(
            statement.kind,
            RawStatementKind::LocalDeclaration { .. } | RawStatementKind::Assignment { .. }
        ) || (allow_vec_effects
            && matches!(statement.kind, RawStatementKind::ExpressionStatement { .. }));
        if !supported {
            errors.at(
                "ZRYNA-M3016",
                span(sources, statement.span),
                "owned loop body contains an unsupported control-flow or ownership statement",
                "use loop-local declarations and push only into a loop-local Vec",
            );
            return false;
        }
    }
    true
}

pub(super) fn preflight_owned_loop_exit(
    function: &syntax::RawFunctionSyntax,
    loop_statement: u32,
    sources: &SourceMap,
    errors: &mut Errors<'_>,
) -> bool {
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    let Some(position) = root.statements.iter().position(|statement| *statement == loop_statement)
    else {
        return false;
    };
    let Some(next_id) = root.statements.get(position + 1).copied() else {
        return false;
    };
    let Some(next) =
        usize::try_from(next_id).ok().and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    if position + 2 != root.statements.len()
        || !matches!(next.kind, RawStatementKind::Return { .. })
    {
        errors.at(
            "ZRYNA-M3016",
            span(sources, next.span),
            "owned loop must be followed immediately by the sole final return",
            "remove repeated control flow and effects after the loop",
        );
        return false;
    }
    true
}

pub(super) fn terminal_owned_if(
    function: &syntax::RawFunctionSyntax,
    sources: &SourceMap,
    errors: &mut Errors<'_>,
) -> Option<TerminalOwnedIf> {
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let [statement_id] = root.statements.as_slice() else { return None };
    let statement = usize::try_from(*statement_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::If { condition, then_block, else_clause, .. } = &statement.kind else {
        return None;
    };
    let Some(else_clause) = else_clause else {
        errors.at(
            "ZRYNA-M3016",
            span(sources, statement.span),
            "terminal owned if requires an else branch",
            "return one exact owned value from both branches",
        );
        return None;
    };
    let arm = |block_id: u32, errors: &mut Errors<'_>| {
        let block =
            usize::try_from(block_id).ok().and_then(|index| function.body.blocks.get(index))?;
        let [arm_statement_id] = block.statements.as_slice() else {
            errors.at(
                "ZRYNA-M3016",
                span(sources, block.span),
                "terminal owned if arm must contain exactly one return",
                "remove fallthrough and extra branch statements",
            );
            return None;
        };
        let arm_statement = usize::try_from(*arm_statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        let RawStatementKind::Return { value, .. } = arm_statement.kind else {
            errors.at(
                "ZRYNA-M3016",
                span(sources, arm_statement.span),
                "terminal owned if arm must end with its sole return",
                "return one exact owned value directly from this branch",
            );
            return None;
        };
        Some((value, span(sources, arm_statement.span)))
    };
    let (then_value, then_span) = arm(*then_block, errors)?;
    let (else_value, else_span) = arm(else_clause.block, errors)?;
    Some(TerminalOwnedIf {
        condition: *condition,
        then_value,
        then_span,
        else_value,
        else_span,
        span: span(sources, statement.span),
    })
}
