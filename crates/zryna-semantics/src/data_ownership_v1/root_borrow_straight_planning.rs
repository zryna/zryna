use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_source::SourceMap;
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind};

use super::SemanticInput;
use super::borrow_call_resources::checked_straight_borrow_call_resources;
use super::diagnostics::Errors;
use super::function_catalog::FunctionCatalog;
use super::layout_graph::{Decl, semantic_type};
use super::owned_control_flow_resources::{
    checked_projected_root_borrow_call_resources, projected_root_borrow_resources,
    root_borrow_resource_violation, straight_root_borrow_resources,
};
use super::root_borrow_arm_planning::plan_root_borrow_arm;
use super::root_borrow_value_planning::plan_root_borrow_initializer;
use super::type_model::{
    RootBorrowArmPlan, RootBorrowBudgetLimit, RootBorrowPlan, RootBorrowShape, RootBorrowStep, Ty,
};
use crate::data_ownership_v1::diagnostics::span;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_private_straight_root_borrow_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    result: Ty,
    enforce_straight_budget: bool,
    errors: &mut Errors<'a>,
) -> Option<RootBorrowPlan> {
    let at = span(input.sources(), function.span);
    if function.export_span.is_some() || !function.parameters.is_empty() {
        errors.at(
            "ZRYNA-M3017",
            at,
            "shared-root borrowing requires one private parameter-free function",
            "keep the first shared-borrow checkpoint private and initialize its root locally",
        );
        return None;
    }
    if !result.is_copy()
        || !matches!(
            result.category,
            TypeCategory::Bool
                | TypeCategory::I32
                | TypeCategory::Struct
                | TypeCategory::FixedArray
        )
    {
        errors.at(
            "ZRYNA-M3017",
            at,
            "root borrowing requires an exact recursively Copy result",
            "return the initialized bool, i32, Copy struct, or Copy fixed-array root",
        );
        return None;
    }
    let file = input.syntax().files().get(module)?;
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let [root_local_id, nested_id, return_id] = root.statements.as_slice() else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root.span),
            "shared-root borrowing requires one root local, one lexical block, and one final return",
            "use `const root`, then one nested borrow block, then return the root",
        );
        return None;
    };
    let root_local = usize::try_from(*root_local_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration {
        mutable: root_mutable,
        name: root_name,
        type_syntax: root_type,
        initializer: root_initializer,
        ..
    } = &root_local.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "shared-root borrowing requires an initialized local root",
            "declare one exact bool or i32 root before the lexical borrow block",
        );
        return None;
    };
    let root_ty = semantic_type(file, *root_type, module, declarations, graph, node_types, errors)?;
    if root_ty != result
        || !root_ty.is_copy()
        || !matches!(
            root_ty.category,
            TypeCategory::Bool
                | TypeCategory::I32
                | TypeCategory::Struct
                | TypeCategory::FixedArray
        )
    {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "root borrowing requires one exact recursively Copy root matching the result",
            "give the root and function result the same Copy scalar or aggregate type",
        );
        return None;
    }
    let root_initializer = plan_root_borrow_initializer(
        input,
        module,
        function,
        file,
        declarations,
        graph,
        node_types,
        layouts,
        *root_initializer,
        root_ty,
        errors,
    )?;
    let nested_statement =
        usize::try_from(*nested_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Block { block: nested_block_id } = nested_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested_statement.span),
            "shared aliases must be contained by one explicit lexical block",
            "place every alias and borrow read inside one nested block",
        );
        return None;
    };
    let nested =
        usize::try_from(nested_block_id).ok().and_then(|index| function.body.blocks.get(index))?;
    let arm = plan_root_borrow_arm(
        input,
        module,
        function,
        file,
        declarations,
        graph,
        node_types,
        layouts,
        catalog,
        *root_mutable,
        root_name,
        root_ty,
        nested,
        enforce_straight_budget,
        errors,
    )?;
    let aliases = arm.aliases;
    let reads = arm.reads;
    let writes = arm.writes;
    let calls = arm.calls;
    let call_values = arm.call_values;
    let return_statement =
        usize::try_from(*return_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Return { value: returned, .. } = return_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), return_statement.span),
            "shared-root borrowing requires one final root return",
            "return the initialized root after the lexical block ends",
        );
        return None;
    };
    let returned =
        usize::try_from(returned).ok().and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Reference { name: returned_name } = &returned.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), returned.span),
            "shared-root borrowing must return the root by exact reference",
            "return the initialized bool or i32 root after all aliases end",
        );
        return None;
    };
    if returned_name.text != root_name.text {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), returned_name.span),
            "a lexical borrow alias or block-local read cannot escape",
            "return the root only after all lexical aliases end",
        );
        return None;
    }
    let resources = if matches!(root_ty.category, TypeCategory::Bool | TypeCategory::I32) {
        if calls == 0 {
            Some(straight_root_borrow_resources(aliases, reads, writes, calls, call_values))
        } else {
            checked_straight_borrow_call_resources(aliases, reads, writes, calls, call_values)
        }
    } else if calls == 0 {
        Some(projected_root_borrow_resources(&root_initializer, &arm))
    } else {
        checked_projected_root_borrow_call_resources(&root_initializer, &arm)
    };
    let Some(resources) = resources else {
        errors.at(
            "ZRYNA-M3201",
            span(input.sources(), nested.span),
            "lexical borrow-call resource planning overflows checked arithmetic",
            "reduce nested Copy aggregate call arguments or lexical borrow operations",
        );
        return None;
    };
    if let Some(limit) =
        enforce_straight_budget.then(|| root_borrow_resource_violation(resources)).flatten()
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
            span(input.sources(), nested.span),
            format!("shared-root borrowing exceeds the per-function limit for {label}"),
            "reduce lexical aliases or Copy reads before lowering",
        );
        return None;
    }
    Some(RootBorrowPlan {
        root_ty,
        root_initializer,
        root_at: span(input.sources(), root_local.span),
        shape: RootBorrowShape::Straight(arm),
        aliases,
        reads,
        writes,
        calls,
        call_values,
        return_at: span(input.sources(), return_statement.span),
    })
}

pub(super) fn shift_root_borrow_arm_ids(arm: &mut RootBorrowArmPlan, offset: usize) -> Option<()> {
    let offset = u32::try_from(offset).ok()?;
    for step in &mut arm.steps {
        let id = match step {
            RootBorrowStep::Begin { id, .. }
            | RootBorrowStep::Read { id, .. }
            | RootBorrowStep::Write { id, .. } => id,
            RootBorrowStep::OwnerRead { .. } | RootBorrowStep::Call(_) => continue,
        };
        id.0 = id.0.checked_add(offset)?;
    }
    Some(())
}

pub(super) fn conditional_arm_scope_statement<'a>(
    function: &'a syntax::RawFunctionSyntax,
    block_id: u32,
    sources: &SourceMap,
    errors: &mut Errors<'a>,
) -> Option<(u32, bool)> {
    let block = usize::try_from(block_id).ok().and_then(|index| function.body.blocks.get(index))?;
    let [scope_statement_id] = block.statements.as_slice() else {
        errors.at(
            "ZRYNA-M3017",
            span(sources, block.span),
            "conditional borrow arms require exactly one explicit lexical scope",
            "put one nested borrow block in each then and else arm",
        );
        return None;
    };
    let scope_statement = usize::try_from(*scope_statement_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Block { block: scope_block_id } = scope_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(sources, scope_statement.span),
            "conditional borrow arms require one nested lexical block",
            "wrap every arm-local borrow in one explicit nested block",
        );
        return None;
    };
    let scope =
        usize::try_from(scope_block_id).ok().and_then(|index| function.body.blocks.get(index))?;
    Some((*scope_statement_id, scope.statements.is_empty()))
}

pub(super) fn synthetic_straight_arm(
    function: &syntax::RawFunctionSyntax,
    root_local: u32,
    scope_statement: u32,
    return_statement: u32,
) -> Option<syntax::RawFunctionSyntax> {
    let mut synthetic = function.clone();
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let mut synthetic_root = root.clone();
    synthetic_root.statements = vec![root_local, scope_statement, return_statement];
    synthetic.body.root_block = u32::try_from(synthetic.body.blocks.len()).ok()?;
    synthetic.body.blocks.push(synthetic_root);
    Some(synthetic)
}
