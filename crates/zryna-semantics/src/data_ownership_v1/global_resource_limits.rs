use super::SemanticInput;
use crate::data_ownership_v1::diagnostics::Errors;
use crate::data_ownership_v1::diagnostics::span;
use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_ownership_runtime_abi as ownership_runtime_abi;
use zryna_source::Span;
use zryna_syntax::v4::{
    self as syntax, RawExpressionKind, RawFieldInitializerKind, RawStatementKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProgramCfgBudgetLimit {
    Blocks,
    Edges,
}

pub(super) fn raw_function_value_count(function: &raw::Function) -> Option<usize> {
    function.blocks.iter().try_fold(function.parameters.len(), |total, block| {
        let total = total.checked_add(block.parameters.len())?;
        total.checked_add(
            block.instructions.iter().filter(|instruction| instruction.result.is_some()).count(),
        )
    })
}

pub(super) fn accumulate_generated_value_function(
    values: usize,
    function: &raw::Function,
    errors: &mut Errors<'_>,
) -> Option<usize> {
    let additional = raw_function_value_count(function);
    let total = additional.and_then(|additional| values.checked_add(additional));
    let Some(total) = total.filter(|total| *total <= ir::MAX_VALUES_PER_PROGRAM) else {
        errors.at(
            "ZRYNA-M3201",
            function.span,
            format!(
                "generated values exceed the program M3 limit of {}",
                ir::MAX_VALUES_PER_PROGRAM
            ),
            "reduce functions or generated result-producing expressions",
        );
        return None;
    };
    Some(total)
}

pub(super) fn generated_cfg_budget_violation(
    current_blocks: usize,
    current_edges: usize,
    additional_blocks: usize,
    additional_edges: usize,
) -> Option<ProgramCfgBudgetLimit> {
    if current_blocks
        .checked_add(additional_blocks)
        .is_none_or(|total| total > ir::MAX_BLOCKS_PER_PROGRAM)
    {
        Some(ProgramCfgBudgetLimit::Blocks)
    } else if current_edges
        .checked_add(additional_edges)
        .is_none_or(|total| total > ir::MAX_CFG_EDGES_PER_PROGRAM)
    {
        Some(ProgramCfgBudgetLimit::Edges)
    } else {
        None
    }
}

pub(super) fn raw_terminator_edge_count(terminator: &raw::Terminator) -> usize {
    match terminator {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
        raw::Terminator::Jump(_) => 1,
        raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
        raw::Terminator::EnumMatch { arms, .. } => arms.len(),
    }
}

pub(super) fn accumulate_generated_cfg_function(
    blocks: usize,
    edges: usize,
    function: &raw::Function,
    errors: &mut Errors<'_>,
) -> Option<(usize, usize)> {
    let additional_blocks = function.blocks.len();
    let additional_edges = function
        .blocks
        .iter()
        .flat_map(|block| &block.terminators)
        .try_fold(0_usize, |total, terminator| {
            total.checked_add(raw_terminator_edge_count(&terminator.kind))
        });
    let Some(additional_edges) = additional_edges else {
        errors.at(
            "ZRYNA-M3201",
            function.span,
            "generated CFG edge accounting overflowed",
            "reduce generated control flow before IR verification",
        );
        return None;
    };
    let Some(limit) =
        generated_cfg_budget_violation(blocks, edges, additional_blocks, additional_edges)
    else {
        return Some((
            blocks.checked_add(additional_blocks)?,
            edges.checked_add(additional_edges)?,
        ));
    };
    let (label, maximum, guidance) = match limit {
        ProgramCfgBudgetLimit::Blocks => (
            "generated blocks",
            ir::MAX_BLOCKS_PER_PROGRAM,
            "reduce functions or generated owned control-flow blocks",
        ),
        ProgramCfgBudgetLimit::Edges => (
            "generated CFG edges",
            ir::MAX_CFG_EDGES_PER_PROGRAM,
            "reduce generated owned branches and loops",
        ),
    };
    errors.at(
        "ZRYNA-M3201",
        function.span,
        format!("{label} exceed the program M3 limit of {maximum}"),
        guidance,
    );
    None
}

pub(super) fn semantic_preflight(input: SemanticInput<'_>, errors: &mut Errors<'_>) {
    let mut declarations = 0_usize;
    let mut program_values = 0_usize;
    let mut string_literal_bytes = 0_usize;
    let mut aggregate_operands = 0_usize;
    for file in input.syntax().files() {
        for declaration in file.data_declarations() {
            declarations = declarations.saturating_add(1);
            if declarations > ir::MAX_NOMINAL_DECLARATIONS {
                errors.at(
                    "ZRYNA-M3201",
                    span(input.sources(), declaration.span),
                    format!(
                        "aggregate declarations exceed the M3 limit of {}",
                        ir::MAX_NOMINAL_DECLARATIONS
                    ),
                    "reduce nominal declarations before aggregate semantic analysis",
                );
                return;
            }
        }
        for function in file.functions() {
            for expression in &function.body.expressions {
                let operands = match &expression.kind {
                    RawExpressionKind::StructConstruction { fields, .. } => fields.len(),
                    RawExpressionKind::EnumConstruction { payload, .. } => {
                        usize::from(payload.is_some())
                    }
                    RawExpressionKind::FixedArrayConstruction { elements, .. }
                    | RawExpressionKind::VecConstruction { elements, .. } => elements.len(),
                    RawExpressionKind::VecPush { .. } => 1,
                    _ => 0,
                };
                let Some(total) = preflight_aggregate_operand_total(
                    aggregate_operands,
                    operands,
                    span(input.sources(), expression.span),
                    errors,
                ) else {
                    return;
                };
                aggregate_operands = total;
                let RawExpressionKind::StringLiteral { spelling } = &expression.kind else {
                    continue;
                };
                let bytes = spelling.len().saturating_sub(2);
                if string_byte_budget_violation(string_literal_bytes, bytes) {
                    errors.at(
                        "ZRYNA-M3201",
                        span(input.sources(), expression.span),
                        format!(
                            "String literal bytes exceed the M3 limit of {}",
                            ir::MAX_STRING_LITERAL_BYTES
                        ),
                        "reduce cumulative String literal bytes before semantic lowering",
                    );
                    return;
                }
                string_literal_bytes += bytes;
            }
            let Some(total) = preflight_function_resources(input, function, program_values, errors)
            else {
                return;
            };
            program_values = total;
        }
    }
}

fn preflight_function_resources(
    input: SemanticInput<'_>,
    function: &syntax::RawFunctionSyntax,
    program_values: usize,
    errors: &mut Errors<'_>,
) -> Option<usize> {
    let values = derived_value_count(function);
    let (message, guidance) = match value_budget_violation(program_values, values) {
        Some(ValueBudgetLimit::Function) => (
            format!(
                "derived values exceed the per-function M3 limit of {}",
                ir::MAX_VALUES_PER_FUNCTION
            ),
            "reduce parameters or result-producing expressions",
        ),
        Some(ValueBudgetLimit::Program) => (
            format!("derived values exceed the program M3 limit of {}", ir::MAX_VALUES_PER_PROGRAM),
            "reduce functions, parameters, or result-producing expressions",
        ),
        None => {
            if resource_budget_violation(
                function.body.expressions.len(),
                1,
                ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
            ) {
                (
                    format!(
                        "derived cleanup sites exceed the per-function M3 limit of {}",
                        ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                    ),
                    "reduce fallible expressions and returns",
                )
            } else {
                return program_values.checked_add(values);
            }
        }
    };
    errors.at("ZRYNA-M3201", span(input.sources(), function.span), message, guidance);
    None
}

pub(super) fn string_byte_budget_violation(program_bytes: usize, literal_bytes: usize) -> bool {
    program_bytes
        .checked_add(literal_bytes)
        .is_none_or(|total| total > ir::MAX_STRING_LITERAL_BYTES)
}

pub(super) fn checked_string_concat_bytes(left: u64, right: u64) -> Option<u64> {
    left.checked_add(right).filter(|total| *total <= ownership_runtime_abi::MAX_STRING_BYTES)
}

pub(super) fn aggregate_operand_budget_violation(current: usize, additional: usize) -> bool {
    current.checked_add(additional).is_none_or(|total| total > ir::MAX_AGGREGATE_OPERANDS)
}

pub(super) fn preflight_aggregate_operand_total(
    current: usize,
    additional: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<usize> {
    if aggregate_operand_budget_violation(current, additional) {
        errors.at(
            "ZRYNA-M3201",
            at,
            format!("aggregate operands exceed the M3 limit of {}", ir::MAX_AGGREGATE_OPERANDS),
            "reduce cumulative Struct, enum, fixed-array, Vec, and push operands",
        );
        None
    } else {
        current.checked_add(additional)
    }
}

pub(super) fn resource_budget_violation(current: usize, extra: usize, maximum: usize) -> bool {
    current.checked_add(extra).is_none_or(|total| total > maximum)
}

pub(super) fn aggregate_transition_budget_violation(
    current: usize,
    reserved: usize,
    additional: usize,
) -> bool {
    reserved.checked_add(additional).is_none_or(|extra| {
        resource_budget_violation(current, extra, ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ValueBudgetLimit {
    Function,
    Program,
}

pub(super) fn value_budget_violation(
    program_values: usize,
    function_values: usize,
) -> Option<ValueBudgetLimit> {
    if function_values > ir::MAX_VALUES_PER_FUNCTION {
        return Some(ValueBudgetLimit::Function);
    }
    match program_values.checked_add(function_values) {
        Some(total) if total <= ir::MAX_VALUES_PER_PROGRAM => None,
        _ => Some(ValueBudgetLimit::Program),
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn derived_value_count(function: &syntax::RawFunctionSyntax) -> usize {
    fn owned_read(body: &syntax::RawFunctionBodySyntax, id: u32) -> usize {
        let Some(expression) =
            usize::try_from(id).ok().and_then(|index| body.expressions.get(index))
        else {
            return 0;
        };
        if matches!(expression.kind, RawExpressionKind::Reference { .. }) {
            0
        } else {
            value(body, id)
        }
    }
    fn place(body: &syntax::RawFunctionBodySyntax, id: u32) -> usize {
        let Some(expression) =
            usize::try_from(id).ok().and_then(|index| body.expressions.get(index))
        else {
            return 0;
        };
        match &expression.kind {
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. } => {
                place(body, *base)
            }
            RawExpressionKind::StructConstruction { .. }
            | RawExpressionKind::EnumConstruction { .. }
            | RawExpressionKind::FixedArrayConstruction { .. } => value(body, id),
            _ => 0,
        }
    }
    fn value(body: &syntax::RawFunctionBodySyntax, id: u32) -> usize {
        let Some(expression) =
            usize::try_from(id).ok().and_then(|index| body.expressions.get(index))
        else {
            return 0;
        };
        let children = match &expression.kind {
            RawExpressionKind::Borrow { .. } | RawExpressionKind::BorrowMut { .. } => return 0,
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. } => {
                place(body, *base)
            }
            RawExpressionKind::Negation { operand, .. } => value(body, *operand),
            RawExpressionKind::Clone { value: operand, .. } => owned_read(body, *operand),
            RawExpressionKind::Addition { lhs, rhs, .. }
            | RawExpressionKind::Subtraction { lhs, rhs, .. }
            | RawExpressionKind::Multiplication { lhs, rhs, .. }
            | RawExpressionKind::Equal { lhs, rhs, .. }
            | RawExpressionKind::NotEqual { lhs, rhs, .. }
            | RawExpressionKind::LessThan { lhs, rhs, .. }
            | RawExpressionKind::LessEqual { lhs, rhs, .. }
            | RawExpressionKind::GreaterThan { lhs, rhs, .. }
            | RawExpressionKind::GreaterEqual { lhs, rhs, .. } => {
                value(body, *lhs).saturating_add(value(body, *rhs))
            }
            RawExpressionKind::StructConstruction { fields, .. } => fields
                .iter()
                .map(|field| match field.kind {
                    RawFieldInitializerKind::Shorthand { value: id, .. }
                    | RawFieldInitializerKind::Explicit { value: id, .. } => value(body, id),
                })
                .sum(),
            RawExpressionKind::EnumConstruction { payload, .. } => {
                payload.map_or(0, |id| value(body, id))
            }
            RawExpressionKind::FixedArrayConstruction { elements, .. }
            | RawExpressionKind::VecConstruction { elements, .. } => {
                elements.iter().map(|id| value(body, *id)).sum()
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                arguments.iter().map(|id| owned_read(body, *id)).sum()
            }
            RawExpressionKind::Call { arguments, .. } => {
                arguments.iter().map(|id| value(body, *id)).sum()
            }
            RawExpressionKind::VecPush { value: pushed, .. } => {
                return value(body, *pushed);
            }
            RawExpressionKind::Match { arms, .. } => return arms.len(),
            _ => 0,
        };
        children.saturating_add(1)
    }
    let mut count = function.parameters.len();
    // Protocol-v4 authentication proves that every statement belongs to exactly one
    // reachable block, so one arena pass counts nested Block/If/While bodies without
    // either missing them or recursively counting them twice.
    for statement in &function.body.statements {
        count = count.saturating_add(match statement.kind {
            RawStatementKind::LocalDeclaration { initializer, .. } => {
                value(&function.body, initializer)
            }
            RawStatementKind::Assignment { target, value: rhs, .. } => {
                place(&function.body, target).saturating_add(value(&function.body, rhs))
            }
            RawStatementKind::Return { value: returned, .. } => value(&function.body, returned),
            RawStatementKind::ExpressionStatement { expression, .. } => {
                value(&function.body, expression)
            }
            RawStatementKind::If { condition, .. } | RawStatementKind::While { condition, .. } => {
                value(&function.body, condition)
            }
            RawStatementKind::WeakUpgrade { weak, .. } => value(&function.body, weak),
            RawStatementKind::Block { .. } => 0,
        });
    }
    count
}
