use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind};

use super::SemanticInput;
use super::diagnostics::Errors;
use super::function_catalog::FunctionCatalog;
use super::layout_graph::Decl;
use super::owned_control_flow_resources::{
    conditional_root_borrow_budget_violation, loop_root_borrow_resources,
    root_borrow_resource_violation,
};
use super::root_borrow_straight_planning::{
    conditional_arm_scope_statement, plan_private_straight_root_borrow_function,
    shift_root_borrow_arm_ids, synthetic_straight_arm,
};
use super::type_model::{
    RootBorrowArmPlan, RootBorrowBudgetLimit, RootBorrowPlan, RootBorrowShape, Ty,
};
use crate::data_ownership_v1::diagnostics::span;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_private_root_borrow_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<RootBorrowPlan> {
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let [root_local_id, middle_id, return_id] = root.statements.as_slice() else {
        return plan_private_straight_root_borrow_function(
            input,
            module,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            catalog,
            result,
            true,
            errors,
        );
    };
    let middle =
        usize::try_from(*middle_id).ok().and_then(|index| function.body.statements.get(index))?;
    if let RawStatementKind::While { condition, body_block, .. } = &middle.kind {
        let root_local = usize::try_from(*root_local_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        let RawStatementKind::LocalDeclaration { name: root_name, .. } = &root_local.kind else {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), root_local.span),
                "loop borrowing requires one literal-initialized bool root",
                "declare the bool root before the loop",
            );
            return None;
        };
        let condition_expression = usize::try_from(*condition)
            .ok()
            .and_then(|index| function.body.expressions.get(index))?;
        let RawExpressionKind::Reference { name: condition_name } = &condition_expression.kind
        else {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), condition_expression.span),
                "loop borrowing requires the exact bool root as its condition",
                "loop directly on the literal-initialized bool root",
            );
            return None;
        };
        if condition_name.text != root_name.text {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), condition_name.span),
                "loop borrowing requires the exact bool root as its condition",
                "loop directly on the literal-initialized bool root",
            );
            return None;
        }
        let mut synthetic = function.clone();
        let scope_statement = u32::try_from(synthetic.body.statements.len()).ok()?;
        synthetic.body.statements.push(syntax::RawStatementSyntax {
            span: middle.span,
            kind: RawStatementKind::Block { block: *body_block },
        });
        let synthetic =
            synthetic_straight_arm(&synthetic, *root_local_id, scope_statement, *return_id)?;
        let plan = plan_private_straight_root_borrow_function(
            input,
            module,
            &synthetic,
            declarations,
            graph,
            node_types,
            layouts,
            catalog,
            result,
            false,
            errors,
        )?;
        let RootBorrowPlan {
            root_ty,
            root_initializer,
            root_at,
            shape: RootBorrowShape::Straight(body),
            aliases,
            reads,
            writes,
            calls,
            call_values,
            return_at,
        } = plan
        else {
            unreachable!("synthetic loop body is straight-line")
        };
        if root_ty.category != TypeCategory::Bool {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), root_local.span),
                "loop borrowing requires one literal-initialized bool root",
                "use the same bool root for the loop condition, body borrows, and final return",
            );
            return None;
        }
        if let Some(limit) =
            root_borrow_resource_violation(loop_root_borrow_resources(aliases, reads, writes))
        {
            let label = match limit {
                RootBorrowBudgetLimit::Values => "derived values",
                RootBorrowBudgetLimit::Places => "derived places",
                RootBorrowBudgetLimit::Transitions => "derived ownership transitions",
                RootBorrowBudgetLimit::Blocks => "derived blocks",
                RootBorrowBudgetLimit::Edges => "derived control-flow edges",
                RootBorrowBudgetLimit::ActiveBorrows => "simultaneously active borrows",
                RootBorrowBudgetLimit::CleanupPlans => "derived cleanup plans",
            };
            errors.at(
                "ZRYNA-M3201",
                span(input.sources(), middle.span),
                format!("loop borrowing exceeds the per-function limit for {label}"),
                "reduce body-local aliases or Copy reads before lowering",
            );
            return None;
        }
        return Some(RootBorrowPlan {
            root_ty,
            root_initializer,
            root_at,
            shape: RootBorrowShape::Loop {
                condition_at: span(input.sources(), condition_expression.span),
                loop_at: span(input.sources(), middle.span),
                body,
            },
            aliases,
            reads,
            writes,
            calls,
            call_values,
            return_at,
        });
    }
    let RawStatementKind::If { condition, then_block, else_clause, .. } = &middle.kind else {
        return plan_private_straight_root_borrow_function(
            input,
            module,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            catalog,
            result,
            true,
            errors,
        );
    };
    let Some(else_clause) = else_clause else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), middle.span),
            "conditional borrowing requires an explicit else arm",
            "discharge one complete lexical scope in both conditional arms",
        );
        return None;
    };
    let root_local = usize::try_from(*root_local_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration { name: root_name, .. } = &root_local.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "conditional borrowing requires one literal-initialized bool root",
            "declare the bool root before the conditional",
        );
        return None;
    };
    let condition_expression =
        usize::try_from(*condition).ok().and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Reference { name: condition_name } = &condition_expression.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), condition_expression.span),
            "conditional borrowing requires the exact bool root as its condition",
            "branch directly on the literal-initialized bool root",
        );
        return None;
    };
    if condition_name.text != root_name.text {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), condition_name.span),
            "conditional borrowing requires the exact bool root as its condition",
            "branch directly on the literal-initialized bool root",
        );
        return None;
    }
    let (then_scope, then_empty) =
        conditional_arm_scope_statement(function, *then_block, input.sources(), errors)?;
    let (else_scope, else_empty) =
        conditional_arm_scope_statement(function, else_clause.block, input.sources(), errors)?;
    if then_empty && else_empty {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), middle.span),
            "conditional borrowing requires at least one complete lexical borrow",
            "declare and use one arm-local Borrow or BorrowMut alias",
        );
        return None;
    }
    let plan_arm = |scope_statement: u32, errors: &mut Errors<'a>| {
        let synthetic =
            synthetic_straight_arm(function, *root_local_id, scope_statement, *return_id)?;
        plan_private_straight_root_borrow_function(
            input,
            module,
            &synthetic,
            declarations,
            graph,
            node_types,
            layouts,
            catalog,
            result,
            false,
            errors,
        )
    };
    let mut then_plan = if then_empty { None } else { Some(plan_arm(then_scope, errors)?) };
    let mut else_plan = if else_empty { None } else { Some(plan_arm(else_scope, errors)?) };
    let common = then_plan.as_ref().or(else_plan.as_ref())?;
    let common_root_ty = common.root_ty;
    let common_root_initializer = common.root_initializer.clone();
    let common_root_at = common.root_at;
    let common_return_at = common.return_at;
    if common_root_ty.category != TypeCategory::Bool {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "conditional borrowing requires one literal-initialized bool root",
            "use the same bool root for the condition, arm-local borrows, and final return",
        );
        return None;
    }
    let empty_arm = |scope_statement: u32| {
        let statement = usize::try_from(scope_statement)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        let RawStatementKind::Block { block } = statement.kind else { return None };
        let block =
            usize::try_from(block).ok().and_then(|index| function.body.blocks.get(index))?;
        Some(RootBorrowArmPlan {
            steps: Vec::new(),
            aliases: 0,
            reads: 0,
            writes: 0,
            calls: 0,
            call_values: 0,
            block_exit: span(input.sources(), block.close_brace_span),
        })
    };
    let then_arm = match then_plan.take() {
        Some(RootBorrowPlan { shape: RootBorrowShape::Straight(arm), .. }) => arm,
        Some(_) => unreachable!("synthetic arm is straight-line"),
        None => empty_arm(then_scope)?,
    };
    let mut else_arm = match else_plan.take() {
        Some(RootBorrowPlan { shape: RootBorrowShape::Straight(arm), .. }) => arm,
        Some(_) => unreachable!("synthetic arm is straight-line"),
        None => empty_arm(else_scope)?,
    };
    shift_root_borrow_arm_ids(&mut else_arm, then_arm.aliases)?;
    if let Some(limit) = conditional_root_borrow_budget_violation(
        then_arm.aliases,
        then_arm.reads,
        then_arm.writes,
        else_arm.aliases,
        else_arm.reads,
        else_arm.writes,
    ) {
        let label = match limit {
            RootBorrowBudgetLimit::Values => "derived values",
            RootBorrowBudgetLimit::Places => "derived places",
            RootBorrowBudgetLimit::Transitions => "derived ownership transitions",
            RootBorrowBudgetLimit::Blocks => "derived blocks",
            RootBorrowBudgetLimit::Edges => "derived control-flow edges",
            RootBorrowBudgetLimit::ActiveBorrows => "simultaneously active borrows",
            RootBorrowBudgetLimit::CleanupPlans => "derived cleanup plans",
        };
        errors.at(
            "ZRYNA-M3201",
            span(input.sources(), middle.span),
            format!("conditional borrowing exceeds the per-function limit for {label}"),
            "reduce arm-local aliases or Copy reads before lowering",
        );
        return None;
    }
    let aliases = then_arm.aliases.checked_add(else_arm.aliases)?;
    let reads = then_arm.reads.checked_add(else_arm.reads)?;
    let writes = then_arm.writes.checked_add(else_arm.writes)?;
    Some(RootBorrowPlan {
        root_ty: common_root_ty,
        root_initializer: common_root_initializer,
        root_at: common_root_at,
        shape: RootBorrowShape::Conditional {
            condition_at: span(input.sources(), condition_expression.span),
            branch_at: span(input.sources(), middle.span),
            then_arm,
            else_arm,
        },
        aliases,
        reads,
        writes,
        calls: 0,
        call_values: 0,
        return_at: common_return_at,
    })
}
