//! Aggregate semantic lowering for the isolated `DataOwnershipV1` profile.
//!
//! This boundary accepts only authenticated protocol-v4 syntax, derives both layout authorities
//! itself, and returns only verifier-sealed IR. Raw layout and IR claims never cross the API.

use std::collections::{BTreeMap, BTreeSet};

use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::data_ownership_v1::{self as ir, RuntimeContractIdentity, raw};
use zryna_layout::{self as layout, StorageTarget, TypeCategory, raw as raw_layout};
use zryna_ownership_runtime_abi as ownership_runtime_abi;
use zryna_source::{FileId, MAX_SOURCE_FILES, SourceMap, Span, UntrustedSpan};
use zryna_syntax::v4::{
    self as syntax, RawDataDeclarationKind, RawExpressionKind, RawFieldInitializerKind,
    RawStatementKind, RawTypeSyntaxKind,
};

mod aggregate_resource_formulas;
mod diagnostics;
mod function_catalog;
mod global_resource_limits;
mod layout_graph;
mod owned_control_flow_resources;
mod root_borrow_arm_planning;
mod root_borrow_straight_planning;
mod root_borrow_value_planning;
mod string_vec_resource_estimates;
mod type_model;

use aggregate_resource_formulas::{
    PartialTransferBudgetViolation, aggregate_clone_budget_violation,
    cleanup_action_budget_violation, partial_assignment_budget_preflight,
    partial_return_budget_preflight, partial_transfer_budget_preflight,
    projected_aggregate_assignment_budget_violation,
    projected_aggregate_clone_assignment_budget_violation,
    projected_aggregate_clone_budget_violation, projected_string_clone_budget_violation,
    projected_subobject_assignment_budget_violation, projected_subobject_move_budget_violation,
    projected_subobject_return_budget_violation,
};
#[cfg(test)]
use aggregate_resource_formulas::{
    partial_assignment_place_delta, partial_return_place_delta, partial_transfer_place_delta,
};
use diagnostics::Errors;
#[cfg(test)]
use function_catalog::FunctionBorrowParameter;
use function_catalog::{
    FunctionCatalog, FunctionParameterOrder, FunctionResolution, FunctionSignature,
    build_function_catalog,
};
#[cfg(test)]
use global_resource_limits::{
    ProgramCfgBudgetLimit, ValueBudgetLimit, derived_value_count, generated_cfg_budget_violation,
    preflight_aggregate_operand_total, raw_function_value_count, raw_terminator_edge_count,
    string_byte_budget_violation, value_budget_violation,
};
use global_resource_limits::{
    accumulate_generated_cfg_function, accumulate_generated_value_function,
    aggregate_operand_budget_violation, aggregate_transition_budget_violation,
    checked_string_concat_bytes, resource_budget_violation, semantic_preflight,
};
use layout_graph::{Decl, build_graph, semantic_type};
#[cfg(test)]
use owned_control_flow_resources::{
    EnumPayloadMoveResourceEstimate, conditional_root_borrow_resources,
    enum_payload_move_resource_estimate, owned_place_budget_violation,
    projected_root_borrow_resource_counts, straight_root_borrow_budget_violation,
};
use owned_control_flow_resources::{
    OwnedCfgBudgetLimit, conditional_root_borrow_budget_violation, dense_owned_value_id,
    enum_payload_move_resource_violation, loop_root_borrow_resources, owned_cfg_budget_violation,
    owned_root_borrow_resource_violation, owned_value_budget_violation,
    preflight_owned_place_capacity, preflight_owned_place_capacity_with_reserved,
    root_borrow_resource_violation,
};
use root_borrow_straight_planning::{
    conditional_arm_scope_statement, plan_private_straight_root_borrow_function,
    shift_root_borrow_arm_ids, synthetic_straight_arm,
};
#[cfg(test)]
use string_vec_resource_estimates::owned_call_cleanup_budget_violation;
use string_vec_resource_estimates::{
    OwnedStringEstimateContext, OwnedStringEstimateError, OwnedStringEstimateOutcome,
    OwnedStringPreparationEstimate, VecPreparationEstimate, add_estimate_counts,
    cleanup_actions_after_additions, cleanup_actions_after_preparation,
    cleanup_actions_after_transfer, empty_owned_string_estimate,
    estimate_owned_string_call_arguments, estimate_owned_string_expression,
    vec_push_target_invalid,
};
#[cfg(test)]
use type_model::RootBorrowResources;
use type_model::{
    Binding, OwnedAggregatePlace, OwnedAggregatePlacePreflight, OwnedProjectionShapeEntry,
    OwnedRootBorrowSyntax, OwnedStaticProjectionKind, OwnedStringBranchState, OwnedVecBranchState,
    OwnerDelta, OwnerState, ProjectedAggregateAssignmentSource, ProjectedAggregateMoveContext,
    RootBorrowArmPlan, RootBorrowBudgetLimit, RootBorrowInitializer, RootBorrowLiteral,
    RootBorrowPlacePlan, RootBorrowPlan, RootBorrowProjectionKey, RootBorrowShape, RootBorrowStep,
    TerminalOwnedIf, Ty, apply_owner_delta, map_node_types,
};

/// Maximum retained semantic diagnostics, including the terminal budget diagnostic.
pub const MAX_SEMANTIC_DIAGNOSTICS: usize = 256;

const _: () = {
    assert!(MAX_SOURCE_FILES <= ir::MAX_MODULES);
    assert!(syntax::MAX_FUNCTIONS_PER_MODULE <= ir::MAX_FUNCTIONS_PER_MODULE);
    assert!(syntax::MAX_FUNCTIONS_PER_PROJECT <= ir::MAX_FUNCTIONS_PER_PROGRAM);
    assert!(syntax::MAX_PARAMETERS_PER_FUNCTION <= ir::MAX_PARAMETERS_PER_FUNCTION);
    assert!(syntax::MAX_DATA_DECLARATIONS_PER_MODULE <= ir::MAX_NOMINAL_DECLARATIONS);
    assert!(syntax::MAX_AGGREGATE_OPERANDS_PER_PROJECT <= ir::MAX_AGGREGATE_OPERANDS);
    assert!(MAX_SEMANTIC_DIAGNOSTICS == ir::MAX_DIAGNOSTICS);
};

/// Exact authenticated inputs for aggregate M3 semantics.
///
/// Raw protocol claims cannot enter this boundary.
///
/// ```compile_fail
/// fn bypass<'a>(raw: &'a zryna_syntax::v4::RawProjectSyntaxSnapshot,
///     sources: &'a zryna_source::SourceMap, entry: zryna_source::FileId)
///     -> Option<zryna_semantics::data_ownership_v1::SemanticInput<'a>> {
///     zryna_semantics::data_ownership_v1::SemanticInput::try_new(raw, sources, entry)
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SemanticInput<'a> {
    syntax: &'a syntax::ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
    entry: FileId,
}

impl<'a> SemanticInput<'a> {
    /// Authenticates syntax, source authority, provider success, and the exact entry module.
    #[must_use]
    pub fn try_new(
        syntax: &'a syntax::ProjectSyntaxSnapshot,
        sources: &'a SourceMap,
        entry: FileId,
    ) -> Option<Self> {
        (syntax.is_bound_to(sources)
            && sources.source(entry).is_some()
            && syntax.files().iter().filter(|file| file.id() == entry).count() == 1
            && syntax.diagnostics().iter().all(|d| d.severity() != Severity::Error))
        .then_some(Self { syntax, sources, entry })
    }

    /// Returns the authenticated syntax snapshot.
    #[must_use]
    pub const fn syntax(self) -> &'a syntax::ProjectSyntaxSnapshot {
        self.syntax
    }
    /// Returns the authoritative source map.
    #[must_use]
    pub const fn sources(self) -> &'a SourceMap {
        self.sources
    }
    /// Returns the independently selected entry source.
    #[must_use]
    pub const fn entry(self) -> FileId {
        self.entry
    }
}

/// Sealed M3 semantic result retaining verified IR and its exact ownership-runtime ABI authority.
///
/// Raw IR and runtime declarations cannot be recovered through this boundary.
///
/// ```compile_fail
/// fn recover(program: &zryna_semantics::data_ownership_v1::VerifiedProgram) {
///     let _: &zryna_ir::data_ownership_v1::raw::Program = program.verified_ir().raw();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct VerifiedProgram {
    ir: ir::VerifiedProgram,
    runtime_abi: ownership_runtime_abi::VerifiedOwnershipRuntimeAbi,
}

impl VerifiedProgram {
    /// Returns the opaque verified IR modules.
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = ir::VerifiedModule<'_>> {
        self.ir.modules()
    }
    /// Returns the retained exact ownership-runtime declaration authority.
    #[must_use]
    pub const fn runtime_abi(&self) -> &ownership_runtime_abi::VerifiedOwnershipRuntimeAbi {
        &self.runtime_abi
    }
    /// Returns the underlying sealed IR authority without exposing raw claims.
    #[must_use]
    pub const fn verified_ir(&self) -> &ir::VerifiedProgram {
        &self.ir
    }
}

/// Successful M3 lowering always carries mandatory IR and runtime-ABI verifier authority.
pub type SemanticResult = Result<VerifiedProgram, Vec<Diagnostic>>;

/// Resolves Copy-only aggregate semantics, derives dual layouts, lowers raw IR deterministically,
/// and immediately invokes the mandatory ownership verifier.
///
/// # Errors
/// Returns stable, bounded, source-located M3 semantic diagnostics, layout diagnostics, or IR
/// verifier diagnostics. No partially checked artifact is returned.
#[allow(clippy::too_many_lines)]
pub fn lower(input: SemanticInput<'_>) -> SemanticResult {
    let mut errors = Errors::new(input.sources());
    semantic_preflight(input, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let (graph, declarations) = build_graph(input, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let linear = layout::verify(&graph, input.sources(), StorageTarget::Linear32V1)?;
    let linux = layout::verify(&graph, input.sources(), StorageTarget::LinuxX8664V1)?;
    if linear.universe_identity() != linux.universe_identity() {
        errors.global(
            "ZRYNA-M3004",
            "the independently derived layout universes disagree",
            "reduce the aggregate type graph and report this deterministic compiler failure",
        );
        return Err(errors.finish());
    }
    let runtime_abi = match ownership_runtime_abi::verify_v1(
        ownership_runtime_abi::raw_v1(&linear, &linux),
        &linear,
        &linux,
    ) {
        Ok(authority) => authority,
        Err(violations) => {
            for violation in violations {
                errors.global(
                    violation.code(),
                    violation.message(),
                    "reduce the program and report this deterministic runtime ABI authority failure",
                );
            }
            return Err(errors.finish());
        }
    };
    let node_types = map_node_types(&graph, &linear, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }

    let catalog = build_function_catalog(input, &declarations, &graph, &node_types, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }

    let mut modules = Vec::with_capacity(input.syntax().files().len());
    let mut generated_values = 0_usize;
    let mut generated_blocks = 0_usize;
    let mut generated_edges = 0_usize;
    'modules: for (module_index, file) in input.syntax().files().iter().enumerate() {
        if !file.imports().is_empty() {
            errors.at(
                "ZRYNA-M3002",
                span(input.sources(), file.imports()[0].span),
                "aggregate M3 does not admit imported aggregate names",
                "declare the Copy aggregate in the same module",
            );
            continue;
        }
        let mut functions = Vec::with_capacity(file.functions().len());
        for (function_index, function) in file.functions().iter().enumerate() {
            let diagnostics_before = errors.len();
            if let Some(lowered) = lower_function(
                input,
                module_index,
                function_index,
                function,
                &declarations,
                &graph,
                &node_types,
                &linear,
                &catalog,
                &mut errors,
            ) {
                let Some(values) =
                    accumulate_generated_value_function(generated_values, &lowered, &mut errors)
                else {
                    break 'modules;
                };
                let Some((blocks, edges)) = accumulate_generated_cfg_function(
                    generated_blocks,
                    generated_edges,
                    &lowered,
                    &mut errors,
                ) else {
                    break 'modules;
                };
                generated_values = values;
                generated_blocks = blocks;
                generated_edges = edges;
                functions.push(lowered);
            } else if errors.len() == diagnostics_before {
                errors.at(
                    "ZRYNA-M3008",
                    span(input.sources(), function.span),
                    format!("function '{}' could not be lowered", function.name.text),
                    "reduce the function to one exact supported semantic form",
                );
            }
        }
        modules.push(raw::Module {
            id: raw::ModuleId(u32::try_from(module_index).unwrap_or(u32::MAX)),
            source_file: file.id(),
            data_declarations: u32::try_from(file.data_declarations().len()).unwrap_or(u32::MAX),
            functions,
        });
    }
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let claims = raw::AuthorityClaims {
        runtime: RuntimeContractIdentity::OwnershipRuntimeV1,
        type_universe: linear.universe_identity().as_bytes(),
        linear32_fingerprint: *linear.fingerprint(),
        linux_x86_64_fingerprint: *linux.fingerprint(),
    };
    let verified_ir = ir::verify(
        raw::Program {
            authorities: claims,
            entry_module: raw::ModuleId(input.entry().index()),
            modules,
        },
        input.sources(),
        input.entry(),
        linear,
        linux,
    )?;
    Ok(VerifiedProgram { ir: verified_ir, runtime_abi })
}

#[derive(Clone, Copy)]
struct OwnedStringPreparationBudget {
    cleanup_plans: usize,
    reserved_cleanup_plans: usize,
    cleanup_actions: usize,
    reserved_cleanup_actions: usize,
    places: usize,
    reserved_places: usize,
}

fn preflight_owned_string_preparation(
    estimate: OwnedStringPreparationEstimate,
    budget: OwnedStringPreparationBudget,
    cfg: &mut OwnedCfgState,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    let plans = budget.cleanup_plans.checked_add(budget.reserved_cleanup_plans);
    let actions = budget.cleanup_actions.checked_add(budget.reserved_cleanup_actions);
    if plans.is_none_or(|current| {
        resource_budget_violation(
            current,
            estimate.cleanup_plans,
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        )
    }) || actions.is_none_or(|current| {
        resource_budget_violation(
            current,
            estimate.cleanup_actions,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        )
    }) {
        errors.at(
            "ZRYNA-M3201",
            at,
            "recursive owned String preparation exceeds the per-function cleanup limits",
            "reduce nested String-producing expressions or simultaneously live owners",
        );
        return false;
    }
    if cfg.reserve_values(estimate.values, at, errors).is_none() {
        return false;
    }
    cfg.release_values(estimate.values);
    if !preflight_owned_place_capacity_with_reserved(
        budget.places,
        budget.reserved_places,
        estimate.places,
        at,
        errors,
    ) {
        return false;
    }
    cfg.preflight_transitions(estimate.transitions, at, errors)
}

fn span(sources: &SourceMap, value: UntrustedSpan) -> Span {
    sources.verify_span(value).expect("verified v4 span")
}

fn require_current_type_only_boundary(
    ty: Ty,
    at: Span,
    public: bool,
    errors: &mut Errors<'_>,
) -> Option<Ty> {
    if ty.is_copy() && ty.is_clone() {
        return Some(ty);
    }
    if public {
        errors.at(
            "ZRYNA-M3010",
            at,
            "public owned signatures are outside scalar ABI v1",
            "keep owned functions internal and export only bool/i32 signatures",
        );
    } else {
        let message = if ty.runtime_kind == 0 {
            "owned aggregate operations are not yet admitted by this lowering slice"
        } else {
            "owned String and Vec operations are not yet admitted by this lowering slice"
        };
        errors.at(
            "ZRYNA-M3003",
            at,
            message,
            "use the authenticated owned type only after owned-operation lowering is enabled",
        );
    }
    None
}

struct FunctionLowerer<'a, 'f, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'f syntax::RawFunctionSyntax,
    module: usize,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    borrow_bindings: BTreeMap<String, BorrowBinding>,
    places: Vec<raw::Place>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    instructions: Vec<raw::Instruction>,
    cleanup_plans: Vec<raw::CleanupPlan>,
    values: u32,
}

#[derive(Clone, Copy)]
struct BorrowBinding {
    ty: Ty,
    borrow: raw::BorrowId,
    access: raw::BorrowAccess,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn plan_private_root_borrow_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
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
        return_at: common_return_at,
    })
}

fn emit_root_borrow_initializer(
    initializer: RootBorrowInitializer,
    values: &mut u32,
    instructions: &mut Vec<raw::Instruction>,
) -> Option<raw::ValueId> {
    let (ty, at, kind) = match initializer {
        RootBorrowInitializer::Literal { literal, ty, at } => {
            let kind = match literal {
                RootBorrowLiteral::Bool(value) => raw::InstructionKind::BoolLiteral(value),
                RootBorrowLiteral::I32(value) => raw::InstructionKind::I32Literal(value),
            };
            (ty, at, kind)
        }
        RootBorrowInitializer::Struct { ty, fields, at } => {
            let fields = fields
                .into_iter()
                .map(|field| emit_root_borrow_initializer(field, values, instructions))
                .collect::<Option<Vec<_>>>()?;
            (ty, at, raw::InstructionKind::StructConstruct { fields, cleanup: None })
        }
        RootBorrowInitializer::FixedArray { ty, elements, at } => {
            let elements = elements
                .into_iter()
                .map(|element| emit_root_borrow_initializer(element, values, instructions))
                .collect::<Option<Vec<_>>>()?;
            (ty, at, raw::InstructionKind::FixedArrayConstruct { elements, cleanup: None })
        }
    };
    let value = raw::ValueId(*values);
    *values = values.checked_add(1)?;
    instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
        span: at,
        kind,
    });
    Some(value)
}

fn materialize_root_borrow_place(
    plan: &RootBorrowPlacePlan,
    root: raw::PlaceId,
    places: &mut Vec<raw::Place>,
    projected: &mut BTreeMap<Vec<RootBorrowProjectionKey>, raw::PlaceId>,
) -> Option<raw::PlaceId> {
    let mut parent = root;
    let mut prefix = Vec::with_capacity(plan.projections.len());
    for projection in &plan.projections {
        prefix.push(projection.key);
        if let Some(place) = projected.get(&prefix).copied() {
            parent = place;
            continue;
        }
        let id = raw::PlaceId(u32::try_from(places.len()).ok()?);
        let kind = match projection.key {
            RootBorrowProjectionKey::StructField(ordinal) => {
                raw::PlaceKind::StructField { base: parent, ordinal }
            }
            RootBorrowProjectionKey::FixedArrayConstant(index) => {
                raw::PlaceKind::FixedArrayConstant { base: parent, index }
            }
        };
        places.push(raw::Place { id, ty: projection.ty.ir, span: projection.at, kind });
        projected.insert(prefix.clone(), id);
        parent = id;
    }
    Some(parent)
}

fn emit_root_borrow_arm(
    arm: RootBorrowArmPlan,
    root_place: raw::PlaceId,
    materialize_reads: bool,
    values: &mut u32,
    places: &mut Vec<raw::Place>,
    instructions: &mut Vec<raw::Instruction>,
) -> Option<()> {
    let mut begun = Vec::with_capacity(arm.aliases);
    let mut projected = BTreeMap::new();
    for step in arm.steps {
        match step {
            RootBorrowStep::Begin { id, place, access, at } => {
                let place =
                    materialize_root_borrow_place(&place, root_place, places, &mut projected)?;
                instructions.push(raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                        id,
                        place,
                        access,
                        span: at,
                    }),
                });
                begun.push(id);
            }
            RootBorrowStep::Read { id, ty, at } => {
                let value = raw::ValueId(*values);
                *values = values.checked_add(1)?;
                instructions.push(raw::Instruction {
                    result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
                    span: at,
                    kind: raw::InstructionKind::BorrowRead { borrow: id },
                });
                if materialize_reads {
                    let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
                    places.push(raw::Place {
                        id: place,
                        ty: ty.ir,
                        span: at,
                        kind: raw::PlaceKind::Local(place.0),
                    });
                    instructions.push(raw::Instruction {
                        result: None,
                        span: at,
                        kind: raw::InstructionKind::InitializePlace { place, value },
                    });
                }
            }
            RootBorrowStep::OwnerRead { place, at } => {
                let ty = place.ty;
                let place =
                    materialize_root_borrow_place(&place, root_place, places, &mut projected)?;
                let value = raw::ValueId(*values);
                *values = values.checked_add(1)?;
                instructions.push(raw::Instruction {
                    result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
                    span: at,
                    kind: raw::InstructionKind::CopyFromPlace { place },
                });
                if materialize_reads {
                    let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
                    places.push(raw::Place {
                        id: place,
                        ty: ty.ir,
                        span: at,
                        kind: raw::PlaceKind::Local(place.0),
                    });
                    instructions.push(raw::Instruction {
                        result: None,
                        span: at,
                        kind: raw::InstructionKind::InitializePlace { place, value },
                    });
                }
            }
            RootBorrowStep::Write { id, value, at } => {
                let value = emit_root_borrow_initializer(value, values, instructions)?;
                instructions.push(raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::BorrowWrite { borrow: id, value },
                });
            }
        }
    }
    for borrow in begun.into_iter().rev() {
        instructions.push(raw::Instruction {
            result: None,
            span: arm.block_exit,
            kind: raw::InstructionKind::EndBorrow { borrow },
        });
    }
    Some(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_root_borrow_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let plan = plan_private_root_borrow_function(
        input,
        module,
        function,
        declarations,
        graph,
        node_types,
        layouts,
        result,
        errors,
    )?;
    let RootBorrowPlan {
        root_ty,
        root_initializer,
        root_at,
        shape,
        aliases,
        reads,
        writes,
        return_at,
    } = plan;
    let mut values = 0_u32;
    let root_place = raw::PlaceId(0);
    let instruction_capacity = aliases
        .checked_mul(2)?
        .checked_add(reads.checked_mul(2)?)?
        .checked_add(writes.checked_mul(2)?)?
        .checked_add(4)?;
    let mut root_initialization = Vec::new();
    let root_value =
        emit_root_borrow_initializer(root_initializer, &mut values, &mut root_initialization)?;
    root_initialization.push(raw::Instruction {
        result: None,
        span: root_at,
        kind: raw::InstructionKind::InitializePlace { place: root_place, value: root_value },
    });
    let mut places = Vec::with_capacity(reads + 1);
    places.push(raw::Place {
        id: root_place,
        ty: root_ty.ir,
        span: root_at,
        kind: raw::PlaceKind::Local(0),
    });
    let cleanup = raw::CleanupPlanId(0);
    let blocks = match shape {
        RootBorrowShape::Straight(arm) => {
            let mut instructions = Vec::with_capacity(instruction_capacity);
            instructions.append(&mut root_initialization);
            emit_root_borrow_arm(
                arm,
                root_place,
                true,
                &mut values,
                &mut places,
                &mut instructions,
            )?;
            let returned = raw::ValueId(values);
            instructions.push(raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: returned,
                    ty: root_ty.ir,
                    span: return_at,
                }),
                span: return_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            });
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions,
                terminators: vec![raw::SpannedTerminator {
                    span: return_at,
                    kind: raw::Terminator::Return { value: returned, cleanup },
                }],
            }]
        }
        RootBorrowShape::Conditional { condition_at, branch_at, then_arm, else_arm } => {
            let mut entry = Vec::with_capacity(3);
            entry.append(&mut root_initialization);
            let condition = raw::ValueId(values);
            values = values.checked_add(1)?;
            entry.push(raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: condition,
                    ty: root_ty.ir,
                    span: condition_at,
                }),
                span: condition_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            });
            let mut then_instructions = Vec::with_capacity(instruction_capacity);
            emit_root_borrow_arm(
                then_arm,
                root_place,
                false,
                &mut values,
                &mut places,
                &mut then_instructions,
            )?;
            let mut else_instructions = Vec::with_capacity(instruction_capacity);
            emit_root_borrow_arm(
                else_arm,
                root_place,
                false,
                &mut values,
                &mut places,
                &mut else_instructions,
            )?;
            let returned = raw::ValueId(values);
            let join_instructions = vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: returned,
                    ty: root_ty.ir,
                    span: return_at,
                }),
                span: return_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            }];
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: entry,
                    terminators: vec![raw::SpannedTerminator {
                        span: branch_at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: raw::BlockId(1), arguments: Vec::new() },
                            when_false: raw::Edge {
                                target: raw::BlockId(2),
                                arguments: Vec::new(),
                            },
                        },
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: Vec::new(),
                    instructions: then_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: branch_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(3),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: Vec::new(),
                    instructions: else_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: branch_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(3),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(3),
                    parameters: Vec::new(),
                    instructions: join_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: return_at,
                        kind: raw::Terminator::Return { value: returned, cleanup },
                    }],
                },
            ]
        }
        RootBorrowShape::Loop { condition_at, loop_at, body } => {
            let entry = root_initialization;
            let condition = raw::ValueId(values);
            values = values.checked_add(1)?;
            let header = vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: condition,
                    ty: root_ty.ir,
                    span: condition_at,
                }),
                span: condition_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            }];
            let mut body_instructions = Vec::with_capacity(instruction_capacity);
            emit_root_borrow_arm(
                body,
                root_place,
                false,
                &mut values,
                &mut places,
                &mut body_instructions,
            )?;
            let returned = raw::ValueId(values);
            let exit = vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: returned,
                    ty: root_ty.ir,
                    span: return_at,
                }),
                span: return_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            }];
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: entry,
                    terminators: vec![raw::SpannedTerminator {
                        span: loop_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(1),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: Vec::new(),
                    instructions: header,
                    terminators: vec![raw::SpannedTerminator {
                        span: loop_at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: raw::BlockId(2), arguments: Vec::new() },
                            when_false: raw::Edge {
                                target: raw::BlockId(3),
                                arguments: Vec::new(),
                            },
                        },
                    }],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: Vec::new(),
                    instructions: body_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: loop_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(1),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(3),
                    parameters: Vec::new(),
                    instructions: exit,
                    terminators: vec![raw::SpannedTerminator {
                        span: return_at,
                        kind: raw::Terminator::Return { value: returned, cleanup },
                    }],
                },
            ]
        }
    };
    let _ = (aliases, reads, writes);
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters: Vec::new(),
        borrow_parameters: Vec::new(),
        result: root_ty.ir,
        places,
        blocks,
        cleanup_plans: vec![raw::CleanupPlan {
            id: cleanup,
            span: span(input.sources(), function.body.span),
            actions: Vec::new(),
        }],
    })
}

fn direct_reference_name(function: &syntax::RawFunctionSyntax, expression: u32) -> Option<&str> {
    let expression =
        usize::try_from(expression).ok().and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Reference { name } = &expression.kind else { return None };
    Some(&name.text)
}

fn is_direct_owned_root_borrow_candidate(
    file: &syntax::SourceUnit,
    function: &syntax::RawFunctionSyntax,
) -> bool {
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    let [root_local, nested, _] = root.statements.as_slice() else { return false };
    let Some(root_local) =
        usize::try_from(*root_local).ok().and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::LocalDeclaration { name: root_name, .. } = &root_local.kind else {
        return false;
    };
    let Some(nested) =
        usize::try_from(*nested).ok().and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::Block { block } = nested.kind else { return false };
    let Some(alias) = usize::try_from(block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
        .and_then(|block| block.statements.first())
        .and_then(|statement| usize::try_from(*statement).ok())
        .and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::LocalDeclaration { type_syntax, initializer, .. } = alias.kind else {
        return false;
    };
    let shared = usize::try_from(type_syntax)
        .ok()
        .and_then(|index| file.type_syntax().get(index))
        .is_some_and(|ty| matches!(ty.kind, RawTypeSyntaxKind::Borrow { .. }));
    let Some(initializer) =
        usize::try_from(initializer).ok().and_then(|index| function.body.expressions.get(index))
    else {
        return false;
    };
    let RawExpressionKind::Borrow { value, .. } = initializer.kind else { return false };
    shared && direct_reference_name(function, value) == Some(root_name.text.as_str())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn plan_private_owned_root_borrow_syntax<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<OwnedRootBorrowSyntax> {
    let at = span(input.sources(), function.span);
    if function.export_span.is_some() || !function.parameters.is_empty() {
        errors.at(
            "ZRYNA-M3017",
            at,
            "owned-root borrow reads require one private parameter-free function",
            "keep this internal checkpoint private and initialize its owner locally",
        );
        return None;
    }
    if result.is_copy()
        || !matches!(
            result.category,
            TypeCategory::String
                | TypeCategory::Vec
                | TypeCategory::Struct
                | TypeCategory::Enum
                | TypeCategory::FixedArray
        )
    {
        errors.at(
            "ZRYNA-M3017",
            at,
            "owned-root borrow reads require one supported non-Copy result",
            "return the exact String, Vec, Struct, Enum, or fixed-array root after lexical end",
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
            "owned-root borrow reads require one root local, one lexical block, and one final return",
            "declare the owner, read it through one nested shared-borrow block, then return it",
        );
        return None;
    };
    let root_local = usize::try_from(*root_local_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration { name: root_name, type_syntax: root_type, .. } =
        &root_local.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "owned-root borrow reads require one initialized local owner",
            "declare one exact owned root before the lexical block",
        );
        return None;
    };
    let root_ty = semantic_type(file, *root_type, module, declarations, graph, node_types, errors)?;
    if root_ty != result || root_ty.is_copy() {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "owned-root borrow result must exactly match the initialized non-Copy root",
            "give the local root and function result one exact owned type",
        );
        return None;
    }
    let nested_statement =
        usize::try_from(*nested_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Block { block: nested_block_id } = nested_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested_statement.span),
            "owned-root shared authority requires one explicit lexical block",
            "place the alias and its read-only operations inside one nested block",
        );
        return None;
    };
    let nested =
        usize::try_from(nested_block_id).ok().and_then(|index| function.body.blocks.get(index))?;
    let Some((alias_statement_id, read_statement_ids)) = nested.statements.split_first() else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested.span),
            "owned-root borrow block requires one shared alias and at least one read",
            "declare Borrow<Root> first, then perform one admitted read-only operation",
        );
        return None;
    };
    if read_statement_ids.is_empty() {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested.span),
            "owned-root borrow block requires at least one read-only operation",
            "clone, concatenate, or index through the shared alias before lexical end",
        );
        return None;
    }
    let alias_statement = usize::try_from(*alias_statement_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration {
        mutable: false,
        name: alias_name,
        type_syntax: alias_type,
        initializer: alias_initializer,
        ..
    } = &alias_statement.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), alias_statement.span),
            "owned-root borrow block must begin with one const shared alias",
            "declare `const alias: Borrow<Root> = borrow(root)`",
        );
        return None;
    };
    let declared =
        usize::try_from(*alias_type).ok().and_then(|index| file.type_syntax().get(index))?;
    let RawTypeSyntaxKind::Borrow { argument, .. } = declared.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), declared.span),
            "owned-root reads require shared Borrow authority",
            "use Borrow rather than BorrowMut for read-only owned access",
        );
        return None;
    };
    let referent = semantic_type(file, argument, module, declarations, graph, node_types, errors)?;
    if referent != root_ty {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), declared.span),
            "owned-root borrow alias has the wrong exact referent type",
            "declare Borrow with the exact whole-root owned type",
        );
        return None;
    }
    let alias_initializer_expression = usize::try_from(*alias_initializer)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Borrow { value: borrowed, .. } = alias_initializer_expression.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), alias_initializer_expression.span),
            "owned-root shared alias must be initialized with borrow(root)",
            "borrow the exact root directly without a projection or reborrow",
        );
        return None;
    };
    if direct_reference_name(function, borrowed) != Some(root_name.text.as_str()) {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), alias_initializer_expression.span),
            "owned-root shared alias must borrow the exact whole root",
            "remove projections, subobjects, and alternate operands from borrow(root)",
        );
        return None;
    }

    let mut flattened = Vec::with_capacity(read_statement_ids.len().saturating_add(2));
    flattened.push(*root_local_id);
    for statement_id in read_statement_ids {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        let RawStatementKind::LocalDeclaration { mutable: false, type_syntax, initializer, .. } =
            statement.kind
        else {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), statement.span),
                "owned-root borrow blocks admit only const read results",
                "remove moves, writes, replacement, drops, calls, and nested control flow",
            );
            return None;
        };
        let local_ty =
            semantic_type(file, type_syntax, module, declarations, graph, node_types, errors)?;
        let expression = usize::try_from(initializer)
            .ok()
            .and_then(|index| function.body.expressions.get(index))?;
        let admitted = match (&expression.kind, root_ty.category) {
            (RawExpressionKind::Clone { value, .. }, TypeCategory::String)
                if local_ty == root_ty
                    && direct_reference_name(function, *value)
                        == Some(alias_name.text.as_str()) =>
            {
                true
            }
            (RawExpressionKind::Call { callee, arguments, .. }, TypeCategory::String)
                if local_ty == root_ty
                    && callee.text == "concat"
                    && arguments.len() == 2
                    && arguments.iter().all(|argument| {
                        direct_reference_name(function, *argument) == Some(alias_name.text.as_str())
                    }) =>
            {
                true
            }
            (RawExpressionKind::Index { base, .. }, TypeCategory::Vec)
                if local_ty.is_copy()
                    && matches!(local_ty.category, TypeCategory::Bool | TypeCategory::I32)
                    && direct_reference_name(function, *base) == Some(alias_name.text.as_str()) =>
            {
                true
            }
            (RawExpressionKind::Clone { value, .. }, category)
                if matches!(
                    category,
                    TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
                ) && local_ty == root_ty
                    && direct_reference_name(function, *value)
                        == Some(alias_name.text.as_str()) =>
            {
                true
            }
            _ => false,
        };
        if !admitted {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), expression.span),
                "operation is outside whole owned-root shared reads",
                "use String clone/concat, exact Vec<bool/i32> indexing, or whole-root aggregate clone through the alias",
            );
            return None;
        }
        flattened.push(*statement_id);
    }

    let return_statement =
        usize::try_from(*return_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Return { value: returned, .. } = return_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), return_statement.span),
            "owned-root borrow reads require one final root return",
            "return the original owner after lexical end",
        );
        return None;
    };
    if direct_reference_name(function, returned) != Some(root_name.text.as_str()) {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), return_statement.span),
            "owned-root borrow aliases and read results cannot escape",
            "return the original root only after the shared alias ends",
        );
        return None;
    }
    flattened.push(*return_id);

    let mut synthetic = function.clone();
    let synthetic_root = usize::try_from(synthetic.body.root_block)
        .ok()
        .and_then(|index| synthetic.body.blocks.get_mut(index))?;
    synthetic_root.statements = flattened;
    let alias_statement_index = usize::try_from(*alias_statement_id).ok()?;
    synthetic.body.statements[alias_statement_index] = root_local.clone();
    for expression in &mut synthetic.body.expressions {
        if let RawExpressionKind::Reference { name } = &mut expression.kind
            && name.text == alias_name.text
        {
            name.text.clone_from(&root_name.text);
        }
    }
    Some(OwnedRootBorrowSyntax {
        synthetic,
        borrow_at: span(input.sources(), alias_statement.span),
        end_at: span(input.sources(), nested.close_brace_span),
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_owned_root_borrow_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let plan = plan_private_owned_root_borrow_syntax(
        input,
        module,
        function,
        declarations,
        graph,
        node_types,
        result,
        errors,
    )?;
    let mut lowered = lower_function_impl(
        input,
        module,
        declaration,
        &plan.synthetic,
        declarations,
        graph,
        node_types,
        layouts,
        catalog,
        errors,
    )?;
    let root_place =
        lowered.places.iter().find(|place| matches!(place.kind, raw::PlaceKind::Local(0)))?.id;
    let [block] = lowered.blocks.as_slice() else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), function.span),
            "owned-root borrow reads require one straight-line lowered block",
            "remove control flow and calls from the lexical read-only checkpoint",
        );
        return None;
    };
    let initialize = block.instructions.iter().position(|instruction| {
        matches!(
            instruction.kind,
            raw::InstructionKind::InitializePlace { place, .. } if place == root_place
        )
    })?;
    let consume = block.instructions.iter().rposition(|instruction| {
        matches!(
            instruction.kind,
            raw::InstructionKind::MoveFromPlace { place } if place == root_place
        )
    })?;
    if consume <= initialize {
        return None;
    }
    let [
        raw::SpannedTerminator {
            kind: raw::Terminator::Return { cleanup: return_cleanup, .. },
            ..
        },
    ] = block.terminators.as_slice()
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), function.span),
            "owned-root borrow reads require one final return",
            "return the original root after the lexical read-only block",
        );
        return None;
    };
    let return_cleanup_index = usize::try_from(return_cleanup.0).ok()?;
    let lexical_owned_drops = lowered
        .cleanup_plans
        .get(return_cleanup_index)?
        .actions
        .iter()
        .map(|action| match action {
            raw::DropAction::DropPlace(place) if *place != root_place => Some(*place),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let existing_transitions = lowered
        .blocks
        .iter()
        .try_fold(0_usize, |count, block| count.checked_add(block.instructions.len()))?;
    if let Some(limit) =
        owned_root_borrow_resource_violation(existing_transitions, lexical_owned_drops.len(), 0)
    {
        let label = match limit {
            RootBorrowBudgetLimit::Transitions => "ownership transitions",
            RootBorrowBudgetLimit::ActiveBorrows => "active borrows",
            _ => unreachable!("owned-root borrow adds only transition and active-borrow costs"),
        };
        errors.at(
            "ZRYNA-M3201",
            span(input.sources(), function.span),
            format!("owned-root borrow reads exceed the checked {label} budget"),
            "reduce repeated read-only operations inside the lexical borrow block",
        );
        return None;
    }
    lowered.cleanup_plans.get_mut(return_cleanup_index)?.actions.clear();
    let [block] = lowered.blocks.as_mut_slice() else {
        unreachable!("single-block shape checked before cleanup rewrite")
    };
    let borrow = raw::BorrowId(0);
    block.instructions.insert(
        initialize + 1,
        raw::Instruction {
            result: None,
            span: plan.borrow_at,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: borrow,
                place: root_place,
                access: raw::BorrowAccess::Shared,
                span: plan.borrow_at,
            }),
        },
    );
    let lexical_exit = lexical_owned_drops
        .into_iter()
        .map(|place| raw::Instruction {
            result: None,
            span: plan.end_at,
            kind: raw::InstructionKind::DropPlace { place },
        })
        .chain(std::iter::once(raw::Instruction {
            result: None,
            span: plan.end_at,
            kind: raw::InstructionKind::EndBorrow { borrow },
        }))
        .collect::<Vec<_>>();
    block.instructions.splice((consume + 1)..=consume, lexical_exit);
    Some(lowered)
}

#[allow(clippy::too_many_arguments)]
fn lower_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let result =
        semantic_type(file, function.result_type, module, declarations, graph, node_types, errors)?;
    let has_root_borrow_syntax = function.body.statements.iter().any(|statement| {
        let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind else {
            return false;
        };
        usize::try_from(type_syntax)
            .ok()
            .and_then(|index| file.type_syntax().get(index))
            .is_some_and(|ty| {
                matches!(
                    ty.kind,
                    RawTypeSyntaxKind::Borrow { .. } | RawTypeSyntaxKind::BorrowMut { .. }
                )
            })
    }) || function.body.expressions.iter().any(|expression| {
        matches!(
            expression.kind,
            RawExpressionKind::Borrow { .. } | RawExpressionKind::BorrowMut { .. }
        )
    });
    let owned_root_candidate =
        !result.is_copy() && is_direct_owned_root_borrow_candidate(file, function);
    if has_root_borrow_syntax && !owned_root_candidate {
        return lower_private_root_borrow_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            result,
            errors,
        );
    }
    if has_root_borrow_syntax {
        return lower_private_owned_root_borrow_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            catalog,
            result,
            errors,
        );
    }
    lower_function_impl(
        input,
        module,
        declaration,
        function,
        declarations,
        graph,
        node_types,
        layouts,
        catalog,
        errors,
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_function_impl<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let result =
        semantic_type(file, function.result_type, module, declarations, graph, node_types, errors)?;
    let has_vec_operation = function.body.expressions.iter().any(|expression| {
        matches!(
            expression.kind,
            RawExpressionKind::VecConstruction { .. } | RawExpressionKind::VecPush { .. }
        )
    });
    let has_vec_signature = catalog
        .modules
        .get(module)
        .and_then(|signatures| signatures.get(declaration))
        .and_then(Option::as_ref)
        .is_some_and(|signature| {
            signature.result.category == TypeCategory::Vec
                || signature
                    .parameters
                    .iter()
                    .any(|parameter| parameter.category == TypeCategory::Vec)
        });
    let has_vec_local = function.body.statements.iter().any(|statement| {
        let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind else {
            return false;
        };
        usize::try_from(type_syntax)
            .ok()
            .and_then(|index| file.type_syntax().get(index))
            .is_some_and(|ty| matches!(ty.kind, RawTypeSyntaxKind::Vec { .. }))
    });
    let terminal_owned_phi_candidate =
        is_terminal_owned_phi_candidate(function, result.category, has_vec_operation);
    if !terminal_owned_phi_candidate {
        verify_single_final_return(function, input.sources(), errors)?;
    }
    if result.category == TypeCategory::String
        && function.export_span.is_none()
        && !has_vec_operation
    {
        let diagnostics_before = errors.len();
        let lowered = lower_private_string_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            result,
            catalog,
            errors,
        );
        if lowered.is_none() && errors.len() == diagnostics_before {
            errors.at(
                "ZRYNA-M3012",
                span(input.sources(), function.span),
                "private String lowering rejected a source function without a specific diagnostic",
                "use only the exact supported private String forms",
            );
        }
        return lowered;
    }
    if function.export_span.is_none()
        && matches!(result.category, TypeCategory::Struct | TypeCategory::FixedArray)
        && is_private_owned_enum_payload_move_candidate(function)
    {
        return lower_private_owned_enum_payload_move_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            result,
            errors,
        );
    }
    if function.export_span.is_none()
        && !result.is_copy()
        && matches!(
            result.category,
            TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
        )
    {
        let diagnostics_before = errors.len();
        let lowered = lower_private_owned_aggregate_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            result,
            errors,
        );
        if lowered.is_none() && errors.len() == diagnostics_before {
            errors.at(
                "ZRYNA-M3016",
                span(input.sources(), function.span),
                "private owned aggregate lowering rejected a source function without a specific diagnostic",
                "use only the exact straight-line owned Struct/Enum/FixedArray forms",
            );
        }
        return lowered;
    }
    if function.export_span.is_none() && (has_vec_signature || has_vec_local || has_vec_operation) {
        let diagnostics_before = errors.len();
        let lowered = lower_private_vec_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            result,
            catalog,
            errors,
        );
        if lowered.is_none() && errors.len() == diagnostics_before {
            errors.at(
                "ZRYNA-M3013",
                span(input.sources(), function.span),
                "private Vec lowering rejected a source function without a specific diagnostic",
                "use only the exact supported private Vec forms",
            );
        }
        return lowered;
    }
    require_current_type_only_boundary(
        result,
        span(input.sources(), function.span),
        function.export_span.is_some(),
        errors,
    )?;
    let tail_match = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
        .and_then(|block| <&[u32; 1]>::try_from(block.statements.as_slice()).ok())
        .and_then(|[statement]| usize::try_from(*statement).ok())
        .and_then(|index| function.body.statements.get(index))
        .and_then(|statement| match statement.kind {
            RawStatementKind::Return { value, .. } => Some(value),
            _ => None,
        })
        .and_then(|value| usize::try_from(value).ok())
        .and_then(|index| function.body.expressions.get(index))
        .is_some_and(|expression| matches!(expression.kind, RawExpressionKind::Match { .. }));
    if tail_match {
        return lower_enum_match_function(
            input,
            module,
            declaration,
            function,
            declarations,
            graph,
            node_types,
            layouts,
            result,
            errors,
        );
    }
    let mut lowerer = FunctionLowerer {
        input,
        file,
        function,
        module,
        declarations,
        graph,
        node_types,
        layouts,
        catalog,
        errors,
        bindings: BTreeMap::new(),
        borrow_bindings: BTreeMap::new(),
        projections: BTreeMap::new(),
        places: Vec::new(),
        instructions: Vec::new(),
        cleanup_plans: Vec::new(),
        values: 0,
    };
    let signature = catalog
        .modules
        .get(module)
        .and_then(|signatures| signatures.get(declaration))
        .and_then(Option::as_ref)?;
    debug_assert_eq!(signature.result, result);
    debug_assert_eq!(signature.parameter_order.len(), function.parameters.len());
    let mut parameters = Vec::with_capacity(signature.parameters.len());
    let mut borrow_parameters = Vec::with_capacity(signature.borrow_parameters.len());
    for (parameter, order) in function.parameters.iter().zip(&signature.parameter_order) {
        let parameter_span = span(input.sources(), parameter.span);
        if lowerer.binding_name_exists(&parameter.name.text) {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!("parameter '{}' is declared more than once", parameter.name.text),
                "give each parameter one exact name",
            );
            continue;
        }
        match *order {
            FunctionParameterOrder::Value(index) => {
                let ty = *signature.parameters.get(usize::try_from(index).ok()?)?;
                require_current_type_only_boundary(
                    ty,
                    parameter_span,
                    function.export_span.is_some(),
                    lowerer.errors,
                )?;
                debug_assert_eq!(usize::try_from(index).ok(), Some(parameters.len()));
                let value = raw::ValueId(lowerer.values);
                lowerer.values += 1;
                parameters.push(raw::ValueDefinition {
                    id: value,
                    ty: ty.ir,
                    span: parameter_span,
                });
                let place =
                    lowerer.push_place(ty, parameter_span, raw::PlaceKind::Parameter(index));
                lowerer
                    .bindings
                    .insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
            }
            FunctionParameterOrder::Borrow(index) => {
                let descriptor = *signature.borrow_parameters.get(usize::try_from(index).ok()?)?;
                debug_assert_eq!(usize::try_from(index).ok(), Some(borrow_parameters.len()));
                let borrow = raw::BorrowId(index);
                borrow_parameters.push(raw::BorrowParameter {
                    id: borrow,
                    referent: descriptor.referent.ir,
                    access: descriptor.access,
                    span: descriptor.span,
                });
                lowerer.borrow_bindings.insert(
                    parameter.name.text.clone(),
                    BorrowBinding { ty: descriptor.referent, borrow, access: descriptor.access },
                );
            }
        }
    }
    let root =
        usize::try_from(function.body.root_block).ok().and_then(|i| function.body.blocks.get(i));
    let root = root?;
    let mut returned = None;
    for statement_id in &root.statements {
        let Some(statement) =
            usize::try_from(*statement_id).ok().and_then(|i| function.body.statements.get(i))
        else {
            continue;
        };
        match &statement.kind {
            RawStatementKind::LocalDeclaration {
                mutable, name, type_syntax, initializer, ..
            } => {
                let ty = semantic_type(
                    file,
                    *type_syntax,
                    module,
                    declarations,
                    graph,
                    node_types,
                    lowerer.errors,
                )?;
                require_current_type_only_boundary(
                    ty,
                    span(input.sources(), statement.span),
                    false,
                    lowerer.errors,
                )?;
                let value = lowerer.value(*initializer)?;
                lowerer.require_type(
                    ty,
                    value.0,
                    span(input.sources(), statement.span),
                    "local initializer",
                )?;
                let place = lowerer.push_place(
                    ty,
                    span(input.sources(), statement.span),
                    raw::PlaceKind::Local(u32::try_from(lowerer.bindings.len()).ok()?),
                );
                lowerer.emit(
                    None,
                    span(input.sources(), statement.span),
                    raw::InstructionKind::InitializePlace { place, value: value.1 },
                );
                if lowerer.binding_name_exists(&name.text) {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!(
                            "binding '{}' collides under portable ASCII case folding",
                            name.text
                        ),
                        "give every binding one portable case-insensitive unique name",
                    );
                } else {
                    lowerer
                        .bindings
                        .insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
                }
            }
            RawStatementKind::Assignment { target, value, .. } => {
                if let Some(binding) = lowerer.borrow_reference(*target) {
                    if binding.access != raw::BorrowAccess::Exclusive {
                        lowerer.errors.at(
                            "ZRYNA-M3016",
                            span(input.sources(), function.body.expressions[*target as usize].span),
                            "shared borrow parameters are read-only",
                            "write only through an exact BorrowMut parameter",
                        );
                        return None;
                    }
                    let value = lowerer.value(*value)?;
                    lowerer.require_type(
                        binding.ty,
                        value.0,
                        span(input.sources(), statement.span),
                        "borrow write",
                    )?;
                    lowerer.emit(
                        None,
                        span(input.sources(), statement.span),
                        raw::InstructionKind::BorrowWrite {
                            borrow: binding.borrow,
                            value: value.1,
                        },
                    );
                    continue;
                }
                let (target_ty, place, mutable) = lowerer.place(*target)?;
                if !mutable {
                    lowerer.errors.at(
                        "ZRYNA-M3007",
                        span(input.sources(), statement.span),
                        "assignment target is not rooted in a mutable local",
                        "declare the root with let mut before assigning",
                    );
                    return None;
                }
                let value = lowerer.value(*value)?;
                lowerer.require_type(
                    target_ty,
                    value.0,
                    span(input.sources(), statement.span),
                    "assignment",
                )?;
                lowerer.emit(
                    None,
                    span(input.sources(), statement.span),
                    raw::InstructionKind::ReplacePlace { place, value: value.1 },
                );
            }
            RawStatementKind::Return { value, .. } => {
                let value = lowerer.value(*value)?;
                lowerer.require_type(
                    result,
                    value.0,
                    span(input.sources(), statement.span),
                    "return",
                )?;
                returned = Some((value.1, span(input.sources(), statement.span)));
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3008",
                    span(input.sources(), statement.span),
                    "this statement form is outside deterministic aggregate M3",
                    "use local initialization, aggregate assignment, and one value return",
                );
                return None;
            }
        }
    }
    let Some((return_value, return_span)) = returned else {
        lowerer.errors.at(
            "ZRYNA-M3010",
            span(input.sources(), function.body.span),
            "function has no value return",
            "return one value of the exact declared type",
        );
        return None;
    };
    let cleanup = raw::CleanupPlanId(u32::try_from(lowerer.cleanup_plans.len()).ok()?);
    lowerer.cleanup_plans.push(raw::CleanupPlan {
        id: cleanup,
        span: span(input.sources(), function.body.span),
        actions: Vec::new(),
    });
    let block = raw::Block {
        id: raw::BlockId(0),
        parameters: Vec::new(),
        instructions: lowerer.instructions,
        terminators: vec![raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: return_value, cleanup },
        }],
    };
    let entry_export = if input.entry() == file.id() && function.export_span.is_some() {
        let aggregate_parameter = parameters.iter().any(|parameter| {
            layouts
                .types()
                .nth(usize::try_from(parameter.ty.0).unwrap_or(usize::MAX))
                .is_none_or(|ty| !matches!(ty.category(), TypeCategory::Bool | TypeCategory::I32))
        });
        if !matches!(result.category, TypeCategory::Bool | TypeCategory::I32) || aggregate_parameter
        {
            lowerer.errors.at(
                "ZRYNA-M3010",
                span(input.sources(), function.span),
                "public aggregate signatures are outside scalar ABI v1",
                "keep aggregate functions internal and export only bool/i32 signatures",
            );
        }
        Some(function.name.text.clone())
    } else {
        None
    };
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters,
        result: result.ir,
        places: lowerer.places,
        blocks: vec![block],
        cleanup_plans: lowerer.cleanup_plans,
    })
}

fn root_is_terminal_if(function: &syntax::RawFunctionSyntax) -> bool {
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

fn is_terminal_owned_phi_candidate(
    function: &syntax::RawFunctionSyntax,
    result: TypeCategory,
    has_vec_operation: bool,
) -> bool {
    function.export_span.is_none()
        && root_is_terminal_if(function)
        && ((result == TypeCategory::String && !has_vec_operation) || result == TypeCategory::Vec)
}

fn preflight_owned_loop_body(
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

fn preflight_owned_loop_exit(
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

fn preflight_owned_string_loop_skeleton(
    cfg: &OwnedCfgState,
    known_bytes: &mut BTreeMap<raw::PlaceId, Option<u64>>,
    normalize_mutation_facts: bool,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    if !cfg.preflight_skeleton(3, 4, at, errors) {
        return false;
    }
    if normalize_mutation_facts {
        for known in known_bytes.values_mut() {
            *known = None;
        }
    }
    true
}

fn terminal_owned_if(
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

struct OwnedPendingBlock {
    populated: bool,
    parameters: Vec<raw::ValueDefinition>,
    instructions: Vec<raw::Instruction>,
    terminator: Option<raw::SpannedTerminator>,
}

struct OwnedBlockArena {
    blocks: Vec<OwnedPendingBlock>,
}

impl OwnedBlockArena {
    fn empty() -> Self {
        Self { blocks: Vec::new() }
    }

    fn finish(self) -> Option<Vec<raw::Block>> {
        self.blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| {
                if !block.populated {
                    return None;
                }
                Some(raw::Block {
                    id: raw::BlockId(u32::try_from(index).ok()?),
                    parameters: block.parameters,
                    instructions: block.instructions,
                    terminators: vec![block.terminator?],
                })
            })
            .collect()
    }
}

// This ledger enforces per-function storage limits. Program-wide block/edge totals remain a
// separate composition check once owned CFG lowering can finalize more than one function graph.
struct OwnedCfgState {
    arena: OwnedBlockArena,
    current: Option<raw::BlockId>,
    incoming: Vec<usize>,
    edges: usize,
    transitions: usize,
    reserved_transitions: usize,
    value_types: Vec<raw::TypeId>,
    reserved_values: usize,
    function_parameters_open: bool,
}

impl OwnedCfgState {
    fn single_block(at: Span, errors: &mut Errors<'_>) -> Option<Self> {
        let mut state = Self {
            arena: OwnedBlockArena::empty(),
            current: None,
            incoming: Vec::new(),
            edges: 0,
            transitions: 0,
            reserved_transitions: 0,
            value_types: Vec::new(),
            reserved_values: 0,
            function_parameters_open: true,
        };
        let entry = state.reserve_block(at, errors)?;
        state.begin_block(entry, Vec::new(), at, errors)?;
        Some(state)
    }

    fn reserve_block(&mut self, at: Span, errors: &mut Errors<'_>) -> Option<raw::BlockId> {
        let Some(blocks) = self.arena.blocks.len().checked_add(1) else {
            Self::limit(OwnedCfgBudgetLimit::Blocks, at, errors);
            return None;
        };
        if owned_cfg_budget_violation(blocks, self.edges, self.transitions).is_some() {
            Self::limit(OwnedCfgBudgetLimit::Blocks, at, errors);
            return None;
        }
        let id = raw::BlockId(u32::try_from(self.arena.blocks.len()).ok()?);
        self.arena.blocks.push(OwnedPendingBlock {
            populated: false,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
        });
        self.incoming.push(0);
        Some(id)
    }

    fn preflight_skeleton(
        &self,
        additional_blocks: usize,
        additional_edges: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if resource_budget_violation(
            self.arena.blocks.len(),
            additional_blocks,
            ir::MAX_BLOCKS_PER_FUNCTION,
        ) {
            Self::limit(OwnedCfgBudgetLimit::Blocks, at, errors);
            return false;
        }
        if resource_budget_violation(self.edges, additional_edges, ir::MAX_CFG_EDGES_PER_FUNCTION) {
            Self::limit(OwnedCfgBudgetLimit::Edges, at, errors);
            return false;
        }
        true
    }

    fn seed_function_parameter(
        &mut self,
        parameter: &raw::ValueDefinition,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        if !self.function_parameters_open {
            Self::shape_error(
                parameter.span,
                "owned CFG function parameters must precede every emitted instruction",
                errors,
            );
            return None;
        }
        let types = self.prevalidate_value_definitions(std::slice::from_ref(parameter), errors)?;
        self.value_types.extend(types);
        Some(())
    }

    fn begin_block(
        &mut self,
        id: raw::BlockId,
        parameters: Vec<raw::ValueDefinition>,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        if self.current_block().is_some_and(|block| block.terminator.is_none()) {
            Self::shape_error(
                at,
                "cannot select another owned CFG block before terminating the current block",
                errors,
            );
            return None;
        }
        let Some(index) = usize::try_from(id.0).ok() else {
            Self::shape_error(at, "owned CFG block identity is not representable", errors);
            return None;
        };
        let next = self.arena.blocks.iter().position(|block| !block.populated);
        let Some(block) = self.arena.blocks.get(index) else {
            Self::shape_error(at, "owned CFG selected an unreserved block identity", errors);
            return None;
        };
        if block.populated || next != Some(index) {
            Self::shape_error(
                at,
                "owned CFG blocks must be populated once in canonical reservation order",
                errors,
            );
            return None;
        }
        let types = self.prevalidate_value_definitions(&parameters, errors)?;
        let block = self.arena.blocks.get_mut(index).expect("reserved block checked");
        block.populated = true;
        block.parameters = parameters;
        self.current = Some(id);
        self.value_types.extend(types);
        if id.0 != 0 {
            self.function_parameters_open = false;
        }
        Some(())
    }

    fn prevalidate_value_definitions(
        &mut self,
        definitions: &[raw::ValueDefinition],
        errors: &mut Errors<'_>,
    ) -> Option<Vec<raw::TypeId>> {
        let Some(capacity_count) = self.value_types.len().checked_add(self.reserved_values) else {
            let at = definitions.first()?.span;
            Self::limit(OwnedCfgBudgetLimit::Values, at, errors);
            return None;
        };
        if owned_value_budget_violation(capacity_count, definitions.len()) {
            let trigger = ir::MAX_VALUES_PER_FUNCTION
                .checked_sub(capacity_count)
                .and_then(|remaining| definitions.get(remaining))
                .or_else(|| definitions.first())?;
            Self::limit(OwnedCfgBudgetLimit::Values, trigger.span, errors);
            return None;
        }
        let mut count = self.value_types.len();
        let mut types = Vec::with_capacity(definitions.len());
        for definition in definitions {
            let Some(expected) = dense_owned_value_id(count) else {
                Self::limit(OwnedCfgBudgetLimit::Values, definition.span, errors);
                return None;
            };
            if definition.id != expected {
                Self::shape_error(
                    definition.span,
                    "owned CFG value definitions break dense global value order",
                    errors,
                );
                return None;
            }
            count = count.checked_add(1)?;
            types.push(definition.ty);
        }
        Some(types)
    }

    fn reserve_values(
        &mut self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        let current = self.value_types.len().checked_add(self.reserved_values);
        if current.is_none_or(|current| owned_value_budget_violation(current, additional)) {
            Self::limit(OwnedCfgBudgetLimit::Values, at, errors);
            return None;
        }
        self.reserved_values = self.reserved_values.checked_add(additional)?;
        Some(())
    }

    fn release_values(&mut self, additional: usize) {
        self.reserved_values =
            self.reserved_values.checked_sub(additional).expect("reserved owned CFG values");
    }

    fn current_block(&self) -> Option<&OwnedPendingBlock> {
        self.current
            .and_then(|id| usize::try_from(id.0).ok())
            .and_then(|index| self.arena.blocks.get(index))
    }

    fn current_mut(&mut self) -> Option<&mut OwnedPendingBlock> {
        self.current
            .and_then(|id| usize::try_from(id.0).ok())
            .and_then(|index| self.arena.blocks.get_mut(index))
    }

    fn emit(&mut self, instruction: raw::Instruction, errors: &mut Errors<'_>) -> bool {
        if !self.preflight_emit(&instruction, errors) {
            return false;
        }
        let transitions = self.transitions + 1;
        let result_type = instruction.result.as_ref().map(|result| result.ty);
        self.current_mut().expect("current block checked").instructions.push(instruction);
        self.transitions = transitions;
        self.value_types.extend(result_type);
        self.function_parameters_open = false;
        true
    }

    fn preflight_transition(&mut self, at: Span, errors: &mut Errors<'_>) -> bool {
        self.preflight_transitions(1, at, errors)
    }

    fn preflight_transitions(
        &mut self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if self.current_mut().is_none_or(|block| block.terminator.is_some()) {
            Self::shape_error(
                at,
                "owned CFG emission requires one selected unterminated block",
                errors,
            );
            return false;
        }
        let Some(transitions) = self
            .transitions
            .checked_add(self.reserved_transitions)
            .and_then(|current| current.checked_add(additional))
        else {
            Self::limit(OwnedCfgBudgetLimit::Transitions, at, errors);
            return false;
        };
        if owned_cfg_budget_violation(self.arena.blocks.len(), self.edges, transitions).is_some() {
            Self::limit(OwnedCfgBudgetLimit::Transitions, at, errors);
            return false;
        }
        true
    }

    fn reserve_transitions(
        &mut self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        if !self.preflight_transitions(additional, at, errors) {
            return None;
        }
        self.reserved_transitions = self.reserved_transitions.checked_add(additional)?;
        Some(())
    }

    fn release_transitions(&mut self, additional: usize) {
        self.reserved_transitions = self
            .reserved_transitions
            .checked_sub(additional)
            .expect("reserved owned CFG transitions");
    }

    fn preflight_emit(&mut self, instruction: &raw::Instruction, errors: &mut Errors<'_>) -> bool {
        let at = instruction.span;
        if !self.preflight_transition(at, errors) {
            return false;
        }
        if let Some(result) = &instruction.result {
            self.prevalidate_value_definitions(std::slice::from_ref(result), errors).is_some()
        } else {
            true
        }
    }

    fn terminate(&mut self, terminator: raw::SpannedTerminator, errors: &mut Errors<'_>) -> bool {
        let at = terminator.span;
        if self.current_mut().is_none_or(|block| block.terminator.is_some()) {
            Self::shape_error(
                at,
                "owned CFG termination requires one selected unterminated block",
                errors,
            );
            return false;
        }
        let Some(targets) = self.validate_targets(&terminator.kind, at, errors) else {
            return false;
        };
        let additional = match &terminator.kind {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
            raw::Terminator::Jump(_) => 1,
            raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
            raw::Terminator::EnumMatch { arms, .. } => arms.len(),
        };
        let Some(edges) = self.edges.checked_add(additional) else {
            Self::limit(OwnedCfgBudgetLimit::Edges, at, errors);
            return false;
        };
        if owned_cfg_budget_violation(self.arena.blocks.len(), edges, self.transitions).is_some() {
            Self::limit(OwnedCfgBudgetLimit::Edges, at, errors);
            return false;
        }
        for target in targets {
            let index = usize::try_from(target.0).expect("reserved target index");
            self.incoming[index] = self.incoming[index].saturating_add(1);
        }
        self.current_mut().expect("current block checked").terminator = Some(terminator);
        self.edges = edges;
        true
    }

    fn validate_targets(
        &mut self,
        terminator: &raw::Terminator,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Vec<raw::BlockId>> {
        let targets = match terminator {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
            raw::Terminator::Jump(edge) => vec![edge.target],
            raw::Terminator::Branch { when_true, when_false, .. } => {
                vec![when_true.target, when_false.target]
            }
            raw::Terminator::EnumMatch { arms, .. } => {
                arms.iter().map(|arm| arm.edge.target).collect()
            }
            raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                vec![success.target, expired.target]
            }
        };
        let Some(current) = self.current else {
            Self::shape_error(at, "owned CFG has no current block for its terminator", errors);
            return None;
        };
        for target in targets {
            let reserved = usize::try_from(target.0)
                .ok()
                .is_some_and(|index| self.arena.blocks.get(index).is_some());
            if !reserved || target.0 == 0 {
                Self::shape_error(
                    at,
                    "owned CFG successor must be a reserved non-entry block",
                    errors,
                );
                return None;
            }
        }
        let _ = current;
        Some(match terminator {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
            raw::Terminator::Jump(edge) => vec![edge.target],
            raw::Terminator::Branch { when_true, when_false, .. } => {
                vec![when_true.target, when_false.target]
            }
            raw::Terminator::EnumMatch { arms, .. } => {
                arms.iter().map(|arm| arm.edge.target).collect()
            }
            raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                vec![success.target, expired.target]
            }
        })
    }

    fn limit(limit: OwnedCfgBudgetLimit, at: Span, errors: &mut Errors<'_>) {
        let (label, maximum, guidance) = match limit {
            OwnedCfgBudgetLimit::Blocks => (
                "owned CFG blocks",
                ir::MAX_BLOCKS_PER_FUNCTION,
                "reduce nested owned control-flow blocks",
            ),
            OwnedCfgBudgetLimit::Edges => (
                "owned CFG edges",
                ir::MAX_CFG_EDGES_PER_FUNCTION,
                "reduce owned branch and loop edges",
            ),
            OwnedCfgBudgetLimit::Transitions => (
                "owned CFG transitions",
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                "reduce owned operations before control-flow lowering",
            ),
            OwnedCfgBudgetLimit::Values => (
                "owned CFG values",
                ir::MAX_VALUES_PER_FUNCTION,
                "reduce owned function parameters, block parameters, and result-producing expressions",
            ),
        };
        errors.at(
            "ZRYNA-M3201",
            at,
            format!("{label} exceed the per-function M3 limit of {maximum}"),
            guidance,
        );
    }

    fn shape_error(at: Span, message: &'static str, errors: &mut Errors<'_>) {
        errors.at(
            "ZRYNA-M3015",
            at,
            message,
            "reserve blocks first, populate them once in order, and terminate each block exactly once",
        );
    }

    fn finish(self, at: Span, errors: &mut Errors<'_>) -> Option<Vec<raw::Block>> {
        if self.arena.blocks.is_empty() {
            Self::shape_error(at, "owned CFG has no entry block", errors);
            return None;
        }
        if self.arena.blocks.iter().any(|block| !block.populated) {
            Self::shape_error(at, "owned CFG contains an unpopulated reserved block", errors);
            return None;
        }
        if self.arena.blocks.iter().any(|block| block.terminator.is_none()) {
            Self::shape_error(at, "owned CFG contains an unterminated block", errors);
            return None;
        }
        if self.incoming.iter().skip(1).any(|incoming| *incoming == 0) {
            Self::shape_error(
                at,
                "owned CFG contains a non-entry block with no predecessor",
                errors,
            );
            return None;
        }
        let mut reachable = vec![false; self.arena.blocks.len()];
        reachable[0] = true;
        let mut work = vec![0_usize];
        while let Some(index) = work.pop() {
            let terminator = &self.arena.blocks[index]
                .terminator
                .as_ref()
                .expect("terminated blocks checked")
                .kind;
            let targets = match terminator {
                raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
                raw::Terminator::Jump(edge) => vec![edge.target],
                raw::Terminator::Branch { when_true, when_false, .. } => {
                    vec![when_true.target, when_false.target]
                }
                raw::Terminator::EnumMatch { arms, .. } => {
                    arms.iter().map(|arm| arm.edge.target).collect()
                }
                raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                    vec![success.target, expired.target]
                }
            };
            for target in targets {
                let target = usize::try_from(target.0).expect("reserved target index");
                if !reachable[target] {
                    reachable[target] = true;
                    work.push(target);
                }
            }
        }
        if reachable.iter().any(|reachable| !reachable) {
            Self::shape_error(at, "owned CFG contains blocks disconnected from its entry", errors);
            return None;
        }
        for block in &self.arena.blocks {
            let terminator = block.terminator.as_ref().expect("terminated blocks checked");
            let edges = match &terminator.kind {
                raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
                raw::Terminator::Jump(edge) => vec![edge],
                raw::Terminator::Branch { when_true, when_false, .. } => {
                    vec![when_true, when_false]
                }
                raw::Terminator::EnumMatch { arms, .. } => {
                    arms.iter().map(|arm| &arm.edge).collect()
                }
                raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                    vec![success, expired]
                }
            };
            for edge in edges {
                let target = &self.arena.blocks
                    [usize::try_from(edge.target.0).expect("reserved target index")];
                if edge.arguments.len() != target.parameters.len()
                    || edge.arguments.iter().zip(&target.parameters).any(|(argument, parameter)| {
                        usize::try_from(argument.0)
                            .ok()
                            .and_then(|index| self.value_types.get(index))
                            != Some(&parameter.ty)
                    })
                {
                    Self::shape_error(
                        at,
                        "owned CFG edge arguments do not match the populated target signature",
                        errors,
                    );
                    return None;
                }
            }
        }
        Some(self.arena.finish().expect("populated dense blocks checked"))
    }
}

struct PrivateStringLowerer<'a, 'f, 'e> {
    input: SemanticInput<'a>,
    function: &'f syntax::RawFunctionSyntax,
    module: usize,
    ty: Ty,
    catalog: &'a FunctionCatalog,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    places: Vec<raw::Place>,
    reserved_places: usize,
    cfg: OwnedCfgState,
    cleanup_plans: Vec<raw::CleanupPlan>,
    cleanup_actions: usize,
    reserved_cleanup_plans: usize,
    reserved_cleanup_actions: usize,
    owners: OwnerState,
    known_bytes: BTreeMap<raw::PlaceId, Option<u64>>,
    next_value: u32,
    next_local: u32,
}

impl PrivateStringLowerer<'_, '_, '_> {
    fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

    fn preparation_estimate(
        &mut self,
        id: u32,
        context: OwnedStringEstimateContext,
        at: Span,
    ) -> Option<OwnedStringEstimateOutcome> {
        match estimate_owned_string_expression(
            self.function,
            &self.bindings,
            &self.owners,
            self.ty,
            id,
            self.owners.pending().len(),
            context,
        ) {
            Ok(estimate) => Some(OwnedStringEstimateOutcome::Estimated(estimate)),
            Err(OwnedStringEstimateError::Unsupported) => {
                Some(OwnedStringEstimateOutcome::Unsupported)
            }
            Err(OwnedStringEstimateError::Unavailable(reference)) => {
                self.errors.at(
                    "ZRYNA-M3011",
                    span(self.input.sources(), reference),
                    "String source owner is no longer available",
                    "move each owned String value at most once",
                );
                None
            }
            Err(OwnedStringEstimateError::Overflow) => {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "recursive owned String preparation overflows its checked resource estimate",
                    "reduce nested String-producing expressions",
                );
                None
            }
        }
    }

    fn preflight_string_expression(&mut self, id: u32, at: Span) -> bool {
        let Some(outcome) = self.preparation_estimate(id, OwnedStringEstimateContext::Value, at)
        else {
            return false;
        };
        let OwnedStringEstimateOutcome::Estimated(estimate) = outcome else {
            return true;
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans: self.reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions: self.reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
    }

    fn preflight_place(&mut self, at: Span) -> bool {
        preflight_owned_place_capacity_with_reserved(
            self.places.len(),
            self.reserved_places,
            1,
            at,
            self.errors,
        )
    }

    fn reserve_local_place(&mut self, at: Span) -> bool {
        if !self.preflight_place(at) {
            return false;
        }
        self.reserved_places += 1;
        true
    }

    fn release_local_place(&mut self) {
        self.reserved_places = self.reserved_places.checked_sub(1).expect("reserved local place");
    }

    fn reserve_local_commit(&mut self, at: Span) -> bool {
        if !self.reserve_local_place(at) {
            return false;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            return false;
        }
        true
    }

    fn release_local_commit(&mut self) {
        self.cfg.release_transitions(1);
        self.release_local_place();
    }

    fn reserve_cleanup_capacity(&mut self, actions: usize, at: Span) -> bool {
        if resource_budget_violation(
            self.cleanup_plans.len(),
            self.reserved_cleanup_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            self.cleanup_actions,
            self.reserved_cleanup_actions.saturating_add(actions),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "reserved String cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live Strings or fallible String operations",
            );
            return false;
        }
        self.reserved_cleanup_plans += 1;
        self.reserved_cleanup_actions += actions;
        true
    }

    fn release_cleanup_capacity(&mut self, actions: usize) {
        self.reserved_cleanup_plans =
            self.reserved_cleanup_plans.checked_sub(1).expect("reserved cleanup plan");
        self.reserved_cleanup_actions =
            self.reserved_cleanup_actions.checked_sub(actions).expect("reserved cleanup actions");
    }

    fn push_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        if resource_budget_violation(
            self.cleanup_plans.len(),
            self.reserved_cleanup_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup sites exceed the per-function M3 limit of {}",
                    ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                ),
                "reduce fallible private String operations",
            );
            return None;
        }
        let pending = self.owners.pending();
        let excluded_present = excluded.is_some_and(|place| self.owners.contains(place));
        let action_count = pending.len() - usize::from(excluded_present);
        if resource_budget_violation(
            self.cleanup_actions,
            self.reserved_cleanup_actions.saturating_add(action_count),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce simultaneously live Strings or fallible private String operations",
            );
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).unwrap_or(u32::MAX));
        let actions = self
            .owners
            .pending()
            .iter()
            .rev()
            .copied()
            .filter(|place| Some(*place) != excluded)
            .map(raw::DropAction::DropPlace)
            .collect();
        self.cleanup_plans.push(raw::CleanupPlan { id, span: at, actions });
        self.cleanup_actions += action_count;
        Some(id)
    }

    fn push_instruction_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        if !self.cfg.preflight_transition(at, self.errors) {
            return None;
        }
        self.push_cleanup(at, excluded)
    }

    fn push_temporary(
        &mut self,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<(raw::ValueId, raw::PlaceId)> {
        if !self.preflight_place(at) {
            return None;
        }
        let value = raw::ValueId(self.next_value);
        let place = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
        let instruction = raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: self.ty.ir, span: at }),
            span: at,
            kind,
        };
        if !self.cfg.preflight_emit(&instruction, self.errors) {
            return None;
        }
        self.next_value += 1;
        self.places.push(raw::Place {
            id: place,
            ty: self.ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        if !self.cfg.emit(instruction, self.errors) {
            return None;
        }
        let _ = self.owners.register(value, place);
        Some((value, place))
    }

    fn push_copy_value(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<raw::ValueId> {
        let value = raw::ValueId(self.next_value);
        let instruction = raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind,
        };
        if !self.cfg.preflight_emit(&instruction, self.errors) {
            return None;
        }
        self.next_value = self.next_value.checked_add(1)?;
        self.cfg.emit(instruction, self.errors).then_some(value)
    }

    fn condition(&mut self, id: u32, bool_ty: Ty) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        debug_assert_eq!(bool_ty.category, TypeCategory::Bool);
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } => {
                self.push_copy_value(bool_ty, at, raw::InstructionKind::BoolLiteral(value))
            }
            RawExpressionKind::Reference { name } => {
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("binding '{}' is not declared in this function", name.text),
                        "reference one exact preceding bool binding",
                    );
                    return None;
                };
                if binding.ty != bool_ty {
                    self.errors.at(
                        "ZRYNA-M3012",
                        span(self.input.sources(), name.span),
                        "owned String control-flow condition must have exact bool type",
                        "use a bool literal or preceding exact bool binding",
                    );
                    return None;
                }
                self.push_copy_value(
                    bool_ty,
                    at,
                    raw::InstructionKind::CopyFromPlace { place: binding.place },
                )
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "owned String control-flow condition must be a bool literal or reference",
                    "use one exact Copy bool condition",
                );
                None
            }
        }
    }

    fn lower_string_local(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let RawStatementKind::LocalDeclaration { mutable, name, type_syntax, initializer, .. } =
            &statement.kind
        else {
            return None;
        };
        let local_ty = semantic_type(
            types.file,
            *type_syntax,
            self.module,
            types.declarations,
            types.graph,
            types.node_types,
            self.errors,
        )?;
        if local_ty != self.ty {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), statement.span),
                "private String lowering requires exact typed String locals",
                "declare each owned local as String",
            );
            return None;
        }
        if self.bindings.keys().any(|existing| existing.eq_ignore_ascii_case(&name.text)) {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' collides under portable ASCII case folding", name.text),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        let at = span(self.input.sources(), statement.span);
        if !self.reserve_local_commit(at) {
            return None;
        }
        let Some((value, temporary)) = self.value(*initializer) else {
            self.release_local_commit();
            return None;
        };
        let local = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let initialize = raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::InitializePlace { place: local, value },
        };
        self.release_local_commit();
        self.places.push(raw::Place {
            id: local,
            ty: self.ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local = self.next_local.checked_add(1)?;
        if !self.cfg.emit(initialize, self.errors) {
            return None;
        }
        let Some(delta) = self.owners.rename(value, local) else {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "String local initializer has no available owner",
                "initialize the local from one available String value",
            );
            return None;
        };
        debug_assert_eq!(delta, OwnerDelta::Renamed { from: temporary, to: local });
        apply_owner_delta(&mut self.known_bytes, delta);
        self.bindings
            .insert(name.text.clone(), Binding { ty: self.ty, place: local, mutable: *mutable });
        Some(())
    }

    fn restore_branch_scope(&mut self, incoming: &OwnedStringBranchState, at: Span) -> Option<()> {
        if !self.owners.pending().starts_with(incoming.owners.pending()) {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "owned String branch changed an incoming owner",
                "leave every outer String unchanged on both branch paths",
            );
            return None;
        }
        let branch_owners = self.owners.pending()[incoming.owners.pending().len()..].to_vec();
        if resource_budget_violation(
            self.cleanup_actions,
            branch_owners.len(),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce branch-local owned Strings or fallible String operations",
            );
            return None;
        }
        if !self.cfg.preflight_transitions(branch_owners.len(), at, self.errors) {
            return None;
        }
        for owner in branch_owners.into_iter().rev() {
            let drop = raw::Instruction {
                result: None,
                span: at,
                kind: raw::InstructionKind::DropPlace { place: owner },
            };
            if !self.cfg.preflight_emit(&drop, self.errors) || !self.cfg.emit(drop, self.errors) {
                return None;
            }
            self.cleanup_actions = self.cleanup_actions.checked_add(1)?;
            let delta = self.owners.consume_owner(owner)?;
            apply_owner_delta(&mut self.known_bytes, delta);
        }
        self.bindings = incoming.bindings.clone();
        if self.owners != incoming.owners || self.known_bytes != incoming.known_bytes {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "owned String branch does not restore the incoming ownership state",
                "drop branch locals and leave every outer String unchanged",
            );
            return None;
        }
        Some(())
    }

    fn drop_non_carried(&mut self, carried: raw::PlaceId, at: Span) -> Option<()> {
        let dropped = self
            .owners
            .pending()
            .iter()
            .copied()
            .filter(|owner| *owner != carried)
            .collect::<Vec<_>>();
        if resource_budget_violation(
            self.cleanup_actions,
            dropped.len(),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "terminal String arm cleanup exceeds the per-function M3 limit",
                "reduce owned temporaries in the returning branch expression",
            );
            return None;
        }
        if !self.cfg.preflight_transitions(dropped.len(), at, self.errors) {
            return None;
        }
        for owner in dropped.into_iter().rev() {
            if !self.cfg.emit(
                raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::DropPlace { place: owner },
                },
                self.errors,
            ) {
                return None;
            }
            self.cleanup_actions = self.cleanup_actions.checked_add(1)?;
            let delta = self.owners.consume_owner(owner)?;
            apply_owner_delta(&mut self.known_bytes, delta);
        }
        Some(())
    }

    fn lower_branch(
        &mut self,
        block_id: Option<u32>,
        incoming: &OwnedStringBranchState,
        at: Span,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let mut scope_span = at;
        if let Some(block_id) = block_id {
            let block = usize::try_from(block_id)
                .ok()
                .and_then(|index| self.function.body.blocks.get(index))?
                .clone();
            scope_span = span(self.input.sources(), block.span);
            for statement_id in block.statements {
                let statement = usize::try_from(statement_id)
                    .ok()
                    .and_then(|index| self.function.body.statements.get(index))?
                    .clone();
                if let RawStatementKind::LocalDeclaration { initializer, .. } = statement.kind {
                    if let Some(reference_span) = self.incoming_move_span(initializer, incoming) {
                        self.errors.at(
                            "ZRYNA-M3015",
                            span(self.input.sources(), reference_span),
                            "owned String loop or branch cannot move an incoming owner",
                            "clone the incoming String or construct the local independently",
                        );
                        return None;
                    }
                    self.lower_string_local(&statement, types)?;
                } else {
                    self.errors.at(
                        "ZRYNA-M3016",
                        span(self.input.sources(), statement.span),
                        "this branch statement is outside the bounded owned String if slice",
                        "use branch-local typed String declarations only",
                    );
                    return None;
                }
            }
        }
        self.restore_branch_scope(incoming, scope_span)
    }

    fn incoming_move_span(
        &self,
        id: u32,
        incoming: &OwnedStringBranchState,
    ) -> Option<UntrustedSpan> {
        self.incoming_move_span_in_context(id, incoming, true)
    }

    fn incoming_move_span_in_context(
        &self,
        id: u32,
        incoming: &OwnedStringBranchState,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && incoming.bindings.get(&name.text).is_some_and(|binding| {
                        incoming.owners.contains(binding.place) && binding.ty == self.ty
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.incoming_move_span_in_context(*value, incoming, false)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                arguments.iter().find_map(|argument| {
                    self.incoming_move_span_in_context(*argument, incoming, false)
                })
            }
            RawExpressionKind::Call { arguments, .. } => arguments
                .iter()
                .find_map(|argument| self.incoming_move_span_in_context(*argument, incoming, true)),
            _ => None,
        }
    }

    fn target_consumption_span(
        &self,
        id: u32,
        target: raw::PlaceId,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && self.bindings.get(&name.text).is_some_and(|binding| {
                        binding.place == target && self.owners.contains(binding.place)
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.target_consumption_span(*value, target, false)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                arguments
                    .iter()
                    .find_map(|argument| self.target_consumption_span(*argument, target, false))
            }
            RawExpressionKind::Call { arguments, .. } => arguments
                .iter()
                .find_map(|argument| self.target_consumption_span(*argument, target, true)),
            _ => None,
        }
    }

    fn reserve_loop_drop_actions(&mut self, actions: usize, at: Span) -> bool {
        if resource_budget_violation(
            self.cleanup_actions,
            self.reserved_cleanup_actions.saturating_add(actions),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "reserved String loop cleanup exceeds the per-function M3 limit",
                "reduce temporary read operands in the loop replacement",
            );
            return false;
        }
        self.reserved_cleanup_actions += actions;
        true
    }

    fn release_loop_drop_actions(&mut self, actions: usize) {
        self.reserved_cleanup_actions =
            self.reserved_cleanup_actions.checked_sub(actions).expect("reserved loop drop actions");
    }

    fn commit_loop_replacement(
        &mut self,
        binding: &Binding,
        prepared_value: raw::ValueId,
        prepared_owner: raw::PlaceId,
        drop_count: usize,
        incoming: &OwnedStringBranchState,
        at: Span,
    ) -> Option<()> {
        if !self.cfg.emit(
            raw::Instruction {
                result: None,
                span: at,
                kind: raw::InstructionKind::ReplacePlace {
                    place: binding.place,
                    value: prepared_value,
                },
            },
            self.errors,
        ) {
            return None;
        }
        let delta = self.owners.replace(prepared_value, binding.place)?;
        debug_assert_eq!(
            delta,
            OwnerDelta::Replaced { prepared: prepared_owner, target: binding.place }
        );
        apply_owner_delta(&mut self.known_bytes, delta);
        self.known_bytes.insert(binding.place, None);
        let temporary_reads = self
            .owners
            .pending()
            .iter()
            .copied()
            .filter(|owner| *owner != binding.place)
            .collect::<Vec<_>>();
        debug_assert_eq!(temporary_reads.len(), drop_count);
        for owner in temporary_reads.into_iter().rev() {
            if !self.cfg.emit(
                raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::DropPlace { place: owner },
                },
                self.errors,
            ) {
                return None;
            }
            self.cleanup_actions = self.cleanup_actions.checked_add(1)?;
            let delta = self.owners.consume_owner(owner)?;
            apply_owner_delta(&mut self.known_bytes, delta);
        }
        self.bindings = incoming.bindings.clone();
        if self.owners != incoming.owners || self.known_bytes != incoming.known_bytes {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "String loop replacement does not restore the exact header owner state",
                "retain the same outer String place across every backedge",
            );
            return None;
        }
        Some(())
    }

    fn lower_loop_assignment(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        incoming: &OwnedStringBranchState,
    ) -> Option<()> {
        let RawStatementKind::Assignment { target, value, .. } = statement.kind else {
            return None;
        };
        let target_expression = self.expression(target)?.clone();
        let RawExpressionKind::Reference { name } = target_expression.kind else {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), target_expression.span),
                "String loop replacement requires one root local target",
                "assign only to the single mutable outer String",
            );
            return None;
        };
        let Some(binding) = incoming.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                "String loop replacement target is not an incoming binding",
                "assign only to the single mutable outer String",
            );
            return None;
        };
        if binding.ty != self.ty || !binding.mutable || !incoming.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3015",
                span(self.input.sources(), name.span),
                "String loop replacement target is immutable, unavailable, or has the wrong type",
                "assign only to the single mutable available outer String",
            );
            return None;
        }
        if let Some(reference_span) = self.incoming_move_span(value, incoming) {
            self.errors.at(
                "ZRYNA-M3015",
                span(self.input.sources(), reference_span),
                "String loop replacement cannot consume an incoming owner while preparing its replacement",
                "prepare an independent String or explicitly clone the target",
            );
            return None;
        }
        let at = span(self.input.sources(), statement.span);
        let outcome = self.preparation_estimate(value, OwnedStringEstimateContext::Value, at)?;
        let OwnedStringEstimateOutcome::Estimated(estimate) = outcome else {
            self.errors.at(
                "ZRYNA-M3012",
                at,
                "String loop replacement is outside checked recursive preparation",
                "use an admitted String literal, clone, concat, or private String call",
            );
            return None;
        };
        let growth = estimate.end_pending.checked_sub(self.owners.pending().len())?;
        let drop_count = growth.checked_sub(1)?;
        let transitions = drop_count.checked_add(1)?;
        if !reserve_owned_commit_transitions(&mut self.cfg, transitions, at, self.errors) {
            return None;
        }
        if !self.reserve_loop_drop_actions(drop_count, at) {
            release_owned_commit_transitions(&mut self.cfg, transitions);
            return None;
        }
        let Some((prepared_value, prepared_owner)) = self.value(value) else {
            self.release_loop_drop_actions(drop_count);
            release_owned_commit_transitions(&mut self.cfg, transitions);
            return None;
        };
        self.release_loop_drop_actions(drop_count);
        release_owned_commit_transitions(&mut self.cfg, transitions);
        self.commit_loop_replacement(
            &binding,
            prepared_value,
            prepared_owner,
            drop_count,
            incoming,
            at,
        )
    }

    fn readable_reference(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
    ) -> Option<(raw::PlaceId, Option<u64>)> {
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' is not declared in this function", name.text),
                "reference one exact preceding String local",
            );
            return None;
        };
        if binding.ty != self.ty || !self.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3011",
                span(self.input.sources(), name.span),
                format!("String binding '{}' was already moved", name.text),
                "use or move each owned String binding only while it remains available",
            );
            return None;
        }
        Some((binding.place, self.known_bytes.get(&binding.place).copied().flatten()))
    }

    fn place_for_read(&mut self, id: u32) -> Option<(raw::PlaceId, Option<u64>)> {
        let expression = self.expression(id)?.clone();
        if let RawExpressionKind::Reference { name } = expression.kind {
            self.readable_reference(&name)
        } else {
            let (_, owner) = self.value(id)?;
            Some((owner, self.known_bytes.get(&owner).copied().flatten()))
        }
    }

    fn value(&mut self, id: u32) -> Option<(raw::ValueId, raw::PlaceId)> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        if !self.preflight_string_expression(id, at) {
            return None;
        }
        match expression.kind {
            RawExpressionKind::StringLiteral { spelling } => {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes().to_vec();
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let value = self
                    .push_temporary(at, raw::InstructionKind::StringFromUtf8 { bytes, cleanup })?;
                self.known_bytes.insert(
                    value.1,
                    Some(u64::try_from(spelling.len().saturating_sub(2)).unwrap_or(u64::MAX)),
                );
                Some(value)
            }
            RawExpressionKind::Reference { name } => {
                let (source, _) = self.readable_reference(&name)?;
                let value =
                    self.push_temporary(at, raw::InstructionKind::MoveFromPlace { place: source })?;
                let delta = self
                    .owners
                    .rehome_move_result(value.0, source)
                    .expect("readable String move has one registered result owner");
                apply_owner_delta(&mut self.known_bytes, delta);
                Some(value)
            }
            RawExpressionKind::Clone { value, .. } => {
                let (source, bytes) = self.place_for_read(value)?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let cloned = self.push_temporary(
                    at,
                    raw::InstructionKind::StringClone { place: source, cleanup },
                )?;
                self.known_bytes.insert(cloned.1, bytes);
                Some(cloned)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                let [left, right] = arguments.as_slice() else {
                    self.errors.at(
                        "ZRYNA-M3012",
                        span(self.input.sources(), callee.span),
                        "String concat requires exactly two operands",
                        "call concat(left, right) with two available String values",
                    );
                    return None;
                };
                let (left, left_bytes) = self.place_for_read(*left)?;
                let (right, right_bytes) = self.place_for_read(*right)?;
                let bytes = match (left_bytes, right_bytes) {
                    (Some(left), Some(right)) => {
                        let Some(bytes) = checked_string_concat_bytes(left, right) else {
                            self.errors.at(
                                "ZRYNA-M3012",
                                at,
                                "String concatenation exceeds the sealed runtime byte limit",
                                "reduce the statically known concatenated String size",
                            );
                            return None;
                        };
                        Some(bytes)
                    }
                    _ => None,
                };
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let concatenated = self.push_temporary(
                    at,
                    raw::InstructionKind::StringConcat { left, right, cleanup },
                )?;
                self.known_bytes.insert(concatenated.1, bytes);
                Some(concatenated)
            }
            RawExpressionKind::Call { callee, arguments, .. } => {
                self.direct_call(&callee, &arguments, at)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "this private String expression is outside straight-line move lowering",
                    "use a String literal or move one preceding typed String local",
                );
                None
            }
        }
    }

    fn resolve_owned_callee(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
    ) -> Option<FunctionSignature> {
        let signature = match self.catalog.resolve(self.module, &callee.text) {
            FunctionResolution::Exact(signature) => signature.clone(),
            FunctionResolution::WrongCase => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("call name '{}' has the wrong portable ASCII case", callee.text),
                    "use the callee's exact declared spelling",
                );
                return None;
            }
            FunctionResolution::Missing => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("function '{}' is not declared in this module", callee.text),
                    "call one exact private same-module function",
                );
                return None;
            }
        };
        if !signature.private {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned calls require one private same-module callee",
                "keep String producers and identity functions internal",
            );
            return None;
        }
        if signature.result != self.ty
            || signature.has_borrow_parameters()
            || signature.parameters.len() > 1
            || signature.parameters.iter().any(|parameter| *parameter != self.ty)
        {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned call signature is outside the sealed String producer/identity checkpoint",
                "call a private zero-argument String producer or one-String identity function",
            );
            return None;
        }
        Some(signature)
    }

    fn prepare_direct_call_arguments(
        &mut self,
        arguments: &[u32],
        at: Span,
    ) -> Option<(Vec<raw::CallArgument>, Vec<raw::PlaceId>)> {
        let mut lowered = Vec::with_capacity(arguments.len());
        let mut owners = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let (value, owner) = self.value(*argument)?;
            if !self.owners.contains(owner) {
                self.errors.at(
                    "ZRYNA-M3011",
                    at,
                    "owned call argument has no available String owner",
                    "pass each String value exactly once",
                );
                return None;
            }
            owners.push(owner);
            lowered.push(raw::CallArgument::Value(value));
        }
        Some((lowered, owners))
    }

    fn release_direct_call_commit(&mut self) {
        self.cfg.release_transitions(1);
        self.release_local_place();
        self.cfg.release_values(1);
    }

    fn preflight_direct_call_preparation(
        &mut self,
        arguments: &[u32],
        at: Span,
    ) -> Option<OwnedStringPreparationEstimate> {
        let estimate = match estimate_owned_string_call_arguments(
            self.function,
            &self.bindings,
            &self.owners,
            self.ty,
            arguments,
            self.owners.pending().len(),
        ) {
            Ok(estimate) => estimate,
            Err(OwnedStringEstimateError::Unsupported) => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "owned String call argument is outside checked recursive preparation",
                    "use an admitted String literal, move, clone, concat, or private String call",
                );
                return None;
            }
            Err(OwnedStringEstimateError::Unavailable(reference)) => {
                self.errors.at(
                    "ZRYNA-M3011",
                    span(self.input.sources(), reference),
                    "owned call argument has no available String owner",
                    "pass each String value exactly once",
                );
                return None;
            }
            Err(OwnedStringEstimateError::Overflow) => {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "recursive owned String call preparation overflows its checked resource estimate",
                    "reduce nested String-producing call arguments",
                );
                return None;
            }
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans: self.reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions: self.reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
        .then_some(estimate)
    }

    fn direct_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        at: Span,
    ) -> Option<(raw::ValueId, raw::PlaceId)> {
        let signature = self.resolve_owned_callee(callee)?;
        if arguments.len() != signature.parameters.len() {
            self.errors.at(
                "ZRYNA-M3012",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameters.len()
                ),
                "pass the exact declared String argument",
            );
            return None;
        }
        let estimate = self.preflight_direct_call_preparation(arguments, at)?;
        self.cfg.reserve_values(1, at, self.errors)?;
        if !self.reserve_local_place(at) {
            self.cfg.release_values(1);
            return None;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        let reserved_actions = estimate.root_cleanup_actions.expect("direct call cleanup estimate");
        if !self.reserve_cleanup_capacity(reserved_actions, at) {
            self.release_direct_call_commit();
            return None;
        }
        let prepared = self.prepare_direct_call_arguments(arguments, at);
        let Some((lowered, owners)) = prepared else {
            self.release_cleanup_capacity(reserved_actions);
            self.release_direct_call_commit();
            return None;
        };
        let cleanup = raw::CleanupPlanId(
            u32::try_from(self.cleanup_plans.len()).expect("cleanup reservation bounds plan id"),
        );
        for (argument, owner) in lowered.iter().zip(owners) {
            let raw::CallArgument::Value(value) = argument else {
                unreachable!("private String calls use only by-value arguments");
            };
            let Some(delta) = self.owners.transfer(*value) else {
                self.errors.at(
                    "ZRYNA-M3011",
                    at,
                    "owned call argument has no unique available String owner",
                    "pass each String value exactly once",
                );
                self.release_cleanup_capacity(reserved_actions);
                self.release_direct_call_commit();
                return None;
            };
            debug_assert_eq!(delta, OwnerDelta::Transferred { owner });
            apply_owner_delta(&mut self.known_bytes, delta);
        }
        self.release_cleanup_capacity(reserved_actions);
        self.release_direct_call_commit();
        let committed_cleanup = self.push_instruction_cleanup(at, None)?;
        debug_assert_eq!(committed_cleanup, cleanup);
        let result = self.push_temporary(
            at,
            raw::InstructionKind::DirectCall {
                callee: signature.id,
                arguments: lowered,
                cleanup: committed_cleanup,
            },
        )?;
        self.known_bytes.insert(result.1, None);
        Some(result)
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_string_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    result: Ty,
    catalog: &'a FunctionCatalog,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    if function.parameters.len() > 1 {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.span),
            "private String calls admit at most one by-value parameter",
            "use a zero-argument producer or one-String identity function",
        );
        return None;
    }
    let file = &input.syntax().files()[module];
    let cfg = OwnedCfgState::single_block(span(input.sources(), function.body.span), errors)?;
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module,
        ty: result,
        catalog,
        errors,
        bindings: BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState::default(),
        known_bytes: BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (index, parameter) in function.parameters.iter().enumerate() {
        let parameter_ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            lowerer.errors,
        )?;
        if parameter_ty != result && parameter_ty.category != TypeCategory::Bool {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), parameter.span),
                "private String lowering admits only one String or bool parameter",
                "use one owned String argument or one Copy bool branch condition",
            );
            return None;
        }
        if lowerer
            .bindings
            .keys()
            .any(|existing| existing.eq_ignore_ascii_case(&parameter.name.text))
        {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!(
                    "parameter '{}' collides under portable ASCII case folding",
                    parameter.name.text
                ),
                "give every parameter one portable case-insensitive unique name",
            );
            return None;
        }
        let parameter_span = span(input.sources(), parameter.span);
        if !preflight_owned_place_capacity(lowerer.places.len(), 1, parameter_span, lowerer.errors)
        {
            return None;
        }
        let value = raw::ValueId(lowerer.next_value);
        let parameter_definition =
            raw::ValueDefinition { id: value, ty: parameter_ty.ir, span: parameter_span };
        lowerer.cfg.seed_function_parameter(&parameter_definition, lowerer.errors)?;
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        parameters.push(parameter_definition);
        let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: place,
            ty: parameter_ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        if parameter_ty == result {
            let _ = lowerer.owners.register_parameter(place);
            lowerer.known_bytes.insert(place, None);
        }
        lowerer.bindings.insert(
            parameter.name.text.clone(),
            Binding { ty: parameter_ty, place, mutable: false },
        );
    }
    if root_is_terminal_if(function) {
        if lowerer.bindings.values().any(|binding| binding.ty.category != TypeCategory::Bool) {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), function.span),
                "terminal owned String if admits only one optional bool parameter",
                "move owned inputs into branch-local producer expressions",
            );
            return None;
        }
        let terminal = terminal_owned_if(function, input.sources(), lowerer.errors)?;
        let bool_ty =
            node_types.iter().flatten().find(|ty| ty.category == TypeCategory::Bool).copied()?;
        if !lowerer.cfg.preflight_skeleton(3, 4, terminal.span, lowerer.errors) {
            return None;
        }
        lowerer.cfg.reserve_values(1, terminal.span, lowerer.errors)?;
        if !lowerer.reserve_local_place(terminal.span) {
            lowerer.cfg.release_values(1);
            return None;
        }
        if !lowerer.reserve_cleanup_capacity(0, terminal.span) {
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        let Some(condition) = lowerer.condition(terminal.condition, bool_ty) else {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        };
        let then_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        let else_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        let join_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        if !lowerer.cfg.terminate(
            raw::SpannedTerminator {
                span: terminal.span,
                kind: raw::Terminator::Branch {
                    condition,
                    when_true: raw::Edge { target: then_id, arguments: Vec::new() },
                    when_false: raw::Edge { target: else_id, arguments: Vec::new() },
                },
            },
            lowerer.errors,
        ) {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        let incoming = OwnedStringBranchState {
            bindings: lowerer.bindings.clone(),
            owners: lowerer.owners.clone(),
            known_bytes: lowerer.known_bytes.clone(),
        };
        let arms_lowered = (|| {
            for (block, expression, arm_span) in [
                (then_id, terminal.then_value, terminal.then_span),
                (else_id, terminal.else_value, terminal.else_span),
            ] {
                lowerer.cfg.begin_block(block, Vec::new(), arm_span, lowerer.errors)?;
                let (value, carried) = lowerer.value(expression)?;
                lowerer.drop_non_carried(carried, arm_span)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: arm_span,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: vec![value],
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings.clone();
                lowerer.owners = incoming.owners.clone();
                lowerer.known_bytes = incoming.known_bytes.clone();
            }
            Some(())
        })();
        if arms_lowered.is_none() {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        lowerer.release_cleanup_capacity(0);
        lowerer.release_local_place();
        lowerer.cfg.release_values(1);
        let joined = raw::ValueId(lowerer.next_value);
        let joined_definition =
            raw::ValueDefinition { id: joined, ty: result.ir, span: terminal.span };
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        lowerer.cfg.begin_block(join_id, vec![joined_definition], terminal.span, lowerer.errors)?;
        let joined_owner = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: joined_owner,
            ty: result.ir,
            span: terminal.span,
            kind: raw::PlaceKind::Temporary(joined),
        });
        let _ = lowerer.owners.register(joined, joined_owner);
        lowerer.known_bytes.insert(joined_owner, None);
        let cleanup = lowerer.push_cleanup(terminal.span, Some(joined_owner))?;
        if !lowerer.cfg.terminate(
            raw::SpannedTerminator {
                span: terminal.span,
                kind: raw::Terminator::Return { value: joined, cleanup },
            },
            lowerer.errors,
        ) {
            return None;
        }
        let blocks = lowerer.cfg.finish(terminal.span, lowerer.errors)?;
        return Some(raw::Function {
            id: raw::FunctionId {
                module: raw::ModuleId(u32::try_from(module).ok()?),
                declaration: u32::try_from(declaration).ok()?,
            },
            entry_export: None,
            span: span(input.sources(), function.span),
            parameters,
            borrow_parameters: Vec::new(),
            result: result.ir,
            places: lowerer.places,
            blocks,
            cleanup_plans: lowerer.cleanup_plans,
        });
    }
    let mut returned = None;
    let mut saw_if = false;
    let mut saw_loop = false;
    for statement_id in &root.statements {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        match &statement.kind {
            RawStatementKind::LocalDeclaration {
                mutable, name, type_syntax, initializer, ..
            } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "owned String control flow must immediately precede the final return",
                        "move every outer declaration before the single top-level control-flow statement",
                    );
                    return None;
                }
                let local_ty = semantic_type(
                    file,
                    *type_syntax,
                    module,
                    declarations,
                    graph,
                    node_types,
                    lowerer.errors,
                )?;
                if local_ty != result {
                    lowerer.errors.at(
                        "ZRYNA-M3012",
                        span(input.sources(), statement.span),
                        "private String lowering requires exact typed String locals",
                        "declare each local as String",
                    );
                    return None;
                }
                if lowerer.bindings.keys().any(|existing| existing.eq_ignore_ascii_case(&name.text))
                {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!(
                            "binding '{}' collides under portable ASCII case folding",
                            name.text
                        ),
                        "give every binding one portable case-insensitive unique name",
                    );
                    return None;
                }
                let local_span = span(input.sources(), statement.span);
                if !lowerer.reserve_local_commit(local_span) {
                    return None;
                }
                let Some((value, temporary)) = lowerer.value(*initializer) else {
                    lowerer.release_local_commit();
                    return None;
                };
                let local =
                    raw::PlaceId(u32::try_from(lowerer.places.len()).expect("bounded places"));
                let initialize = raw::Instruction {
                    result: None,
                    span: local_span,
                    kind: raw::InstructionKind::InitializePlace { place: local, value },
                };
                lowerer.release_local_commit();
                lowerer.places.push(raw::Place {
                    id: local,
                    ty: result.ir,
                    span: local_span,
                    kind: raw::PlaceKind::Local(lowerer.next_local),
                });
                lowerer.next_local += 1;
                if !lowerer.cfg.emit(initialize, lowerer.errors) {
                    return None;
                }
                let Some(delta) = lowerer.owners.rename(value, local) else {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), statement.span),
                        "String local initializer has no available owner",
                        "initialize the local from one available String value",
                    );
                    return None;
                };
                debug_assert_eq!(delta, OwnerDelta::Renamed { from: temporary, to: local });
                apply_owner_delta(&mut lowerer.known_bytes, delta);
                lowerer.bindings.insert(
                    name.text.clone(),
                    Binding { ty: result, place: local, mutable: *mutable },
                );
            }
            RawStatementKind::Assignment { target, value, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "owned String control-flow lowering excludes assignment after its exit",
                        "leave the joined outer String state unchanged and return it directly",
                    );
                    return None;
                }
                let target_expression = lowerer.expression(*target)?.clone();
                let RawExpressionKind::Reference { name } = target_expression.kind else {
                    lowerer.errors.at(
                        "ZRYNA-M3012",
                        span(input.sources(), target_expression.span),
                        "String assignment requires one root local target",
                        "assign only to an initialized mutable String local",
                    );
                    return None;
                };
                let Some(binding) = lowerer.bindings.get(&name.text).cloned() else {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!("String assignment target '{}' is not declared", name.text),
                        "assign one exact preceding String local",
                    );
                    return None;
                };
                if binding.ty != result {
                    lowerer.errors.at(
                        "ZRYNA-M3012",
                        span(input.sources(), name.span),
                        "String assignment target has the wrong exact type",
                        "assign only to an exact String local",
                    );
                    return None;
                }
                if !binding.mutable || !lowerer.owners.contains(binding.place) {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), name.span),
                        "String assignment target is immutable, uninitialized, or already moved",
                        "assign only to an initialized mutable available String local",
                    );
                    return None;
                }
                if let Some(reference_span) =
                    lowerer.target_consumption_span(*value, binding.place, true)
                {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), reference_span),
                        "String assignment cannot consume its destination while preparing its replacement",
                        "prepare a distinct String value or explicitly clone the destination",
                    );
                    return None;
                }
                let assignment_span = span(input.sources(), statement.span);
                if !reserve_owned_commit_transition(
                    &mut lowerer.cfg,
                    assignment_span,
                    lowerer.errors,
                ) {
                    return None;
                }
                let Some((prepared_value, prepared_owner)) = lowerer.value(*value) else {
                    release_owned_commit_transition(&mut lowerer.cfg);
                    return None;
                };
                release_owned_commit_transition(&mut lowerer.cfg);
                if !lowerer.cfg.emit(
                    raw::Instruction {
                        result: None,
                        span: assignment_span,
                        kind: raw::InstructionKind::ReplacePlace {
                            place: binding.place,
                            value: prepared_value,
                        },
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                let Some(delta) = lowerer.owners.replace(prepared_value, binding.place) else {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), statement.span),
                        "String assignment replacement has no distinct prepared owner",
                        "replace from one available independently prepared String value",
                    );
                    return None;
                };
                debug_assert_eq!(
                    delta,
                    OwnerDelta::Replaced { prepared: prepared_owner, target: binding.place }
                );
                apply_owner_delta(&mut lowerer.known_bytes, delta);
            }
            RawStatementKind::If { condition, then_block, else_clause, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "nested or repeated owned String if statements are not supported",
                        "use exactly one top-level if before the final return",
                    );
                    return None;
                }
                saw_if = true;
                let bool_ty = node_types
                    .iter()
                    .flatten()
                    .find(|ty| ty.category == TypeCategory::Bool)
                    .copied()?;
                let condition = lowerer.condition(*condition, bool_ty)?;
                let at = span(input.sources(), statement.span);
                let then_id = lowerer.cfg.reserve_block(at, lowerer.errors)?;
                let else_id = lowerer.cfg.reserve_block(at, lowerer.errors)?;
                let join_id = lowerer.cfg.reserve_block(at, lowerer.errors)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: then_id, arguments: Vec::new() },
                            when_false: raw::Edge { target: else_id, arguments: Vec::new() },
                        },
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                let incoming = OwnedStringBranchState {
                    bindings: lowerer.bindings.clone(),
                    owners: lowerer.owners.clone(),
                    known_bytes: lowerer.known_bytes.clone(),
                };
                let branch_types = StringBranchTypes { file, declarations, graph, node_types };
                lowerer.cfg.begin_block(then_id, Vec::new(), at, lowerer.errors)?;
                lowerer.lower_branch(Some(*then_block), &incoming, at, branch_types)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings.clone();
                lowerer.owners = incoming.owners.clone();
                lowerer.known_bytes = incoming.known_bytes.clone();
                lowerer.cfg.begin_block(else_id, Vec::new(), at, lowerer.errors)?;
                lowerer.lower_branch(
                    else_clause.as_ref().map(|clause| clause.block),
                    &incoming,
                    at,
                    branch_types,
                )?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings;
                lowerer.owners = incoming.owners;
                lowerer.known_bytes = incoming.known_bytes;
                lowerer.cfg.begin_block(join_id, Vec::new(), at, lowerer.errors)?;
            }
            RawStatementKind::While { condition, body_block, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "nested or repeated owned String loops are not supported",
                        "use exactly one top-level while before the final return",
                    );
                    return None;
                }
                saw_loop = true;
                let bool_ty = node_types
                    .iter()
                    .flatten()
                    .find(|ty| ty.category == TypeCategory::Bool)
                    .copied()?;
                let at = span(input.sources(), statement.span);
                if !preflight_owned_loop_exit(
                    function,
                    *statement_id,
                    input.sources(),
                    lowerer.errors,
                ) {
                    return None;
                }
                if !preflight_owned_loop_body(
                    function,
                    *body_block,
                    false,
                    input.sources(),
                    lowerer.errors,
                ) {
                    return None;
                }
                let body = usize::try_from(*body_block)
                    .ok()
                    .and_then(|index| function.body.blocks.get(index))?;
                let mutation = match body.statements.as_slice() {
                    [mutation_id] => usize::try_from(*mutation_id)
                        .ok()
                        .and_then(|index| function.body.statements.get(index))
                        .filter(|statement| {
                            matches!(statement.kind, RawStatementKind::Assignment { .. })
                        })
                        .cloned(),
                    _ => None,
                };
                if mutation.is_some() && lowerer.owners.pending().len() != 1 {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        at,
                        "owned String mutation loop requires exactly one incoming owned root",
                        "declare one mutable outer String before the loop",
                    );
                    return None;
                }
                if !preflight_owned_string_loop_skeleton(
                    &lowerer.cfg,
                    &mut lowerer.known_bytes,
                    mutation.is_some(),
                    at,
                    lowerer.errors,
                ) {
                    return None;
                }
                let header_id = lowerer.cfg.reserve_block(at, lowerer.errors).expect("preflight");
                let body_id = lowerer.cfg.reserve_block(at, lowerer.errors).expect("preflight");
                let exit_id = lowerer.cfg.reserve_block(at, lowerer.errors).expect("preflight");
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: header_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                let incoming = OwnedStringBranchState {
                    bindings: lowerer.bindings.clone(),
                    owners: lowerer.owners.clone(),
                    known_bytes: lowerer.known_bytes.clone(),
                };
                lowerer.cfg.begin_block(header_id, Vec::new(), at, lowerer.errors)?;
                let condition = lowerer.condition(*condition, bool_ty)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: body_id, arguments: Vec::new() },
                            when_false: raw::Edge { target: exit_id, arguments: Vec::new() },
                        },
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                let branch_types = StringBranchTypes { file, declarations, graph, node_types };
                lowerer.cfg.begin_block(body_id, Vec::new(), at, lowerer.errors)?;
                if let Some(mutation) = mutation.as_ref() {
                    lowerer.lower_loop_assignment(mutation, &incoming)?;
                } else {
                    lowerer.lower_branch(Some(*body_block), &incoming, at, branch_types)?;
                }
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: header_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings;
                lowerer.owners = incoming.owners;
                lowerer.known_bytes = incoming.known_bytes;
                lowerer.cfg.begin_block(exit_id, Vec::new(), at, lowerer.errors)?;
            }
            RawStatementKind::Return { value, .. } => {
                let value = lowerer.value(*value)?;
                returned = Some((value, span(input.sources(), statement.span)));
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3012",
                    span(input.sources(), statement.span),
                    "this statement is outside straight-line private String lowering",
                    "use typed local initialization and one final return",
                );
                return None;
            }
        }
    }
    let ((return_value, returned_place), return_span) = returned?;
    let return_owner = lowerer.owners.owner(return_value)?;
    debug_assert_eq!(return_owner, returned_place);
    let return_cleanup = lowerer.push_cleanup(return_span, Some(return_owner))?;
    if !lowerer.cfg.terminate(
        raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: return_value, cleanup: return_cleanup },
        },
        lowerer.errors,
    ) {
        return None;
    }
    let blocks = lowerer.cfg.finish(return_span, lowerer.errors)?;
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters: Vec::new(),
        result: result.ir,
        places: lowerer.places,
        blocks,
        cleanup_plans: lowerer.cleanup_plans,
    })
}

fn reserve_owned_commit_transition(
    cfg: &mut OwnedCfgState,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    reserve_owned_commit_transitions(cfg, 1, at, errors)
}

fn release_owned_commit_transition(cfg: &mut OwnedCfgState) {
    release_owned_commit_transitions(cfg, 1);
}

fn reserve_owned_commit_transitions(
    cfg: &mut OwnedCfgState,
    transitions: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    cfg.reserve_transitions(transitions, at, errors).is_some()
}

fn release_owned_commit_transitions(cfg: &mut OwnedCfgState, transitions: usize) {
    cfg.release_transitions(transitions);
}

#[derive(Clone, Copy)]
struct StringBranchTypes<'a> {
    file: &'a syntax::SourceUnit,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
}

fn aggregate_graph_is_supported(
    ty: Ty,
    layouts: &layout::VerifiedLayouts,
    visiting: &mut BTreeSet<layout::TypeId>,
) -> bool {
    if !visiting.insert(ty.layout) {
        return false;
    }
    let supported = layouts.type_by_id(ty.layout).is_some_and(|record| match record.category() {
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => true,
        TypeCategory::Struct => record.fields().iter().all(|field| {
            let child = layouts.type_by_id(field.ty()).map(|child| Ty {
                layout: child.id(),
                ir: raw::TypeId(child.id().index()),
                category: child.category(),
                drop_kind: child.drop_kind(),
                runtime_kind: child.runtime_kind(),
                cloneable: false,
            });
            child.is_some_and(|child| aggregate_graph_is_supported(child, layouts, visiting))
        }),
        TypeCategory::FixedArray => record.referenced_type().is_some_and(|child| {
            layouts.type_by_id(child).is_some_and(|child| {
                aggregate_graph_is_supported(
                    Ty {
                        layout: child.id(),
                        ir: raw::TypeId(child.id().index()),
                        category: child.category(),
                        drop_kind: child.drop_kind(),
                        runtime_kind: child.runtime_kind(),
                        cloneable: false,
                    },
                    layouts,
                    visiting,
                )
            })
        }),
        TypeCategory::Enum | TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => false,
    });
    visiting.remove(&ty.layout);
    supported
}

fn owned_enum_graph_is_supported(ty: Ty, layouts: &layout::VerifiedLayouts) -> bool {
    layouts.type_by_id(ty.layout).is_some_and(|record| {
        record.category() == TypeCategory::Enum
            && record.variants().iter().all(|variant| {
                variant.payload().is_none_or(|payload| {
                    layouts.type_by_id(payload).is_some_and(|payload| {
                        aggregate_graph_is_supported(
                            Ty {
                                layout: payload.id(),
                                ir: raw::TypeId(payload.id().index()),
                                category: payload.category(),
                                drop_kind: payload.drop_kind(),
                                runtime_kind: payload.runtime_kind(),
                                cloneable: false,
                            },
                            layouts,
                            &mut BTreeSet::new(),
                        )
                    })
                })
            })
    })
}

struct PrivateOwnedAggregateLowerer<'a, 'f, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'f syntax::RawFunctionSyntax,
    module: usize,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    moved_projections: BTreeSet<raw::PlaceId>,
    partial_roots: BTreeSet<raw::PlaceId>,
    places: Vec<raw::Place>,
    instructions: Vec<raw::Instruction>,
    cleanup_plans: Vec<raw::CleanupPlan>,
    cleanup_actions: usize,
    aggregate_operands: usize,
    aggregate_subobject_moves: usize,
    projected_aggregate_clones: usize,
    projected_aggregate_assignments: usize,
    reserved_transitions: usize,
    owners: OwnerState,
    next_value: u32,
    next_local: u32,
}

fn append_owned_projection_shape(
    ty: Ty,
    layouts: &layout::VerifiedLayouts,
    parent: Option<usize>,
    shape: &mut Vec<OwnedProjectionShapeEntry>,
) -> Option<()> {
    let record = layouts.type_by_id(ty.layout)?;
    match ty.category {
        TypeCategory::Struct => {
            for field in record.fields() {
                if shape.len() >= ir::MAX_PLACES_PER_FUNCTION {
                    return None;
                }
                let child_record = layouts.type_by_id(field.ty())?;
                let child = Ty {
                    layout: child_record.id(),
                    ir: raw::TypeId(child_record.id().index()),
                    category: child_record.category(),
                    drop_kind: child_record.drop_kind(),
                    runtime_kind: child_record.runtime_kind(),
                    cloneable: false,
                };
                let index = shape.len();
                shape.push(OwnedProjectionShapeEntry {
                    parent,
                    ty: child,
                    kind: OwnedStaticProjectionKind::StructField { ordinal: field.ordinal() },
                });
                append_owned_projection_shape(child, layouts, Some(index), shape)?;
            }
        }
        TypeCategory::FixedArray => {
            let child_record = layouts.type_by_id(record.referenced_type()?)?;
            let child = Ty {
                layout: child_record.id(),
                ir: raw::TypeId(child_record.id().index()),
                category: child_record.category(),
                drop_kind: child_record.drop_kind(),
                runtime_kind: child_record.runtime_kind(),
                cloneable: false,
            };
            let length = usize::try_from(record.array_length()?).ok()?;
            for index in 0..length {
                if shape.len() >= ir::MAX_PLACES_PER_FUNCTION {
                    return None;
                }
                let ordinal = u32::try_from(index).ok()?;
                let child_index = shape.len();
                shape.push(OwnedProjectionShapeEntry {
                    parent,
                    ty: child,
                    kind: OwnedStaticProjectionKind::FixedArrayConstant { index: ordinal },
                });
                append_owned_projection_shape(child, layouts, Some(child_index), shape)?;
            }
        }
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
        TypeCategory::Enum | TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => {
            return None;
        }
    }
    Some(())
}

fn complete_owned_projection_shape(
    ty: Ty,
    layouts: &layout::VerifiedLayouts,
) -> Option<Vec<OwnedProjectionShapeEntry>> {
    let mut shape = Vec::new();
    append_owned_projection_shape(ty, layouts, None, &mut shape)?;
    Some(shape)
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

    fn supported(&self, ty: Ty) -> bool {
        if ty.category == TypeCategory::Enum {
            owned_enum_graph_is_supported(ty, self.layouts)
        } else {
            aggregate_graph_is_supported(ty, self.layouts, &mut BTreeSet::new())
        }
    }

    fn place_parent(&self, place: raw::PlaceId) -> Option<raw::PlaceId> {
        match self.places.get(place.0 as usize)?.kind {
            raw::PlaceKind::StructField { base, .. }
            | raw::PlaceKind::EnumPayload { base, .. }
            | raw::PlaceKind::FixedArrayConstant { base, .. } => Some(base),
            raw::PlaceKind::Parameter(_)
            | raw::PlaceKind::Local(_)
            | raw::PlaceKind::Temporary(_) => None,
        }
    }

    fn place_is_at_or_below(&self, mut place: raw::PlaceId, root: raw::PlaceId) -> bool {
        let mut visited = BTreeSet::new();
        while visited.insert(place) {
            if place == root {
                return true;
            }
            let Some(parent) = self.place_parent(place) else { return false };
            place = parent;
        }
        false
    }

    fn places_overlap(&self, left: raw::PlaceId, right: raw::PlaceId) -> bool {
        self.place_is_at_or_below(left, right) || self.place_is_at_or_below(right, left)
    }

    fn complete_projection_shape(&self, ty: Ty) -> Option<Vec<OwnedProjectionShapeEntry>> {
        complete_owned_projection_shape(ty, self.layouts)
    }

    fn existing_projection_shape(
        &self,
        root: raw::PlaceId,
        shape: &[OwnedProjectionShapeEntry],
    ) -> Vec<Option<raw::PlaceId>> {
        let mut places = Vec::with_capacity(shape.len());
        for entry in shape {
            let parent = match entry.parent {
                Some(index) => places[index],
                None => Some(root),
            };
            places.push(parent.and_then(|parent| {
                let key = match entry.kind {
                    OwnedStaticProjectionKind::StructField { ordinal } => (parent.0, 0, ordinal),
                    OwnedStaticProjectionKind::FixedArrayConstant { index } => (parent.0, 1, index),
                };
                self.projections.get(&key).copied()
            }));
        }
        places
    }

    fn materialize_projection_shape(
        &mut self,
        root: raw::PlaceId,
        shape: &[OwnedProjectionShapeEntry],
        at: Span,
    ) -> Vec<raw::PlaceId> {
        let mut places = Vec::with_capacity(shape.len());
        for entry in shape {
            let parent = entry.parent.map_or(root, |index| places[index]);
            let (key, kind) = match entry.kind {
                OwnedStaticProjectionKind::StructField { ordinal } => {
                    ((parent.0, 0, ordinal), raw::PlaceKind::StructField { base: parent, ordinal })
                }
                OwnedStaticProjectionKind::FixedArrayConstant { index } => (
                    (parent.0, 1, index),
                    raw::PlaceKind::FixedArrayConstant { base: parent, index },
                ),
            };
            places.push(
                self.push_projection(entry.ty, at, key, kind)
                    .expect("partial transfer topology capacity preflighted"),
            );
        }
        places
    }

    fn migrate_partial_mask(
        &mut self,
        source_root: raw::PlaceId,
        target_root: raw::PlaceId,
        source_places: &[raw::PlaceId],
        target_places: &[raw::PlaceId],
    ) {
        assert_eq!(source_places.len(), target_places.len(), "partial transfer topology matched");
        assert!(self.partial_roots.contains(&source_root), "partial transfer source tracked");
        let moved = self
            .moved_projections
            .iter()
            .copied()
            .filter(|place| self.place_is_at_or_below(*place, source_root))
            .collect::<Vec<_>>();
        let mapped = moved
            .iter()
            .map(|source| {
                source_places
                    .iter()
                    .position(|candidate| candidate == source)
                    .map(|index| target_places[index])
            })
            .collect::<Option<Vec<_>>>()
            .expect("complete partial transfer mask mapping");
        assert!(self.partial_roots.remove(&source_root), "partial transfer source tracked");
        self.partial_roots.insert(target_root);
        for (source, target) in moved.into_iter().zip(mapped) {
            self.moved_projections.remove(&source);
            self.moved_projections.insert(target);
        }
    }

    fn whole_root_available(&self, root: raw::PlaceId) -> bool {
        self.owners.contains(root) && !self.partial_roots.contains(&root)
    }

    fn projection_available(&self, projection: raw::PlaceId, root: raw::PlaceId) -> bool {
        self.owners.contains(root)
            && !self.moved_projections.iter().any(|moved| self.places_overlap(*moved, projection))
    }

    fn push_projection(
        &mut self,
        ty: Ty,
        at: Span,
        key: (u32, u8, u32),
        kind: raw::PlaceKind,
    ) -> Option<raw::PlaceId> {
        if let Some(place) = self.projections.get(&key).copied() {
            return Some(place);
        }
        if self.places.len() >= ir::MAX_PLACES_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "derived owned projection places exceed the per-function M3 limit",
                "reduce distinct private aggregate field and fixed-array projections",
            );
            return None;
        }
        let place = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        self.places.push(raw::Place { id: place, ty: ty.ir, span: at, kind });
        self.projections.insert(key, place);
        Some(place)
    }

    fn field_projection_type(
        &mut self,
        base: Ty,
        name: &syntax::RawIdentifierSyntax,
    ) -> Option<(u32, Ty)> {
        let use_span = span(self.input.sources(), name.span);
        let Some(nominal) =
            self.layouts.type_by_id(base.layout).and_then(layout::VerifiedType::nominal_identity)
        else {
            self.errors.at(
                "ZRYNA-M3006",
                use_span,
                "owned field projection requires an exact struct place",
                "project one declared field from a supported private struct",
            );
            return None;
        };
        let Some(decl) = self.declarations.iter().find(|decl| {
            (u32::try_from(decl.module).ok(), u32::try_from(decl.declaration).ok())
                == (Some(nominal.0), Some(nominal.1))
        }) else {
            self.errors.at(
                "ZRYNA-M3006",
                use_span,
                "owned field projection has no authenticated declaration",
                "project one declared field from a supported private struct",
            );
            return None;
        };
        let RawDataDeclarationKind::Struct { fields, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3006",
                use_span,
                "owned field projection requires a struct, not an enum",
                "project one declared field from a supported private struct",
            );
            return None;
        };
        fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name.text == name.text)
            .and_then(|(ordinal, field)| {
                u32::try_from(ordinal).ok().zip(semantic_type(
                    self.file,
                    field.type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                ))
            })
            .or_else(|| {
                self.errors.at(
                    "ZRYNA-M3006",
                    use_span,
                    format!("struct '{}' has no field '{}'", decl.name, name.text),
                    "use one exact declared field name",
                );
                None
            })
    }

    fn constant_projection_type(&mut self, base: Ty, index_id: u32) -> Option<(u32, Ty)> {
        let expression = self.expression(index_id)?.clone();
        let at = span(self.input.sources(), expression.span);
        if base.category != TypeCategory::FixedArray {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                "owned indexing currently admits only fixed-array projections",
                "use one constant index into a supported private fixed array",
            );
            return None;
        }
        let RawExpressionKind::I32Literal { spelling } = expression.kind else {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                "owned fixed-array indices must be compile-time i32 literals",
                "use a nonnegative literal within the fixed-array length",
            );
            return None;
        };
        let index = spelling.parse::<u32>().ok().or_else(|| {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                "owned fixed-array index is negative or outside u32",
                "use a nonnegative constant index",
            );
            None
        })?;
        let record = self.layouts.type_by_id(base.layout)?;
        let length = record.array_length()?;
        if u64::from(index) >= length {
            self.errors.at(
                "ZRYNA-M3006",
                at,
                format!("owned fixed-array index {index} is outside length {length}"),
                "use an index less than the exact fixed-array length",
            );
            return None;
        }
        let element = record.referenced_type()?;
        let ty = self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        Some((index, ty))
    }

    fn projection_expression_type(&self, id: u32) -> Option<Ty> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).map(|binding| binding.ty)
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let base = self.projection_expression_type(*base)?;
                let nominal = self.layouts.type_by_id(base.layout)?.nominal_identity()?;
                let declaration = self.declarations.iter().find(|declaration| {
                    (
                        u32::try_from(declaration.module).ok(),
                        u32::try_from(declaration.declaration).ok(),
                    ) == (Some(nominal.0), Some(nominal.1))
                })?;
                let RawDataDeclarationKind::Struct { fields, .. } =
                    &self.file.data_declarations()[declaration.declaration].kind
                else {
                    return None;
                };
                let ordinal =
                    fields.iter().position(|candidate| candidate.name.text == field.text)?;
                let field_ty = self.layouts.type_by_id(base.layout)?.fields().get(ordinal)?.ty();
                self.node_types.iter().flatten().find(|ty| ty.layout == field_ty).copied()
            }
            RawExpressionKind::Index { base, index, .. } => {
                let base = self.projection_expression_type(*base)?;
                if base.category != TypeCategory::FixedArray {
                    return None;
                }
                let index_expression = self.expression(*index)?;
                let RawExpressionKind::I32Literal { spelling } = &index_expression.kind else {
                    return None;
                };
                let index = spelling.parse::<u32>().ok()?;
                let record = self.layouts.type_by_id(base.layout)?;
                if u64::from(index) >= record.array_length()? {
                    return None;
                }
                let element = record.referenced_type()?;
                self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()
            }
            _ => None,
        }
    }

    fn owned_place_preflight(&self, id: u32) -> Option<OwnedAggregatePlacePreflight> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name } => {
                let binding = self.bindings.get(&name.text)?;
                Some(OwnedAggregatePlacePreflight {
                    place: OwnedAggregatePlace {
                        ty: binding.ty,
                        place: binding.place,
                        root: binding.place,
                        mutable: binding.mutable,
                        is_root: true,
                    },
                    missing: 0,
                    lineage: vec![binding.place],
                })
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let mut base = self.owned_place_preflight(*base)?;
                let nominal = self.layouts.type_by_id(base.place.ty.layout)?.nominal_identity()?;
                let declaration = self.declarations.iter().find(|declaration| {
                    (
                        u32::try_from(declaration.module).ok(),
                        u32::try_from(declaration.declaration).ok(),
                    ) == (Some(nominal.0), Some(nominal.1))
                })?;
                let RawDataDeclarationKind::Struct { fields, .. } =
                    &self.file.data_declarations()[declaration.declaration].kind
                else {
                    return None;
                };
                let ordinal = u32::try_from(
                    fields.iter().position(|candidate| candidate.name.text == field.text)?,
                )
                .ok()?;
                let ty = self.projection_expression_type(id)?;
                let key = (base.place.place.0, 0, ordinal);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let index = self.places.len().checked_add(base.missing)?;
                    base.missing = base.missing.checked_add(1)?;
                    raw::PlaceId(u32::try_from(index).ok()?)
                };
                base.lineage.push(place);
                base.place = OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.place.root,
                    mutable: base.place.mutable,
                    is_root: false,
                };
                Some(base)
            }
            RawExpressionKind::Index { base, index, .. } => {
                let mut base = self.owned_place_preflight(*base)?;
                if base.place.ty.category != TypeCategory::FixedArray {
                    return None;
                }
                let RawExpressionKind::I32Literal { spelling } = &self.expression(*index)?.kind
                else {
                    return None;
                };
                let index = spelling.parse::<u32>().ok()?;
                let record = self.layouts.type_by_id(base.place.ty.layout)?;
                if u64::from(index) >= record.array_length()? {
                    return None;
                }
                let ty = self.projection_expression_type(id)?;
                let key = (base.place.place.0, 1, index);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let place_index = self.places.len().checked_add(base.missing)?;
                    base.missing = base.missing.checked_add(1)?;
                    raw::PlaceId(u32::try_from(place_index).ok()?)
                };
                base.lineage.push(place);
                base.place = OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.place.root,
                    mutable: base.place.mutable,
                    is_root: false,
                };
                Some(base)
            }
            _ => None,
        }
    }

    fn preflight_projection_available(&self, place: &OwnedAggregatePlacePreflight) -> bool {
        self.owners.contains(place.place.root)
            && !self.moved_projections.iter().any(|moved| {
                place.lineage.contains(moved)
                    || (self.places.get(place.place.place.0 as usize).is_some()
                        && self.place_is_at_or_below(*moved, place.place.place))
            })
    }

    fn owned_place(&mut self, id: u32) -> Option<OwnedAggregatePlace> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        match expression.kind {
            RawExpressionKind::Reference { name } => self
                .bindings
                .get(&name.text)
                .cloned()
                .map(|binding| OwnedAggregatePlace {
                    ty: binding.ty,
                    place: binding.place,
                    root: binding.place,
                    mutable: binding.mutable,
                    is_root: true,
                })
                .or_else(|| {
                    let wrong_case =
                        self.bindings.keys().any(|key| key.eq_ignore_ascii_case(&name.text));
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        if wrong_case {
                            format!(
                                "aggregate value '{}' has the wrong portable ASCII case",
                                name.text
                            )
                        } else {
                            format!("aggregate value '{}' is not declared", name.text)
                        },
                        "reference one exact preceding local using its declared spelling",
                    );
                    None
                }),
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let base = self.owned_place(base)?;
                let (ordinal, ty) = self.field_projection_type(base.ty, &field)?;
                let key = (base.place.0, 0, ordinal);
                let place = self.push_projection(
                    ty,
                    at,
                    key,
                    raw::PlaceKind::StructField { base: base.place, ordinal },
                )?;
                Some(OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.root,
                    mutable: base.mutable,
                    is_root: false,
                })
            }
            RawExpressionKind::Index { base, index, .. } => {
                let base = self.owned_place(base)?;
                let (index, ty) = self.constant_projection_type(base.ty, index)?;
                let key = (base.place.0, 1, index);
                let place = self.push_projection(
                    ty,
                    at,
                    key,
                    raw::PlaceKind::FixedArrayConstant { base: base.place, index },
                )?;
                Some(OwnedAggregatePlace {
                    ty,
                    place,
                    root: base.root,
                    mutable: base.mutable,
                    is_root: false,
                })
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned projection base is outside the static place checkpoint",
                    "project from a named private Struct or fixed-array local",
                );
                None
            }
        }
    }

    fn preflight_transition(&mut self, additional: usize, at: Span) -> bool {
        if aggregate_transition_budget_violation(
            self.instructions.len(),
            self.reserved_transitions,
            additional,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived ownership transitions exceed the per-function M3 limit of {}",
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                ),
                "reduce private aggregate expressions and assignments",
            );
            return false;
        }
        true
    }

    fn reserve_transition(&mut self, at: Span) -> bool {
        if !self.preflight_transition(1, at) {
            return false;
        }
        self.reserved_transitions += 1;
        true
    }

    fn release_transition(&mut self) {
        self.reserved_transitions = self
            .reserved_transitions
            .checked_sub(1)
            .expect("reserved aggregate assignment transition");
    }

    fn emit_effect(&mut self, at: Span, kind: raw::InstructionKind) -> bool {
        if !self.preflight_transition(1, at) {
            return false;
        }
        self.instructions.push(raw::Instruction { result: None, span: at, kind });
        true
    }

    fn push_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        if self.cleanup_plans.len() >= ir::MAX_CLEANUP_PLANS_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup sites exceed the per-function M3 limit of {}",
                    ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                ),
                "reduce fallible String leaves in private aggregate construction",
            );
            return None;
        }
        let pending = self.owners.pending();
        let excluded_present = excluded.is_some_and(|place| self.owners.contains(place));
        let action_count = pending.len() - usize::from(excluded_present);
        if cleanup_action_budget_violation(self.cleanup_actions, pending.len(), excluded_present) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce simultaneously live owned aggregates and String leaves",
            );
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).unwrap_or(u32::MAX));
        let actions = pending
            .iter()
            .rev()
            .copied()
            .filter(|place| Some(*place) != excluded)
            .map(raw::DropAction::DropPlace)
            .collect();
        self.cleanup_plans.push(raw::CleanupPlan { id, span: at, actions });
        self.cleanup_actions += action_count;
        Some(id)
    }

    fn emit(&mut self, ty: Ty, at: Span, kind: raw::InstructionKind) -> Option<raw::ValueId> {
        if !self.preflight_transition(1, at) {
            return None;
        }
        if self.next_value as usize >= ir::MAX_VALUES_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived values exceed the per-function M3 limit of {}",
                    ir::MAX_VALUES_PER_FUNCTION
                ),
                "reduce private aggregate expressions",
            );
            return None;
        }
        if !ty.is_copy() && self.places.len() >= ir::MAX_PLACES_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned aggregate temporaries and locals",
            );
            return None;
        }
        let value = raw::ValueId(self.next_value);
        self.next_value += 1;
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind,
        });
        if !ty.is_copy() {
            let owner = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
            self.places.push(raw::Place {
                id: owner,
                ty: ty.ir,
                span: at,
                kind: raw::PlaceKind::Temporary(value),
            });
            self.owners.register(value, owner)?;
        }
        Some(value)
    }

    fn target_consumption_span(
        &self,
        id: u32,
        target: raw::PlaceId,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && self.bindings.get(&name.text).is_some_and(|binding| {
                        binding.place == target && self.owners.contains(binding.place)
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.target_consumption_span(*value, target, false)
            }
            RawExpressionKind::StructConstruction { fields, .. } => {
                fields.iter().find_map(|field| {
                    let value = match field.kind {
                        RawFieldInitializerKind::Shorthand { value, .. }
                        | RawFieldInitializerKind::Explicit { value, .. } => value,
                    };
                    self.target_consumption_span(value, target, true)
                })
            }
            RawExpressionKind::FixedArrayConstruction { elements, .. } => elements
                .iter()
                .find_map(|element| self.target_consumption_span(*element, target, true)),
            RawExpressionKind::EnumConstruction { payload: Some(payload), .. } => {
                self.target_consumption_span(*payload, target, true)
            }
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. }
                if consumes_reference
                    && self.projection_expression_type(id).is_some_and(|ty| !ty.is_copy()) =>
            {
                self.projection_root_reference_span(*base, target)
            }
            _ => None,
        }
    }

    fn projection_root_reference_span(
        &self,
        id: u32,
        target: raw::PlaceId,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if self.bindings.get(&name.text).is_some_and(|binding| binding.place == target) =>
            {
                Some(name.span)
            }
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. } => {
                self.projection_root_reference_span(*base, target)
            }
            _ => None,
        }
    }

    fn reserve_operands(&mut self, additional: usize, at: Span) -> Option<()> {
        if aggregate_operand_budget_violation(self.aggregate_operands, additional) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived aggregate operands exceed the M3 limit of {}",
                    ir::MAX_AGGREGATE_OPERANDS
                ),
                "reduce Struct fields and fixed-array elements",
            );
            return None;
        }
        self.aggregate_operands += additional;
        Some(())
    }

    fn prevalidate_constructor_operands(
        &mut self,
        values: &[raw::ValueId],
        at: Span,
    ) -> Option<Vec<raw::ValueId>> {
        let mut seen = BTreeSet::new();
        let mut consumed = Vec::new();
        for value in values {
            let Some(owner) = self.owners.owner(*value) else { continue };
            if !self.owners.contains(owner) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "aggregate constructor operand owner is unavailable before commit",
                    "construct from only currently pending exact values",
                );
                return None;
            }
            if !seen.insert(owner) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "aggregate constructor attempts to consume one owner more than once",
                    "move each non-Copy field or element exactly once",
                );
                return None;
            }
            consumed.push(*value);
        }
        Some(consumed)
    }

    fn commit_constructor_operands(&mut self, values: &[raw::ValueId]) {
        for value in values {
            self.owners
                .transfer(*value)
                .expect("prevalidated aggregate operand remains pending until infallible commit");
        }
    }

    fn commit_enum(
        &mut self,
        expected: Ty,
        at: Span,
        ordinal: usize,
        payload: Option<raw::ValueId>,
    ) -> Option<raw::ValueId> {
        self.reserve_operands(usize::from(payload.is_some()), at)?;
        let operands = payload.into_iter().collect::<Vec<_>>();
        let consumed = self.prevalidate_constructor_operands(&operands, at)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::EnumConstruct {
                variant: u32::try_from(ordinal).ok()?,
                payload: operands.first().copied(),
                cleanup: None,
            },
        )?;
        self.commit_constructor_operands(&consumed);
        Some(result)
    }

    fn ty_for_layout(&self, id: layout::TypeId) -> Option<Ty> {
        self.node_types.iter().flatten().find(|ty| ty.layout == id).copied()
    }

    fn partial_local_transfer_source(
        &self,
        initializer: u32,
        expected: Ty,
    ) -> Option<raw::PlaceId> {
        if !matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray) {
            return None;
        }
        let RawExpressionKind::Reference { name } = &self.expression(initializer)?.kind else {
            return None;
        };
        let binding = self.bindings.get(&name.text)?;
        (binding.ty == expected
            && self.owners.contains(binding.place)
            && self.partial_roots.contains(&binding.place))
        .then_some(binding.place)
    }

    fn partial_return_transfer_source(&self, value: u32, expected: Ty) -> Option<raw::PlaceId> {
        if !matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray) {
            return None;
        }
        let RawExpressionKind::Reference { name } = &self.expression(value)?.kind else {
            return None;
        };
        let binding = self.bindings.get(&name.text)?;
        (binding.ty == expected
            && self.owners.contains(binding.place)
            && self.partial_roots.contains(&binding.place))
        .then_some(binding.place)
    }

    fn partial_assignment_transfer_source(
        &self,
        value: u32,
        expected: Ty,
        target: raw::PlaceId,
    ) -> Option<raw::PlaceId> {
        let source = self.partial_return_transfer_source(value, expected)?;
        (source != target).then_some(source)
    }

    fn report_partial_transfer_budget(
        &mut self,
        violation: PartialTransferBudgetViolation,
        at: Span,
    ) {
        let (message, guidance) = match violation {
            PartialTransferBudgetViolation::PlaceAccounting => (
                "partial aggregate transfer place accounting overflowed".to_owned(),
                "reduce projected aggregate depth and local transfers",
            ),
            PartialTransferBudgetViolation::Values => (
                "partial aggregate transfer exceeds the per-function value limit".to_owned(),
                "reduce private aggregate expressions and transfers",
            ),
            PartialTransferBudgetViolation::Places => (
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned parameters, expressions, and local declarations",
            ),
            PartialTransferBudgetViolation::Transitions => (
                format!(
                    "derived ownership transitions exceed the per-function M3 limit of {}",
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                ),
                "reduce private aggregate expressions and assignments",
            ),
        };
        self.errors.at("ZRYNA-M3201", at, message, guidance);
    }

    fn lower_partial_local_transfer(
        &mut self,
        source: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<raw::PlaceId> {
        let Some(shape) = self.complete_projection_shape(ty) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate transfer topology exceeds a deterministic resource limit",
                "reduce nested Struct fields, fixed-array lengths, and local transfers",
            );
            return None;
        };
        let existing = self.existing_projection_shape(source, &shape);
        let existing_count = existing.iter().filter(|place| place.is_some()).count();
        let _additional_places = match partial_transfer_budget_preflight(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.reserved_transitions,
            shape.len(),
            existing_count,
        ) {
            Ok(additional_places) => additional_places,
            Err(violation) => {
                self.report_partial_transfer_budget(violation, at);
                return None;
            }
        };
        let next_local = self.next_local.checked_add(1).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate transfer local identity overflowed",
                "reduce private aggregate local declarations",
            );
            None
        })?;

        let source_places = self.materialize_projection_shape(source, &shape, at);
        let value = raw::ValueId(self.next_value);
        self.next_value = self.next_value.checked_add(1).expect("value capacity preflighted");
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        });
        let temporary = raw::PlaceId(
            u32::try_from(self.places.len()).expect("partial transfer place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: temporary,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        self.owners.register(value, temporary).expect("fresh partial transfer temporary owner");
        let temporary_places = self.materialize_projection_shape(temporary, &shape, at);
        self.owners
            .rehome_move_result(value, source)
            .expect("partial transfer source owner available");
        self.migrate_partial_mask(source, temporary, &source_places, &temporary_places);

        let local = raw::PlaceId(
            u32::try_from(self.places.len()).expect("partial transfer place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: local,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local = next_local;
        let local_places = self.materialize_projection_shape(local, &shape, at);
        self.instructions.push(raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::InitializePlace { place: local, value },
        });
        self.owners.rename(value, local).expect("partial transfer temporary owner available");
        self.migrate_partial_mask(temporary, local, &temporary_places, &local_places);
        Some(local)
    }

    fn lower_partial_return_transfer(
        &mut self,
        source: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let Some(shape) = self.complete_projection_shape(ty) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate return topology exceeds a deterministic resource limit",
                "reduce nested Struct fields, fixed-array lengths, and return transfers",
            );
            return None;
        };
        let existing = self.existing_projection_shape(source, &shape);
        let existing_count = existing.iter().filter(|place| place.is_some()).count();
        if let Err(violation) = partial_return_budget_preflight(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.reserved_transitions,
            shape.len(),
            existing_count,
        ) {
            self.report_partial_transfer_budget(violation, at);
            return None;
        }

        let source_places = self.materialize_projection_shape(source, &shape, at);
        let value = raw::ValueId(self.next_value);
        self.next_value = self.next_value.checked_add(1).expect("value capacity preflighted");
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        });
        let temporary = raw::PlaceId(
            u32::try_from(self.places.len()).expect("partial return place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: temporary,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        self.owners.register(value, temporary).expect("fresh partial return temporary owner");
        let temporary_places = self.materialize_projection_shape(temporary, &shape, at);
        self.owners
            .rehome_move_result(value, source)
            .expect("partial return source owner available");
        self.migrate_partial_mask(source, temporary, &source_places, &temporary_places);
        Some(value)
    }

    fn lower_partial_assignment_transfer(
        &mut self,
        source: raw::PlaceId,
        target: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<()> {
        let Some(shape) = self.complete_projection_shape(ty) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate assignment topology exceeds a deterministic resource limit",
                "reduce nested Struct fields, fixed-array lengths, and assignment transfers",
            );
            return None;
        };
        let source_existing = self.existing_projection_shape(source, &shape);
        let target_existing = self.existing_projection_shape(target, &shape);
        if let Err(violation) = partial_assignment_budget_preflight(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.reserved_transitions,
            shape.len(),
            source_existing.iter().filter(|place| place.is_some()).count(),
            target_existing.iter().filter(|place| place.is_some()).count(),
        ) {
            self.report_partial_transfer_budget(violation, at);
            return None;
        }

        let source_places = self.materialize_projection_shape(source, &shape, at);
        let target_places = self.materialize_projection_shape(target, &shape, at);
        let value = raw::ValueId(self.next_value);
        self.next_value = self.next_value.checked_add(1).expect("value capacity preflighted");
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        });
        let temporary = raw::PlaceId(
            u32::try_from(self.places.len())
                .expect("partial assignment place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: temporary,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        self.owners.register(value, temporary).expect("fresh partial assignment temporary owner");
        let temporary_places = self.materialize_projection_shape(temporary, &shape, at);
        self.owners
            .rehome_move_result(value, source)
            .expect("partial assignment source owner available");
        self.migrate_partial_mask(source, temporary, &source_places, &temporary_places);

        self.instructions.push(raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::ReplacePlace { place: target, value },
        });
        self.owners.replace(value, target).expect("partial assignment temporary owner available");
        self.migrate_partial_mask(temporary, target, &temporary_places, &target_places);
        Some(())
    }

    fn reference_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            let wrong_case = self.bindings.keys().any(|key| key.eq_ignore_ascii_case(&name.text));
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                if wrong_case {
                    format!("aggregate value '{}' has the wrong portable ASCII case", name.text)
                } else {
                    format!("aggregate value '{}' is not declared", name.text)
                },
                "reference one exact preceding local using its declared spelling",
            );
            return None;
        };
        if binding.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "aggregate operand has the wrong exact type",
                "use the exact declared field, element, local, or result type",
            );
            return None;
        }
        if expected.is_copy() {
            return self.emit(
                expected,
                at,
                raw::InstructionKind::CopyFromPlace { place: binding.place },
            );
        }
        if !self.whole_root_available(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is moved or only partially available", name.text),
                "move a whole owned aggregate only before moving any of its projections",
            );
            return None;
        }
        let value =
            self.emit(expected, at, raw::InstructionKind::MoveFromPlace { place: binding.place })?;
        self.owners.rehome_move_result(value, binding.place)?;
        Some(value)
    }

    fn preflight_aggregate_subobject_move_site(&mut self, at: Span) -> bool {
        if self.aggregate_subobject_moves == 0 {
            return true;
        }
        self.errors.at(
            "ZRYNA-M3016",
            at,
            "this checkpoint admits only one aggregate-subobject move per function",
            "move one supported Struct or fixed-array subobject into one exact direct local",
        );
        false
    }

    #[allow(clippy::too_many_lines)]
    fn projected_value(
        &mut self,
        id: u32,
        expected: Ty,
        aggregate_context: Option<ProjectedAggregateMoveContext>,
    ) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        let final_return_preflight = if aggregate_context
            == Some(ProjectedAggregateMoveContext::FinalReturn)
            && matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray)
        {
            let Some(preflight) = self.owned_place_preflight(id) else {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "final aggregate-subobject return has no canonical static source path",
                    "return one supported Struct field or constant fixed-array element from a local root",
                );
                return None;
            };
            if preflight.place.is_root || preflight.place.ty != expected {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned projection has the wrong exact contextual type",
                    "return one exact supported Struct field or fixed-array element",
                );
                return None;
            }
            if !self.preflight_projection_available(&preflight) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "owned projection is unavailable or overlaps an already moved subobject",
                    "move each owned field or fixed-array element at most once",
                );
                return None;
            }
            if !self.supported(expected) || !self.preflight_aggregate_subobject_move_site(at) {
                return None;
            }
            let Some(shape) = self.complete_projection_shape(expected) else {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned subobject projection has no finite static topology",
                    "return an acyclic supported Struct or fixed-array projection",
                );
                return None;
            };
            let missing_descendants = if self.places.get(preflight.place.place.0 as usize).is_some()
            {
                self.existing_projection_shape(preflight.place.place, &shape)
                    .iter()
                    .filter(|place| place.is_none())
                    .count()
            } else {
                shape.len()
            };
            if projected_subobject_return_budget_violation(
                self.next_value as usize,
                self.places.len(),
                self.instructions.len(),
                self.reserved_transitions,
                self.cleanup_plans.len(),
                self.cleanup_actions,
                self.owners.pending().len(),
                preflight.missing,
                missing_descendants,
            ) {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "static aggregate-subobject return exceeds an M3 resource limit",
                    "reduce the canonical source path, projected topology, or preceding owned expressions",
                );
                return None;
            }
            Some(shape)
        } else {
            None
        };
        let projection = self.owned_place(id)?;
        if projection.is_root || projection.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection has the wrong exact contextual type",
                "use one exact supported Struct field or fixed-array element",
            );
            return None;
        }
        if !self.projection_available(projection.place, projection.root) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "owned projection is unavailable or overlaps an already moved subobject",
                "move each owned field or fixed-array element at most once",
            );
            return None;
        }
        if expected.is_copy() {
            return self.emit(
                expected,
                at,
                raw::InstructionKind::CopyFromPlace { place: projection.place },
            );
        }
        let aggregate_subobject =
            matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray);
        if aggregate_subobject && aggregate_context.is_none() {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "static aggregate-subobject move requires one exact direct local or final return",
                "initialize one exact private local or return the exact result type from the Struct field or constant fixed-array element",
            );
            return None;
        }
        if aggregate_subobject && !self.preflight_aggregate_subobject_move_site(at) {
            return None;
        }
        if !matches!(
            expected.category,
            TypeCategory::String | TypeCategory::Struct | TypeCategory::FixedArray
        ) || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection type is outside the static subobject move checkpoint",
                "move a String, supported Struct, or supported fixed-array field or constant element into one exact direct local",
            );
            return None;
        }
        if aggregate_subobject {
            let Some(shape) = self.complete_projection_shape(expected) else {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned subobject projection has no finite static topology",
                    "move an acyclic supported Struct or fixed-array projection",
                );
                return None;
            };
            let existing = self.existing_projection_shape(projection.place, &shape);
            let missing = existing.iter().filter(|place| place.is_none()).count();
            let budget_violation = match aggregate_context {
                Some(
                    ProjectedAggregateMoveContext::DirectLocal
                    | ProjectedAggregateMoveContext::ProjectedReplacement,
                ) => projected_subobject_move_budget_violation(
                    self.next_value as usize,
                    self.places.len(),
                    self.instructions.len(),
                    self.reserved_transitions,
                    missing,
                ),
                Some(ProjectedAggregateMoveContext::FinalReturn) | None => false,
            };
            if budget_violation {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "static aggregate-subobject move exceeds an M3 resource limit",
                    "reduce projected aggregate topology or preceding owned expressions",
                );
                return None;
            }
            self.materialize_projection_shape(
                projection.place,
                final_return_preflight.as_deref().unwrap_or(&shape),
                at,
            );
        }
        let value = self.emit(
            expected,
            at,
            raw::InstructionKind::MoveFromPlace { place: projection.place },
        )?;
        if aggregate_subobject {
            self.aggregate_subobject_moves += 1;
        }
        self.moved_projections.insert(projection.place);
        self.partial_roots.insert(projection.root);
        Some(value)
    }

    fn clone_aggregate(&mut self, operand: u32, expected: Ty, at: Span) -> Option<raw::ValueId> {
        if expected.is_copy()
            || !matches!(
                expected.category,
                TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
            )
            || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "structural clone requires one exact supported String-bearing aggregate",
                "clone an acyclic private Struct, Enum, or fixed array containing only bool, i32, String, and supported aggregate nodes",
            );
            return None;
        }
        let operand = self.expression(operand)?.clone();
        let RawExpressionKind::Reference { name } = operand.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), operand.span),
                "structural clone requires an addressable aggregate local root",
                "clone one available aggregate local by name",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("aggregate binding '{}' is not declared in this function", name.text),
                "clone one preceding available aggregate local",
            );
            return None;
        };
        if binding.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "structural clone source has the wrong exact aggregate type",
                "clone a local with the exact contextual aggregate type",
            );
            return None;
        }
        if !self.whole_root_available(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is moved or only partially available", name.text),
                "clone the aggregate only before moving any owned projection",
            );
            return None;
        }

        let pending = self.owners.pending().len();
        let prefix_actions = pending.checked_add(1).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "aggregate clone prefix cleanup overflows its checked action count",
                "reduce simultaneously live owned aggregates",
            );
            None
        })?;
        let _total_actions = pending.checked_add(prefix_actions).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "aggregate clone cleanup accounting overflows",
                "reduce simultaneously live owned aggregates",
            );
            None
        })?;
        if aggregate_clone_budget_violation(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.cleanup_plans.len(),
            self.cleanup_actions,
            pending,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "structural clone exceeds a checked value, place, or cleanup resource limit",
                "reduce simultaneously live owned aggregates or clone sites",
            );
            return None;
        }

        self.emit_aggregate_clone(&binding, expected, at)
    }

    fn emit_aggregate_clone(
        &mut self,
        binding: &Binding,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        self.emit(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place: binding.place,
                cleanup,
                element_cleanup: Some(element_cleanup),
            },
        )
    }

    fn projected_aggregate_assignment_source(
        &mut self,
        value: u32,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let expression = self.expression(value).cloned()?;
        let expression_at = span(self.input.sources(), expression.span);
        if let RawExpressionKind::Reference { name } = expression.kind {
            return self.projected_aggregate_assignment_root_source(
                name,
                expression_at,
                expected,
                target_root,
            );
        }

        self.projected_aggregate_assignment_projection_source(
            value,
            expression_at,
            expected,
            target_root,
        )
    }

    fn projected_aggregate_assignment_root_source(
        &mut self,
        name: syntax::RawIdentifierSyntax,
        expression_at: Span,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let Some(source) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is not declared", name.text),
                "reference one exact preceding aggregate local",
            );
            return None;
        };
        if source.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "projected aggregate assignment source has the wrong exact type",
                "move a whole root or static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate assignment cannot consume its enclosing root",
                "move one aggregate root under a distinct enclosing root into the projection",
            );
            return None;
        }
        if !self.whole_root_available(source.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate assignment source is moved or only partially available",
                "move one distinct fully initialized aggregate root into the projection",
            );
            return None;
        }
        Some(ProjectedAggregateAssignmentSource::MoveRoot { name, at: expression_at })
    }

    fn projected_aggregate_assignment_projection_source(
        &mut self,
        value: u32,
        expression_at: Span,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let Some(source) = self.owned_place_preflight(value) else {
            self.errors.at(
                "ZRYNA-M3013",
                expression_at,
                "projected aggregate assignment requires one whole root or static aggregate subobject source",
                "move one distinct fully initialized exact aggregate root or canonical Struct field or constant fixed-array element into the projection",
            );
            return None;
        };
        if source.place.is_root
            || source.place.ty.is_copy()
            || !matches!(source.place.ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(source.place.ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                expression_at,
                "projected aggregate assignment source is outside the static non-Copy subobject checkpoint",
                "move one supported Struct field or constant fixed-array aggregate element",
            );
            return None;
        }
        if source.place.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                expression_at,
                "projected aggregate assignment source has the wrong exact type",
                "move a static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place.root == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                expression_at,
                "projected aggregate assignment source and target require distinct enclosing roots",
                "move a static aggregate subobject between two distinct local owners",
            );
            return None;
        }
        if !self.preflight_projection_available(&source) {
            self.errors.at(
                "ZRYNA-M3014",
                expression_at,
                "projected aggregate assignment source subobject is moved or overlaps a moved projection",
                "move one initialized available static aggregate subobject",
            );
            return None;
        }
        if !self.preflight_aggregate_subobject_move_site(expression_at) {
            return None;
        }
        let Some(shape) = self.complete_projection_shape(expected) else {
            self.errors.at(
                "ZRYNA-M3016",
                expression_at,
                "projected aggregate assignment source has no finite static topology",
                "move an acyclic supported Struct or fixed-array subobject",
            );
            return None;
        };
        let existing = self.existing_projection_shape(source.place.place, &shape);
        let missing_descendant_places = existing.iter().filter(|place| place.is_none()).count();
        Some(ProjectedAggregateAssignmentSource::MoveProjection {
            expression: value,
            missing_path_places: source.missing,
            missing_descendant_places,
        })
    }

    fn projected_aggregate_clone_assignment_source(
        &mut self,
        operand: u32,
        expected: Ty,
        target_root: raw::PlaceId,
        clone_span: Span,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let expression = self.expression(operand).cloned()?;
        if let RawExpressionKind::Reference { name } = expression.kind {
            return self.projected_aggregate_clone_root_assignment_source(
                &name,
                expected,
                target_root,
                clone_span,
            );
        }

        let Some(source) = self.owned_place_preflight(operand) else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment requires one whole root or static aggregate subobject source",
                "clone one distinct fully initialized exact aggregate root or canonical Struct field or constant fixed-array element into the projection",
            );
            return None;
        };
        if source.place.is_root
            || source.place.ty.is_copy()
            || !matches!(source.place.ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(source.place.ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source is outside the static non-Copy subobject checkpoint",
                "clone one supported Struct field or constant fixed-array aggregate element",
            );
            return None;
        }
        if source.place.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source has the wrong exact type",
                "clone a static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place.root == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source and target require distinct enclosing roots",
                "clone a static aggregate subobject between two distinct local owners",
            );
            return None;
        }
        if !self.preflight_projection_available(&source) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source subobject is moved or overlaps a moved projection",
                "clone one initialized available static aggregate subobject",
            );
            return None;
        }
        if !self.function.parameters.is_empty() {
            self.errors.at(
                "ZRYNA-M3016",
                clone_span,
                "projected aggregate clone assignment does not admit function parameters",
                "clone one static aggregate subobject in a parameter-free private straight-line function",
            );
            return None;
        }
        Some(ProjectedAggregateAssignmentSource::CloneProjection {
            expression: operand,
            at: clone_span,
            missing_path_places: source.missing,
        })
    }

    fn projected_aggregate_clone_root_assignment_source(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        expected: Ty,
        target_root: raw::PlaceId,
        clone_span: Span,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let Some(source) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is not declared", name.text),
                "clone one exact preceding aggregate local",
            );
            return None;
        };
        if source.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "projected aggregate clone assignment source has the wrong exact type",
                "clone a whole root or static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate clone assignment source cannot be its enclosing root",
                "clone one distinct aggregate root or static subobject into the projection",
            );
            return None;
        }
        if !self.whole_root_available(source.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate clone assignment source is moved or only partially available",
                "clone one distinct fully initialized aggregate root into the projection",
            );
            return None;
        }
        Some(ProjectedAggregateAssignmentSource::CloneRoot { binding: source, at: clone_span })
    }

    fn projected_aggregate_assignment_value_source(
        &mut self,
        value: u32,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let expression = self.expression(value).cloned()?;
        if let RawExpressionKind::Clone { value: operand, .. } = expression.kind {
            let clone_span = span(self.input.sources(), expression.span);
            self.projected_aggregate_clone_assignment_source(
                operand,
                expected,
                target_root,
                clone_span,
            )
        } else {
            self.projected_aggregate_assignment_source(value, expected, target_root)
        }
    }

    fn projected_aggregate_assignment_exceeds_budget(
        &self,
        source: &ProjectedAggregateAssignmentSource,
        missing_path_places: usize,
    ) -> bool {
        match source {
            ProjectedAggregateAssignmentSource::MoveRoot { .. } => {
                projected_aggregate_assignment_budget_violation(
                    self.next_value as usize,
                    self.places.len(),
                    self.instructions.len(),
                    self.reserved_transitions,
                    missing_path_places,
                )
            }
            ProjectedAggregateAssignmentSource::MoveProjection {
                missing_path_places: source_missing_path_places,
                missing_descendant_places,
                ..
            } => projected_subobject_assignment_budget_violation(
                self.next_value as usize,
                self.places.len(),
                self.instructions.len(),
                self.reserved_transitions,
                *source_missing_path_places,
                *missing_descendant_places,
                missing_path_places,
            ),
            ProjectedAggregateAssignmentSource::CloneRoot { .. } => {
                projected_aggregate_clone_assignment_budget_violation(
                    self.next_value as usize,
                    self.places.len(),
                    self.instructions.len(),
                    self.reserved_transitions,
                    self.cleanup_plans.len(),
                    self.cleanup_actions,
                    self.owners.pending().len(),
                    0,
                    missing_path_places,
                )
            }
            ProjectedAggregateAssignmentSource::CloneProjection {
                missing_path_places: source_missing_path_places,
                ..
            } => projected_aggregate_clone_assignment_budget_violation(
                self.next_value as usize,
                self.places.len(),
                self.instructions.len(),
                self.reserved_transitions,
                self.cleanup_plans.len(),
                self.cleanup_actions,
                self.owners.pending().len(),
                *source_missing_path_places,
                missing_path_places,
            ),
        }
    }

    fn emit_projected_aggregate_clone(
        &mut self,
        expression: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let projection = self.owned_place(expression)?;
        debug_assert_eq!(projection.ty, expected);
        debug_assert!(!projection.is_root);
        debug_assert!(self.projection_available(projection.place, projection.root));
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        self.emit(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place: projection.place,
                cleanup,
                element_cleanup: Some(element_cleanup),
            },
        )
    }

    fn projected_aggregate_clone_site_available(&mut self, at: Span) -> bool {
        if self.projected_aggregate_clones == 0 {
            return true;
        }
        self.errors.at(
            "ZRYNA-M3016",
            at,
            "this checkpoint admits only one projected aggregate clone per function",
            "clone one static Struct or fixed-array subobject into one exact direct local or distinct-root static projection",
        );
        false
    }

    #[allow(clippy::too_many_lines)]
    fn lower_projected_aggregate_assignment(
        &mut self,
        target: u32,
        value: u32,
        at: Span,
    ) -> Option<()> {
        if self.projected_aggregate_assignments != 0 {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "this checkpoint admits only one projected aggregate assignment per function",
                "move one complete static aggregate subobject, or move or clone one complete root, into one static aggregate projection",
            );
            return None;
        }
        let Some(target_preflight) = self.owned_place_preflight(target) else {
            let _ = self.owned_place(target);
            return None;
        };
        let target_ty = target_preflight.place.ty;
        if target_preflight.place.is_root
            || target_ty.is_copy()
            || !matches!(target_ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(target_ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "projected aggregate assignment requires one exact non-Copy static Struct or fixed-array target",
                "assign one complete exact aggregate root or distinct static aggregate subobject to a static Struct field or constant fixed-array element",
            );
            return None;
        }
        if !target_preflight.place.mutable
            || !self.preflight_projection_available(&target_preflight)
        {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected aggregate assignment target is immutable, moved, or overlaps a moved subobject",
                "replace one initialized mutable available static aggregate projection",
            );
            return None;
        }
        let source = self.projected_aggregate_assignment_value_source(
            value,
            target_ty,
            target_preflight.place.root,
        )?;
        let clones_projection =
            matches!(&source, ProjectedAggregateAssignmentSource::CloneProjection { .. });
        if clones_projection && !self.projected_aggregate_clone_site_available(at) {
            return None;
        }
        if self.projected_aggregate_assignment_exceeds_budget(&source, target_preflight.missing) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "projected aggregate assignment exceeds a checked value, place, transition, or cleanup resource limit",
                "reduce static projection depth, simultaneously live owners, or preceding owned expressions",
            );
            return None;
        }

        let target = self.owned_place(target)?;
        debug_assert_eq!(target.ty, target_ty);
        debug_assert!(!target.is_root);
        if !self.reserve_transition(at) {
            return None;
        }
        let prepared = match &source {
            ProjectedAggregateAssignmentSource::MoveRoot { name, at } => {
                self.reference_value(name, target_ty, *at)
            }
            ProjectedAggregateAssignmentSource::MoveProjection { expression, .. } => self
                .projected_value(
                    *expression,
                    target_ty,
                    Some(ProjectedAggregateMoveContext::ProjectedReplacement),
                ),
            ProjectedAggregateAssignmentSource::CloneRoot { binding, at } => {
                self.emit_aggregate_clone(binding, target_ty, *at)
            }
            ProjectedAggregateAssignmentSource::CloneProjection { expression, at, .. } => {
                self.emit_projected_aggregate_clone(*expression, target_ty, *at)
            }
        };
        self.release_transition();
        let prepared = prepared?;
        if !self.emit_effect(
            at,
            raw::InstructionKind::ReplacePlace { place: target.place, value: prepared },
        ) {
            return None;
        }
        if self.owners.transfer(prepared).is_none() {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected aggregate assignment has no distinct prepared owner",
                "move one available static aggregate subobject, or move or clone one independently owned root, into the projection",
            );
            return None;
        }
        self.projected_aggregate_assignments += 1;
        if clones_projection {
            self.projected_aggregate_clones += 1;
        }
        Some(())
    }

    fn clone_projected_aggregate_local(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        if !self.projected_aggregate_clone_site_available(at) {
            return None;
        }
        if expected.is_copy()
            || !matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "projected structural clone requires one exact supported non-Copy aggregate",
                "clone an acyclic static Struct or fixed-array subobject containing only bool, i32, String, and supported aggregate nodes",
            );
            return None;
        }
        let Some(preflight) = self.owned_place_preflight(operand) else {
            let _ = self.owned_place(operand);
            return None;
        };
        if preflight.place.is_root || preflight.place.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "projected structural clone source has the wrong exact contextual type",
                "clone one exact supported Struct field or constant fixed-array element",
            );
            return None;
        }
        if !self.preflight_projection_available(&preflight) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected aggregate clone source is unavailable or overlaps a moved subobject",
                "clone only one initialized available static aggregate projection",
            );
            return None;
        }
        let pending = self.owners.pending().len();
        if projected_aggregate_clone_budget_violation(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.reserved_transitions,
            self.cleanup_plans.len(),
            self.cleanup_actions,
            pending,
            preflight.missing,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "projected structural clone exceeds a checked value, place, transition, or cleanup resource limit",
                "reduce static projection depth, simultaneously live owners, or projected clone sites",
            );
            return None;
        }

        let projection = self.owned_place(operand)?;
        debug_assert_eq!(projection.ty, expected);
        debug_assert!(!projection.is_root);
        debug_assert!(self.projection_available(projection.place, projection.root));
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place: projection.place,
                cleanup,
                element_cleanup: Some(element_cleanup),
            },
        )?;
        self.projected_aggregate_clones += 1;
        Some(result)
    }

    fn clone_projected_string(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let projection = self.owned_place(operand)?;
        if projection.is_root
            || expected.category != TypeCategory::String
            || projection.ty != expected
        {
            self.errors.at(
                "ZRYNA-M3012",
                at,
                "projected String clone requires one exact static String leaf",
                "clone an initialized Struct field or constant fixed-array String element",
            );
            return None;
        }
        if !self.projection_available(projection.place, projection.root) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected String clone source is moved or overlaps a moved subobject",
                "clone only an initialized available static String projection",
            );
            return None;
        }
        let pending = self.owners.pending().len();
        if projected_string_clone_budget_violation(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.reserved_transitions,
            self.cleanup_plans.len(),
            self.cleanup_actions,
            pending,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "projected String clone exceeds a checked value, place, transition, or cleanup limit",
                "reduce simultaneously live owned aggregates or projected clone sites",
            );
            return None;
        }
        let cleanup = self.push_cleanup(at, None)?;
        self.emit(
            expected,
            at,
            raw::InstructionKind::StringClone { place: projection.place, cleanup },
        )
    }

    fn push_aggregate_clone_prefix_cleanup(
        &mut self,
        at: Span,
        result_owner: raw::PlaceId,
    ) -> Option<raw::CleanupPlanId> {
        let action_count = self.owners.pending().len().checked_add(1)?;
        let id = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).ok()?);
        let actions =
            std::iter::once(raw::DropAction::DropAggregateInitializedPrefix(result_owner))
                .chain(self.owners.pending().iter().rev().copied().map(raw::DropAction::DropPlace))
                .collect();
        self.cleanup_plans.push(raw::CleanupPlan { id, span: at, actions });
        self.cleanup_actions += action_count;
        Some(id)
    }

    fn value(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } if expected.category == TypeCategory::Bool => {
                self.emit(expected, at, raw::InstructionKind::BoolLiteral(value))
            }
            RawExpressionKind::I32Literal { spelling }
                if expected.category == TypeCategory::I32 =>
            {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3016",
                        at,
                        "aggregate leaf integer is outside i32",
                        "use one in-range i32 leaf",
                    );
                    None
                })?;
                self.emit(expected, at, raw::InstructionKind::I32Literal(value))
            }
            RawExpressionKind::StringLiteral { spelling }
                if expected.category == TypeCategory::String =>
            {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes().to_vec();
                let cleanup = self.push_cleanup(at, None)?;
                self.emit(expected, at, raw::InstructionKind::StringFromUtf8 { bytes, cleanup })
            }
            RawExpressionKind::Reference { name } => self.reference_value(&name, expected, at),
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                self.projected_value(id, expected, None)
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::String => {
                self.clone_projected_string(value, expected, at)
            }
            RawExpressionKind::Clone { value, .. } => self.clone_aggregate(value, expected, at),
            RawExpressionKind::StructConstruction { type_name, fields, .. }
                if expected.category == TypeCategory::Struct =>
            {
                self.struct_value(&type_name, &fields, expected, at)
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. }
                if expected.category == TypeCategory::FixedArray =>
            {
                self.array_value(type_syntax, &elements, expected, at)
            }
            RawExpressionKind::EnumConstruction { type_name, variant, payload, .. }
                if expected.category == TypeCategory::Enum =>
            {
                self.enum_value(&type_name, &variant, payload, expected, at)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "expression is outside private owned Struct/Enum/FixedArray lowering",
                    "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors",
                );
                None
            }
        }
    }

    fn struct_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        fields: &[syntax::RawFieldInitializer],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let decl = self
            .declarations
            .iter()
            .find(|decl| decl.module == self.module && decl.name == name.text)
            .cloned();
        let Some(decl) = decl else {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                format!("'{}' is not an exact module-local owned struct", name.text),
                "construct one exact supported struct type",
            );
            return None;
        };
        let actual = self.node_types.get(decl.node.0 as usize).and_then(|ty| *ty)?;
        if actual != expected || !self.supported(actual) {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "struct constructor type or ownership graph is outside the exact supported slice",
                "use an acyclic struct containing only bool, i32, String, or supported fixed arrays",
            );
            return None;
        }
        let RawDataDeclarationKind::Struct { fields: declared, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "owned struct construction names an enum",
                "construct one exact supported struct",
            );
            return None;
        };
        let declared = declared.clone();
        let mut ordered = vec![None; declared.len()];
        for field in fields {
            let (field_name, expression) = match &field.kind {
                RawFieldInitializerKind::Shorthand { name, value }
                | RawFieldInitializerKind::Explicit { name, value, .. } => (&name.text, *value),
            };
            let Some((ordinal, declaration)) = declared
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.name.text == *field_name)
            else {
                self.errors.at(
                    "ZRYNA-M3016",
                    span(self.input.sources(), field.span),
                    format!("struct '{}' has no field '{field_name}'", name.text),
                    "initialize every exact declared field once",
                );
                return None;
            };
            if ordered[ordinal].is_some() {
                self.errors.at(
                    "ZRYNA-M3016",
                    span(self.input.sources(), field.span),
                    format!("field '{field_name}' is initialized more than once"),
                    "initialize every exact declared field once",
                );
                return None;
            }
            let field_ty = semantic_type(
                self.file,
                declaration.type_syntax,
                self.module,
                self.declarations,
                self.graph,
                self.node_types,
                self.errors,
            )?;
            ordered[ordinal] = Some(self.value(expression, field_ty)?);
        }
        if let Some((ordinal, _)) = ordered.iter().enumerate().find(|(_, value)| value.is_none()) {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!("field '{}' is not initialized", declared[ordinal].name.text),
                "initialize every exact declared field once",
            );
            return None;
        }
        let values = ordered.into_iter().flatten().collect::<Vec<_>>();
        self.reserve_operands(values.len(), at)?;
        let consumed = self.prevalidate_constructor_operands(&values, at)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::StructConstruct { fields: values.clone(), cleanup: None },
        )?;
        self.commit_constructor_operands(&consumed);
        Some(result)
    }

    fn array_value(
        &mut self,
        type_syntax: u32,
        elements: &[u32],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let actual = semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        if actual != expected || !self.supported(actual) {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "fixed-array constructor type or ownership graph differs from context",
                "construct the exact supported fixed-array type",
            );
            return None;
        }
        let record = self.layouts.type_by_id(actual.layout)?;
        let length = usize::try_from(record.array_length()?).ok()?;
        if elements.len() != length {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!(
                    "fixed-array constructor has {} elements but requires {length}",
                    elements.len()
                ),
                "provide exactly the fixed-array length",
            );
            return None;
        }
        let element = self.ty_for_layout(record.referenced_type()?)?;
        let mut values = Vec::with_capacity(elements.len());
        for expression in elements {
            values.push(self.value(*expression, element)?);
        }
        self.reserve_operands(values.len(), at)?;
        let consumed = self.prevalidate_constructor_operands(&values, at)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::FixedArrayConstruct { elements: values.clone(), cleanup: None },
        )?;
        self.commit_constructor_operands(&consumed);
        Some(result)
    }

    fn enum_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        variant_name: &syntax::RawIdentifierSyntax,
        payload: Option<u32>,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let Some(decl) = self
            .declarations
            .iter()
            .find(|decl| decl.module == self.module && decl.name == name.text)
            .cloned()
        else {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                format!("'{}' is not a module-local enum type", name.text),
                "construct one exact declared enum variant",
            );
            return None;
        };
        let Some(actual) = self.node_types.get(decl.node.0 as usize).and_then(|ty| *ty) else {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "enum constructor type has no authenticated semantic identity",
                "construct one exact supported enum type",
            );
            return None;
        };
        if actual != expected {
            self.errors.at(
                "ZRYNA-M3007",
                span(self.input.sources(), name.span),
                "enum constructor has a different exact result type",
                "construct the exact enum required by this context",
            );
            return None;
        }
        if !owned_enum_graph_is_supported(actual, self.layouts) {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "enum constructor type or payload graph is outside the exact supported slice",
                "use an acyclic enum with payloadless variants or bool, i32, String, Struct, or fixed-array payloads",
            );
            return None;
        }
        let RawDataDeclarationKind::Enum { variants, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                "owned enum construction names a struct",
                "construct a declared enum variant",
            );
            return None;
        };
        let Some((ordinal, variant)) =
            variants.iter().enumerate().find(|(_, variant)| variant.name.text == variant_name.text)
        else {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), variant_name.span),
                format!("enum '{}' has no variant '{}'", name.text, variant_name.text),
                "use one exact declared variant",
            );
            return None;
        };
        let payload_value = match (variant.payload_type, payload) {
            (None, None) => None,
            (Some(type_syntax), Some(expression)) => {
                let payload_ty = semantic_type(
                    self.file,
                    type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?;
                if !aggregate_graph_is_supported(payload_ty, self.layouts, &mut BTreeSet::new()) {
                    self.errors.at(
                        "ZRYNA-M3016",
                        span(self.input.sources(), variant_name.span),
                        "enum payload graph is outside the private owned aggregate slice",
                        "use only bool, i32, String, Struct, or fixed-array payloads",
                    );
                    return None;
                }
                Some(self.value(expression, payload_ty)?)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3005",
                    at,
                    "enum payload presence does not match the declared variant",
                    "supply exactly one payload only for a payload variant",
                );
                return None;
            }
        };
        self.commit_enum(expected, at, ordinal, payload_value)
    }
}

fn is_private_owned_enum_payload_move_candidate(function: &syntax::RawFunctionSyntax) -> bool {
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    let Some(statement) = root
        .statements
        .first()
        .and_then(|id| usize::try_from(*id).ok())
        .and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::LocalDeclaration { initializer, .. } = statement.kind else {
        return false;
    };
    usize::try_from(initializer)
        .ok()
        .and_then(|index| function.body.expressions.get(index))
        .is_some_and(|expression| matches!(expression.kind, RawExpressionKind::Match { .. }))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_owned_enum_payload_move_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let function_span = span(input.sources(), function.span);
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let [local_id, return_id] = root.statements.as_slice() else {
        errors.at(
            "ZRYNA-M3016",
            function_span,
            "owned enum payload extraction requires one local initializer and one final return",
            "bind the one-arm match result to one exact local, then return that local",
        );
        return None;
    };
    let local_statement =
        usize::try_from(*local_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration {
        mutable,
        name: local_name,
        type_syntax,
        initializer,
        ..
    } = &local_statement.kind
    else {
        return None;
    };
    if *mutable {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), local_statement.span),
            "owned enum payload extraction requires an immutable direct local",
            "declare the exact match result with const",
        );
        return None;
    }
    let local_ty =
        semantic_type(file, *type_syntax, module, declarations, graph, node_types, errors)?;
    if local_ty != result
        || result.is_copy()
        || !matches!(result.category, TypeCategory::Struct | TypeCategory::FixedArray)
        || !aggregate_graph_is_supported(result, layouts, &mut BTreeSet::new())
    {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), local_statement.span),
            "enum payload result is outside the exact owned Struct/fixed-array extraction slice",
            "use one exact acyclic non-Copy Struct or fixed array with bool, i32, and String leaves",
        );
        return None;
    }
    let return_statement =
        usize::try_from(*return_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Return { value: returned, .. } = return_statement.kind else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), return_statement.span),
            "owned enum payload extraction must end with the direct local return",
            "return the exact initialized payload local as the final statement",
        );
        return None;
    };
    let returned_expression =
        usize::try_from(returned).ok().and_then(|index| function.body.expressions.get(index))?;
    if !matches!(
        &returned_expression.kind,
        RawExpressionKind::Reference { name } if name.text == local_name.text
    ) {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), returned_expression.span),
            "owned enum payload extraction continuation must return its exact local",
            "return the match-initialized local without another expression",
        );
        return None;
    }
    let match_expression = usize::try_from(*initializer)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Match { scrutinee, arms, .. } = &match_expression.kind else {
        return None;
    };
    let [parameter] = function.parameters.as_slice() else {
        errors.at(
            "ZRYNA-M3016",
            function_span,
            "owned enum payload extraction requires one exact enum source parameter",
            "pass one single-variant owned enum source",
        );
        return None;
    };
    if parameter.name.text.eq_ignore_ascii_case(&local_name.text) {
        errors.at(
            "ZRYNA-M3002",
            span(input.sources(), local_name.span),
            format!("local '{}' collides under portable ASCII case folding", local_name.text),
            "give the result local a name distinct from the source parameter",
        );
        return None;
    }
    let source_ty = semantic_type(
        file,
        parameter.type_syntax,
        module,
        declarations,
        graph,
        node_types,
        errors,
    )?;
    let source_record = layouts.type_by_id(source_ty.layout)?;
    let Some((nominal_module, nominal_declaration)) = source_record.nominal_identity() else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), parameter.span),
            "owned payload source is not one exact nominal enum",
            "use a declared enum with exactly one payload variant",
        );
        return None;
    };
    if source_record.category() != TypeCategory::Enum
        || source_ty.is_copy()
        || usize::try_from(nominal_module).ok() != Some(module)
    {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), parameter.span),
            "owned payload source is outside the private single-variant enum slice",
            "use one private same-module non-Copy enum source",
        );
        return None;
    }
    let enum_decl = declarations.iter().find(|decl| {
        decl.module == module && u32::try_from(decl.declaration).ok() == Some(nominal_declaration)
    })?;
    let RawDataDeclarationKind::Enum { variants, .. } =
        &file.data_declarations()[enum_decl.declaration].kind
    else {
        return None;
    };
    let [variant] = variants.as_slice() else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), file.data_declarations()[enum_decl.declaration].span),
            "owned payload extraction requires an enum with exactly one variant",
            "use one enum containing exactly one payload-bearing variant",
        );
        return None;
    };
    let Some(payload_type) = variant.payload_type else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), variant.span),
            "owned payload extraction requires a payload-bearing variant",
            "give the enum's only variant the exact result payload type",
        );
        return None;
    };
    let payload_ty =
        semantic_type(file, payload_type, module, declarations, graph, node_types, errors)?;
    if payload_ty != result {
        errors.at(
            "ZRYNA-M3007",
            span(input.sources(), variant.span),
            "enum payload type does not match the exact extracted local type",
            "use the variant's exact payload type for the local and function result",
        );
        return None;
    }
    let scrutinee_expression =
        usize::try_from(*scrutinee).ok().and_then(|index| function.body.expressions.get(index))?;
    if !matches!(
        &scrutinee_expression.kind,
        RawExpressionKind::Reference { name } if name.text == parameter.name.text
    ) {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expression.span),
            "owned payload match must refine the exact source parameter",
            "match the one declared enum source directly",
        );
        return None;
    }
    let [arm] = arms.as_slice() else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), match_expression.span),
            "owned payload match requires exactly one exhaustive arm",
            "provide the enum's only variant exactly once",
        );
        return None;
    };
    let Some(binding) = arm.binding.as_ref() else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), arm.span),
            "owned payload arm must bind its payload",
            "bind one name and return that exact binding from the arm",
        );
        return None;
    };
    if arm.type_name.text != enum_decl.name || arm.variant.text != variant.name.text {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), arm.span),
            "owned payload arm does not name the source enum's only variant",
            "match the exact enum and variant spelling",
        );
        return None;
    }
    if binding.text.eq_ignore_ascii_case(&parameter.name.text)
        || binding.text.eq_ignore_ascii_case(&local_name.text)
    {
        errors.at(
            "ZRYNA-M3002",
            span(input.sources(), binding.span),
            format!("match binding '{}' collides under portable ASCII case folding", binding.text),
            "give the payload binding a distinct portable name",
        );
        return None;
    }
    let arm_expression =
        usize::try_from(arm.value).ok().and_then(|index| function.body.expressions.get(index))?;
    if !matches!(
        &arm_expression.kind,
        RawExpressionKind::Reference { name } if name.text == binding.text
    ) {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), arm_expression.span),
            "owned payload arm must yield its exact bound payload",
            "return the payload binding directly from the one match arm",
        );
        return None;
    }

    let Some(shape) = complete_owned_projection_shape(payload_ty, layouts) else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), variant.span),
            "enum payload topology is outside the bounded owned aggregate slice",
            "use one acyclic Struct or fixed array with bool, i32, and String leaves",
        );
        return None;
    };
    if enum_payload_move_resource_violation(shape.len()) {
        errors.at(
            "ZRYNA-M3201",
            span(input.sources(), match_expression.span),
            "derived enum payload extraction exceeds an M3 function resource limit",
            "reduce the payload's static Struct/fixed-array topology",
        );
        return None;
    }

    let parameter_span = span(input.sources(), parameter.span);
    let arm_span = span(input.sources(), arm.span);
    let local_span = span(input.sources(), local_statement.span);
    let return_span = span(input.sources(), return_statement.span);
    let source_place = raw::PlaceId(0);
    let payload_place = raw::PlaceId(1);
    let mut places = vec![
        raw::Place {
            id: source_place,
            ty: source_ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: payload_place,
            ty: payload_ty.ir,
            span: span(input.sources(), binding.span),
            kind: raw::PlaceKind::EnumPayload { base: source_place, variant: 0 },
        },
    ];
    let mut descendants = Vec::<raw::PlaceId>::with_capacity(shape.len());
    for entry in &shape {
        let base = entry.parent.map_or(payload_place, |index| descendants[index]);
        let kind = match entry.kind {
            OwnedStaticProjectionKind::StructField { ordinal } => {
                raw::PlaceKind::StructField { base, ordinal }
            }
            OwnedStaticProjectionKind::FixedArrayConstant { index } => {
                raw::PlaceKind::FixedArrayConstant { base, index }
            }
        };
        let id = raw::PlaceId(u32::try_from(places.len()).ok()?);
        places.push(raw::Place { id, ty: entry.ty.ir, span: arm_span, kind });
        descendants.push(id);
    }
    let moved_value = raw::ValueId(1);
    let moved_owner = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: moved_owner,
        ty: payload_ty.ir,
        span: arm_span,
        kind: raw::PlaceKind::Temporary(moved_value),
    });
    let local_place = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: local_place,
        ty: payload_ty.ir,
        span: local_span,
        kind: raw::PlaceKind::Local(0),
    });
    let returned_value = raw::ValueId(2);
    let returned_owner = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: returned_owner,
        ty: payload_ty.ir,
        span: return_span,
        kind: raw::PlaceKind::Temporary(returned_value),
    });

    let cleanup = raw::CleanupPlanId(0);
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: function_span,
        parameters: vec![raw::ValueDefinition {
            id: raw::ValueId(0),
            ty: source_ty.ir,
            span: parameter_span,
        }],
        borrow_parameters: Vec::new(),
        result: result.ir,
        places,
        blocks: vec![
            raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminators: vec![raw::SpannedTerminator {
                    span: span(input.sources(), match_expression.span),
                    kind: raw::Terminator::EnumMatch {
                        place: source_place,
                        arms: vec![raw::EnumArm {
                            variant: 0,
                            edge: raw::Edge { target: raw::BlockId(1), arguments: Vec::new() },
                        }],
                    },
                }],
            },
            raw::Block {
                id: raw::BlockId(1),
                parameters: Vec::new(),
                instructions: vec![
                    raw::Instruction {
                        result: Some(raw::ValueDefinition {
                            id: moved_value,
                            ty: payload_ty.ir,
                            span: arm_span,
                        }),
                        span: arm_span,
                        kind: raw::InstructionKind::MoveFromPlace { place: payload_place },
                    },
                    raw::Instruction {
                        result: None,
                        span: local_span,
                        kind: raw::InstructionKind::InitializePlace {
                            place: local_place,
                            value: moved_value,
                        },
                    },
                    raw::Instruction {
                        result: None,
                        span: arm_span,
                        kind: raw::InstructionKind::DropPlace { place: source_place },
                    },
                ],
                terminators: vec![raw::SpannedTerminator {
                    span: arm_span,
                    kind: raw::Terminator::Jump(raw::Edge {
                        target: raw::BlockId(2),
                        arguments: Vec::new(),
                    }),
                }],
            },
            raw::Block {
                id: raw::BlockId(2),
                parameters: Vec::new(),
                instructions: vec![raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: returned_value,
                        ty: payload_ty.ir,
                        span: return_span,
                    }),
                    span: return_span,
                    kind: raw::InstructionKind::MoveFromPlace { place: local_place },
                }],
                terminators: vec![raw::SpannedTerminator {
                    span: return_span,
                    kind: raw::Terminator::Return { value: returned_value, cleanup },
                }],
            },
        ],
        cleanup_plans: vec![raw::CleanupPlan {
            id: cleanup,
            span: return_span,
            actions: Vec::new(),
        }],
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_owned_aggregate_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    if !if result.category == TypeCategory::Enum {
        owned_enum_graph_is_supported(result, layouts)
    } else {
        aggregate_graph_is_supported(result, layouts, &mut BTreeSet::new())
    } {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.span),
            "owned aggregate graph contains an unsupported nested enum, Vec, handle, borrow, or cycle",
            "use an acyclic Struct/Enum/FixedArray graph with only bool, i32, and String leaves",
        );
        return None;
    }
    let file = &input.syntax().files()[module];
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    if result.category == TypeCategory::Enum && !function.parameters.is_empty() {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.parameters[0].span),
            "private owned enum functions do not admit parameters",
            "construct the enum from literals and explicitly typed initialized locals",
        );
        return None;
    }
    let mut lowerer = PrivateOwnedAggregateLowerer {
        input,
        file,
        function,
        module,
        declarations,
        graph,
        node_types,
        layouts,
        errors,
        bindings: BTreeMap::new(),
        projections: BTreeMap::new(),
        moved_projections: BTreeSet::new(),
        partial_roots: BTreeSet::new(),
        places: Vec::new(),
        instructions: Vec::new(),
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        aggregate_operands: 0,
        aggregate_subobject_moves: 0,
        projected_aggregate_clones: 0,
        projected_aggregate_assignments: 0,
        reserved_transitions: 0,
        owners: OwnerState::default(),
        next_value: 0,
        next_local: 0,
    };
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            lowerer.errors,
        )?;
        if !ty.is_copy() || !matches!(ty.category, TypeCategory::Bool | TypeCategory::I32) {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), parameter.span),
                "owned aggregate functions do not admit owned or aggregate parameters",
                "use only optional bool/i32 parameters in this private checkpoint",
            );
            return None;
        }
        if lowerer.bindings.keys().any(|name| name.eq_ignore_ascii_case(&parameter.name.text)) {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!(
                    "parameter '{}' collides under portable ASCII case folding",
                    parameter.name.text
                ),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        if lowerer.next_value as usize >= ir::MAX_VALUES_PER_FUNCTION
            || lowerer.places.len() >= ir::MAX_PLACES_PER_FUNCTION
        {
            lowerer.errors.at(
                "ZRYNA-M3201",
                span(input.sources(), parameter.span),
                "derived aggregate parameter storage exceeds an M3 resource limit",
                "reduce private aggregate parameters",
            );
            return None;
        }
        let value = raw::ValueId(lowerer.next_value);
        lowerer.next_value += 1;
        parameters.push(raw::ValueDefinition {
            id: value,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
        });
        let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    }
    let mut returned = None;
    let final_statement = root.statements.last().copied();
    let return_count = root
        .statements
        .iter()
        .filter(|statement_id| {
            usize::try_from(**statement_id)
                .ok()
                .and_then(|index| function.body.statements.get(index))
                .is_some_and(|statement| matches!(statement.kind, RawStatementKind::Return { .. }))
        })
        .count();
    for statement_id in &root.statements {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        match &statement.kind {
            RawStatementKind::LocalDeclaration {
                mutable, name, type_syntax, initializer, ..
            } => {
                if lowerer.bindings.keys().any(|key| key.eq_ignore_ascii_case(&name.text)) {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!(
                            "binding '{}' collides under portable ASCII case folding",
                            name.text
                        ),
                        "give every binding one portable case-insensitive unique name",
                    );
                    return None;
                }
                let ty = semantic_type(
                    file,
                    *type_syntax,
                    module,
                    declarations,
                    graph,
                    node_types,
                    lowerer.errors,
                )?;
                if !lowerer.supported(ty) {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "local type is outside the private owned aggregate graph",
                        "use bool, i32, String, or a supported Struct/Enum/FixedArray type",
                    );
                    return None;
                }
                let statement_span = span(input.sources(), statement.span);
                if let Some(source) = lowerer.partial_local_transfer_source(*initializer, ty) {
                    let place = lowerer.lower_partial_local_transfer(source, ty, statement_span)?;
                    lowerer
                        .bindings
                        .insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
                    continue;
                }
                let aggregate_projection_local =
                    matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
                        && lowerer.expression(*initializer).is_some_and(|expression| {
                            matches!(
                                &expression.kind,
                                RawExpressionKind::FieldAccess { .. }
                                    | RawExpressionKind::Index { .. }
                            )
                        });
                let projected_aggregate_clone_operand =
                    matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
                        .then(|| lowerer.expression(*initializer))
                        .flatten()
                        .and_then(|expression| match &expression.kind {
                            RawExpressionKind::Clone { value, .. }
                                if lowerer.expression(*value).is_some_and(|operand| {
                                    matches!(
                                        &operand.kind,
                                        RawExpressionKind::FieldAccess { .. }
                                            | RawExpressionKind::Index { .. }
                                    )
                                }) =>
                            {
                                Some((*value, span(input.sources(), expression.span)))
                            }
                            _ => None,
                        });
                let value = if let Some((operand, clone_span)) = projected_aggregate_clone_operand {
                    lowerer.clone_projected_aggregate_local(operand, ty, clone_span)?
                } else if aggregate_projection_local {
                    lowerer.projected_value(
                        *initializer,
                        ty,
                        Some(ProjectedAggregateMoveContext::DirectLocal),
                    )?
                } else {
                    lowerer.value(*initializer, ty)?
                };
                if lowerer.places.len() >= ir::MAX_PLACES_PER_FUNCTION {
                    lowerer.errors.at(
                        "ZRYNA-M3201",
                        span(input.sources(), statement.span),
                        "derived aggregate places exceed the per-function M3 limit",
                        "reduce private aggregate locals",
                    );
                    return None;
                }
                let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
                lowerer.places.push(raw::Place {
                    id: place,
                    ty: ty.ir,
                    span: statement_span,
                    kind: raw::PlaceKind::Local(lowerer.next_local),
                });
                lowerer.next_local += 1;
                if !lowerer.emit_effect(
                    statement_span,
                    raw::InstructionKind::InitializePlace { place, value },
                ) {
                    return None;
                }
                if !ty.is_copy() && lowerer.owners.rename(value, place).is_none() {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        statement_span,
                        "owned aggregate local initializer has no available owner",
                        "initialize from one exact available owned value",
                    );
                    return None;
                }
                lowerer
                    .bindings
                    .insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
            }
            RawStatementKind::Return { value, .. } => {
                let return_span = span(input.sources(), statement.span);
                let aggregate_projection_return =
                    matches!(result.category, TypeCategory::Struct | TypeCategory::FixedArray)
                        && lowerer.expression(*value).is_some_and(|expression| {
                            matches!(
                                expression.kind,
                                RawExpressionKind::FieldAccess { .. }
                                    | RawExpressionKind::Index { .. }
                            )
                        });
                let value = if let Some(source) =
                    lowerer.partial_return_transfer_source(*value, result)
                {
                    lowerer.lower_partial_return_transfer(source, result, return_span)?
                } else if aggregate_projection_return {
                    if !function.parameters.is_empty()
                        || Some(*statement_id) != final_statement
                        || return_count != 1
                    {
                        lowerer.errors.at(
                                "ZRYNA-M3016",
                                return_span,
                                "direct aggregate-subobject return requires the sole final return of one parameter-free private function",
                                "return one complete static Struct or fixed-array subobject from a local root as the sole final return in a parameter-free function",
                            );
                        return None;
                    }
                    lowerer.projected_value(
                        *value,
                        result,
                        Some(ProjectedAggregateMoveContext::FinalReturn),
                    )?
                } else {
                    lowerer.value(*value, result)?
                };
                returned = Some((value, return_span));
            }
            RawStatementKind::Assignment { target, value, .. } => {
                let target_expression = lowerer.expression(*target)?.clone();
                if !matches!(
                    target_expression.kind,
                    RawExpressionKind::Reference { .. }
                        | RawExpressionKind::FieldAccess { .. }
                        | RawExpressionKind::Index { .. }
                ) {
                    lowerer.errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), target_expression.span),
                        "owned aggregate assignment target is not an addressable static place",
                        "assign to one mutable root or static Struct/FixedArray String projection",
                    );
                    return None;
                }
                if !matches!(target_expression.kind, RawExpressionKind::Reference { .. }) {
                    let target_ty = lowerer.projection_expression_type(*target);
                    if target_ty.is_some_and(|ty| {
                        matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
                    }) {
                        lowerer.lower_projected_aggregate_assignment(
                            *target,
                            *value,
                            span(input.sources(), statement.span),
                        )?;
                        continue;
                    }
                    let target_place = lowerer.owned_place(*target)?;
                    let target_span = span(input.sources(), target_expression.span);
                    if target_place.is_root || target_place.ty.category != TypeCategory::String {
                        lowerer.errors.at(
                            "ZRYNA-M3013",
                            target_span,
                            "owned projected assignment requires one exact String leaf",
                            "assign only to a static String field or constant String fixed-array element",
                        );
                        return None;
                    }
                    if !target_place.mutable
                        || !lowerer.projection_available(target_place.place, target_place.root)
                    {
                        lowerer.errors.at(
                            "ZRYNA-M3014",
                            target_span,
                            "owned projected assignment target is immutable, moved, or overlaps a moved subobject",
                            "assign only to an initialized mutable available String projection",
                        );
                        return None;
                    }
                    if let Some(reference_span) =
                        lowerer.target_consumption_span(*value, target_place.root, true)
                    {
                        lowerer.errors.at(
                            "ZRYNA-M3014",
                            span(input.sources(), reference_span),
                            "owned projected assignment cannot consume its enclosing root while preparing the replacement",
                            "prepare a distinct String value before replacing the projection",
                        );
                        return None;
                    }
                    let assignment_span = span(input.sources(), statement.span);
                    if !lowerer.reserve_transition(assignment_span) {
                        return None;
                    }
                    let Some(prepared) = lowerer.value(*value, target_place.ty) else {
                        lowerer.release_transition();
                        return None;
                    };
                    lowerer.release_transition();
                    if !lowerer.emit_effect(
                        assignment_span,
                        raw::InstructionKind::ReplacePlace {
                            place: target_place.place,
                            value: prepared,
                        },
                    ) {
                        return None;
                    }
                    if lowerer.owners.transfer(prepared).is_none() {
                        lowerer.errors.at(
                            "ZRYNA-M3014",
                            assignment_span,
                            "owned projected assignment replacement has no distinct prepared owner",
                            "replace from one available independently prepared String value",
                        );
                        return None;
                    }
                    continue;
                }
                let RawExpressionKind::Reference { name } = target_expression.kind else {
                    lowerer.errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), target_expression.span),
                        "owned aggregate assignment requires one root local target",
                        "assign only to an initialized mutable Struct, Enum, or fixed-array local",
                    );
                    return None;
                };
                let Some(binding) = lowerer.bindings.get(&name.text).cloned() else {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!("aggregate assignment target '{}' is not declared", name.text),
                        "assign one exact preceding local",
                    );
                    return None;
                };
                if binding.ty.is_copy()
                    || !matches!(
                        binding.ty.category,
                        TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
                    )
                    || !lowerer.supported(binding.ty)
                {
                    lowerer.errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), name.span),
                        "assignment target is outside the exact supported owned aggregate graph",
                        "assign only to a supported String-bearing Struct, Enum, or fixed-array root",
                    );
                    return None;
                }
                if !binding.mutable || !lowerer.whole_root_available(binding.place) {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), name.span),
                        "owned aggregate assignment target is immutable, moved, or only partially available",
                        "assign only to an initialized mutable aggregate root before moving any projection",
                    );
                    return None;
                }
                if let Some(reference_span) =
                    lowerer.target_consumption_span(*value, binding.place, true)
                {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), reference_span),
                        "owned aggregate assignment cannot consume its destination while preparing its replacement",
                        "clone the destination or prepare a distinct aggregate value before replacement",
                    );
                    return None;
                }
                let assignment_span = span(input.sources(), statement.span);
                if let Some(source) =
                    lowerer.partial_assignment_transfer_source(*value, binding.ty, binding.place)
                {
                    lowerer.lower_partial_assignment_transfer(
                        source,
                        binding.place,
                        binding.ty,
                        assignment_span,
                    )?;
                    continue;
                }
                if !lowerer.reserve_transition(assignment_span) {
                    return None;
                }
                let Some(prepared) = lowerer.value(*value, binding.ty) else {
                    lowerer.release_transition();
                    return None;
                };
                lowerer.release_transition();
                if !lowerer.emit_effect(
                    assignment_span,
                    raw::InstructionKind::ReplacePlace { place: binding.place, value: prepared },
                ) {
                    return None;
                }
                if lowerer.owners.replace(prepared, binding.place).is_none() {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        assignment_span,
                        "owned aggregate assignment replacement has no distinct prepared owner",
                        "replace from one available independently prepared aggregate value",
                    );
                    return None;
                }
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3016",
                    span(input.sources(), statement.span),
                    "statement is outside straight-line private owned aggregate lowering",
                    "use explicitly typed initialized locals and one final return",
                );
                return None;
            }
        }
    }
    let (returned, return_span) = returned?;
    let return_owner = lowerer.owners.owner(returned);
    let cleanup = lowerer.push_cleanup(return_span, return_owner)?;
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters: Vec::new(),
        result: result.ir,
        places: lowerer.places,
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: Vec::new(),
            instructions: lowerer.instructions,
            terminators: vec![raw::SpannedTerminator {
                span: return_span,
                kind: raw::Terminator::Return { value: returned, cleanup },
            }],
        }],
        cleanup_plans: lowerer.cleanup_plans,
    })
}

struct PrivateVecLowerer<'a, 'f, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'f syntax::RawFunctionSyntax,
    module: usize,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    catalog: &'a FunctionCatalog,
    vec_ty: Ty,
    element: Ty,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    places: Vec<raw::Place>,
    reserved_places: usize,
    cfg: OwnedCfgState,
    cleanup_plans: Vec<raw::CleanupPlan>,
    cleanup_actions: usize,
    reserved_cleanup_plans: usize,
    reserved_cleanup_actions: usize,
    owners: OwnerState,
    known_string_bytes: BTreeMap<raw::PlaceId, u64>,
    next_value: u32,
    next_local: u32,
}

impl PrivateVecLowerer<'_, '_, '_> {
    fn push_scope_error(
        incoming: Option<&OwnedVecBranchState>,
        place: raw::PlaceId,
        allow_incoming_target: bool,
    ) -> Option<(&'static str, &'static str)> {
        let incoming = incoming?;
        let is_incoming_target = incoming.bindings.values().any(|outer| outer.place == place);
        if allow_incoming_target && !is_incoming_target {
            return Some((
                "owned Vec loop must mutate its one incoming Vec root",
                "push only into the mutable Vec declared before this loop",
            ));
        }
        if !allow_incoming_target && is_incoming_target {
            return Some((
                "owned Vec branch cannot mutate an incoming Vec",
                "push only into a Vec declared inside this branch",
            ));
        }
        None
    }

    fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

    fn preflight_string_expression(&mut self, id: u32, string_ty: Ty, at: Span) -> bool {
        let estimate = match estimate_owned_string_expression(
            self.function,
            &self.bindings,
            &self.owners,
            string_ty,
            id,
            self.owners.pending().len(),
            OwnedStringEstimateContext::Value,
        ) {
            Ok(estimate) => estimate,
            Err(OwnedStringEstimateError::Unsupported) => return true,
            Err(OwnedStringEstimateError::Unavailable(reference)) => {
                self.errors.at(
                    "ZRYNA-M3014",
                    span(self.input.sources(), reference),
                    "Vec String element has no available owner",
                    "move each String element at most once",
                );
                return false;
            }
            Err(OwnedStringEstimateError::Overflow) => {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "recursive Vec String-element preparation overflows its checked resource estimate",
                    "reduce nested String-producing element expressions",
                );
                return false;
            }
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans: self.reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions: self.reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
    }

    fn estimate_string_sequence(
        &mut self,
        expressions: &[u32],
        string_ty: Ty,
        at: Span,
    ) -> Option<OwnedStringPreparationEstimate> {
        let mut total = empty_owned_string_estimate(self.owners.pending().len());
        for expression in expressions {
            let child = match estimate_owned_string_expression(
                self.function,
                &self.bindings,
                &self.owners,
                string_ty,
                *expression,
                total.end_pending,
                OwnedStringEstimateContext::Value,
            ) {
                Ok(estimate) => estimate,
                Err(OwnedStringEstimateError::Unsupported) => {
                    let expression = self.expression(*expression)?;
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), expression.span),
                        "Vec<String> element expression is outside checked String preparation",
                        "use a String literal, available String move, clone, concat, or private String call",
                    );
                    return None;
                }
                Err(OwnedStringEstimateError::Unavailable(reference)) => {
                    self.errors.at(
                        "ZRYNA-M3014",
                        span(self.input.sources(), reference),
                        "Vec String element has no available owner",
                        "move each String element at most once",
                    );
                    return None;
                }
                Err(OwnedStringEstimateError::Overflow) => {
                    self.errors.at(
                        "ZRYNA-M3201",
                        at,
                        "recursive Vec String-element preparation overflows its checked resource estimate",
                        "reduce nested String-producing element expressions",
                    );
                    return None;
                }
            };
            total = match add_estimate_counts(total, child) {
                Ok(total) => total,
                Err(OwnedStringEstimateError::Overflow) => {
                    self.errors.at(
                        "ZRYNA-M3201",
                        at,
                        "recursive Vec String-element sequence overflows its checked resource estimate",
                        "reduce nested String-producing element expressions",
                    );
                    return None;
                }
                Err(
                    OwnedStringEstimateError::Unsupported
                    | OwnedStringEstimateError::Unavailable(_),
                ) => {
                    unreachable!("combining checked estimates cannot change expression support")
                }
            };
        }
        Some(total)
    }

    fn preflight_string_sequence_with_enclosing_cleanup(
        &mut self,
        estimate: OwnedStringPreparationEstimate,
        enclosing_actions: usize,
        at: Span,
    ) -> bool {
        let Some(reserved_cleanup_plans) = self.reserved_cleanup_plans.checked_add(1) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "enclosing Vec cleanup reservation overflows its checked resource estimate",
                "reduce nested Vec and String-producing expressions",
            );
            return false;
        };
        let Some(reserved_cleanup_actions) =
            self.reserved_cleanup_actions.checked_add(enclosing_actions)
        else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "enclosing Vec cleanup action reservation overflows its checked resource estimate",
                "reduce simultaneously live Vec and String owners",
            );
            return false;
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
    }

    fn estimate_vec_preparation(
        &mut self,
        id: u32,
        expected: Ty,
        pending: usize,
        at: Span,
    ) -> Option<VecPreparationEstimate> {
        let expression = self.expression(id)?.clone();
        match expression.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).filter(|binding| {
                    binding.ty == expected && self.owners.contains(binding.place)
                })?;
                Some(VecPreparationEstimate {
                    end_pending: pending,
                    resources: OwnedStringPreparationEstimate {
                        values: 1,
                        places: 1,
                        transitions: 1,
                        transfers_existing: true,
                        ..empty_owned_string_estimate(pending)
                    },
                })
            }
            RawExpressionKind::Clone { value, .. } => {
                let operand = self.expression(value)?;
                let RawExpressionKind::Reference { name } = &operand.kind else {
                    return None;
                };
                self.bindings.get(&name.text).filter(|binding| {
                    binding.ty == expected && self.owners.contains(binding.place)
                })?;
                let end_pending = pending.checked_add(1)?;
                let clones_non_copy_elements = self.element.category == TypeCategory::String;
                Some(VecPreparationEstimate {
                    end_pending,
                    resources: OwnedStringPreparationEstimate {
                        end_pending,
                        peak_pending: end_pending,
                        cleanup_plans: 1 + usize::from(clones_non_copy_elements),
                        cleanup_actions: pending.checked_add(if clones_non_copy_elements {
                            pending.checked_add(1)?
                        } else {
                            0
                        })?,
                        values: 1,
                        places: 1,
                        transitions: 1,
                        transfers_existing: false,
                        root_cleanup_actions: Some(pending),
                    },
                })
            }
            RawExpressionKind::VecConstruction { elements, .. } => {
                let mut resources = if self.element.category == TypeCategory::String {
                    self.estimate_string_sequence(&elements, self.element, at)?
                } else {
                    empty_owned_string_estimate(pending)
                };
                let prepared_pending = resources.end_pending;
                let consumed = usize::from(!self.element.is_copy()) * elements.len();
                let end_pending = prepared_pending.checked_sub(consumed)?.checked_add(1)?;
                resources.end_pending = end_pending;
                resources.peak_pending = resources.peak_pending.max(end_pending);
                resources.cleanup_plans = resources.cleanup_plans.checked_add(1)?;
                resources.cleanup_actions =
                    resources.cleanup_actions.checked_add(prepared_pending)?;
                resources.values = resources.values.checked_add(1)?;
                resources.places = resources.places.checked_add(1)?;
                resources.transitions = resources.transitions.checked_add(1)?;
                resources.root_cleanup_actions = Some(prepared_pending);
                Some(VecPreparationEstimate { end_pending, resources })
            }
            RawExpressionKind::Call { arguments, .. } if arguments.is_empty() => {
                let end_pending = pending.checked_add(1)?;
                Some(VecPreparationEstimate {
                    end_pending,
                    resources: OwnedStringPreparationEstimate {
                        end_pending,
                        peak_pending: end_pending,
                        cleanup_plans: 1,
                        cleanup_actions: pending,
                        values: 1,
                        places: 1,
                        transitions: 1,
                        transfers_existing: false,
                        root_cleanup_actions: Some(pending),
                    },
                })
            }
            RawExpressionKind::Call { arguments, .. } if arguments.len() == 1 => {
                let mut preparation =
                    self.estimate_vec_preparation(arguments[0], expected, pending, at)?;
                let cleanup = preparation.end_pending.checked_sub(1)?;
                preparation.resources.cleanup_plans =
                    preparation.resources.cleanup_plans.checked_add(1)?;
                preparation.resources.cleanup_actions =
                    preparation.resources.cleanup_actions.checked_add(cleanup)?;
                preparation.resources.values = preparation.resources.values.checked_add(1)?;
                preparation.resources.places = preparation.resources.places.checked_add(1)?;
                preparation.resources.transitions =
                    preparation.resources.transitions.checked_add(1)?;
                preparation.resources.root_cleanup_actions = Some(cleanup);
                Some(preparation)
            }
            _ => None,
        }
    }

    fn preflight_push_cleanup(&mut self, value: u32, at: Span) -> Option<usize> {
        let moves_existing_owner = !self.element.is_copy()
            && self.expression(value).is_some_and(|expression| {
                matches!(&expression.kind, RawExpressionKind::Reference { name }
                if self.bindings.get(&name.text).is_some_and(|binding| {
                    binding.ty == self.element && self.owners.contains(binding.place)
                }))
            });
        let nested_estimate = if self.element.category == TypeCategory::String {
            Some(self.estimate_string_sequence(&[value], self.element, at)?)
        } else {
            None
        };
        let reserved_actions = nested_estimate.map_or_else(
            || {
                cleanup_actions_after_preparation(
                    self.owners.pending().len(),
                    !self.element.is_copy() && !moves_existing_owner,
                )
            },
            |estimate| estimate.end_pending,
        );
        if let Some(estimate) = nested_estimate
            && !self.preflight_string_sequence_with_enclosing_cleanup(
                estimate,
                reserved_actions,
                at,
            )
        {
            return None;
        }
        Some(reserved_actions)
    }

    fn preflight_construct_cleanup(&mut self, elements: &[u32], at: Span) -> Option<usize> {
        let nested_estimate = if self.element.category == TypeCategory::String {
            Some(self.estimate_string_sequence(elements, self.element, at)?)
        } else {
            None
        };
        let additional_owners = elements
            .iter()
            .filter(|element| {
                !self.element.is_copy()
                    && !self.expression(**element).is_some_and(|expression| {
                        matches!(&expression.kind, RawExpressionKind::Reference { name }
                        if self.bindings.get(&name.text).is_some_and(|binding| {
                            binding.ty == self.element && self.owners.contains(binding.place)
                        }))
                    })
            })
            .count();
        let reserved_actions = nested_estimate.map_or_else(
            || cleanup_actions_after_additions(self.owners.pending().len(), additional_owners),
            |estimate| estimate.end_pending,
        );
        if let Some(estimate) = nested_estimate
            && !self.preflight_string_sequence_with_enclosing_cleanup(
                estimate,
                reserved_actions,
                at,
            )
        {
            return None;
        }
        Some(reserved_actions)
    }

    fn preflight_place(&mut self, at: Span) -> bool {
        preflight_owned_place_capacity_with_reserved(
            self.places.len(),
            self.reserved_places,
            1,
            at,
            self.errors,
        )
    }

    fn reserve_local_place(&mut self, at: Span) -> bool {
        if !self.preflight_place(at) {
            return false;
        }
        self.reserved_places += 1;
        true
    }

    fn release_local_place(&mut self) {
        self.reserved_places = self.reserved_places.checked_sub(1).expect("reserved local place");
    }

    fn reserve_local_commit(&mut self, at: Span) -> bool {
        if !self.reserve_local_place(at) {
            return false;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            return false;
        }
        true
    }

    fn release_local_commit(&mut self) {
        self.cfg.release_transitions(1);
        self.release_local_place();
    }

    fn reserve_cleanup_capacity(&mut self, actions: usize, at: Span) -> bool {
        if resource_budget_violation(
            self.cleanup_plans.len(),
            self.reserved_cleanup_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            self.cleanup_actions,
            self.reserved_cleanup_actions.saturating_add(actions),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "reserved Vec cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live owned values or fallible Vec operations",
            );
            return false;
        }
        self.reserved_cleanup_plans += 1;
        self.reserved_cleanup_actions += actions;
        true
    }

    fn release_cleanup_capacity(&mut self, actions: usize) {
        self.reserved_cleanup_plans =
            self.reserved_cleanup_plans.checked_sub(1).expect("reserved cleanup plan");
        self.reserved_cleanup_actions =
            self.reserved_cleanup_actions.checked_sub(actions).expect("reserved cleanup actions");
    }

    #[allow(clippy::too_many_lines)]
    fn clone_vec(&mut self, operand: u32, expected: Ty, at: Span) -> Option<raw::ValueId> {
        if !matches!(
            self.element.category,
            TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String
        ) {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "Vec clone is sealed to exact Vec<bool>, Vec<i32>, and Vec<String>",
                "use clone only with one admitted exact private Vec element type",
            );
            return None;
        }
        let operand = self.expression(operand)?.clone();
        let RawExpressionKind::Reference { name } = operand.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), operand.span),
                "Vec clone requires an addressable local root",
                "clone one available Vec local by name",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("Vec binding '{}' is not declared in this function", name.text),
                "clone one preceding available Vec local",
            );
            return None;
        };
        if binding.ty != expected {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), name.span),
                "Vec clone source has the wrong exact container type",
                "clone a local with the exact contextual Vec element type",
            );
            return None;
        }
        if !self.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("Vec value '{}' was already moved", name.text),
                "clone the Vec only while its owner remains available",
            );
            return None;
        }
        let actions = self.owners.pending().len();
        let clones_non_copy_elements = self.element.category == TypeCategory::String;
        let prefix_actions = if clones_non_copy_elements {
            Some(actions.checked_add(1).or_else(|| {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "Vec clone prefix cleanup overflows its checked action count",
                    "reduce simultaneously live owned values",
                );
                None
            })?)
        } else {
            None
        };
        self.cfg.reserve_values(1, at, self.errors)?;
        if !self.reserve_local_place(at) {
            self.cfg.release_values(1);
            return None;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if !self.reserve_cleanup_capacity(actions, at) {
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if prefix_actions
            .is_some_and(|prefix_actions| !self.reserve_cleanup_capacity(prefix_actions, at))
        {
            self.release_cleanup_capacity(actions);
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if let Some(prefix_actions) = prefix_actions {
            self.release_cleanup_capacity(prefix_actions);
        }
        self.release_cleanup_capacity(actions);
        self.cfg.release_transitions(1);
        self.release_local_place();
        self.cfg.release_values(1);
        let cleanup = self.push_instruction_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let element_cleanup = if clones_non_copy_elements {
            Some(self.push_vec_clone_prefix_cleanup(at, result_owner)?)
        } else {
            None
        };
        Some(
            self.emit(
                expected,
                at,
                raw::InstructionKind::VecClone { place: binding.place, cleanup, element_cleanup },
            )?
            .0,
        )
    }

    fn push_vec_clone_prefix_cleanup(
        &mut self,
        at: Span,
        result_owner: raw::PlaceId,
    ) -> Option<raw::CleanupPlanId> {
        let action_count = self.owners.pending().len().checked_add(1)?;
        if resource_budget_violation(
            self.cleanup_plans.len(),
            self.reserved_cleanup_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            self.cleanup_actions,
            self.reserved_cleanup_actions.saturating_add(action_count),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "Vec clone element cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live owned values or fallible Vec clones",
            );
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).ok()?);
        let actions = std::iter::once(raw::DropAction::DropVecInitializedPrefix(result_owner))
            .chain(self.owners.pending().iter().rev().copied().map(raw::DropAction::DropPlace))
            .collect();
        self.cleanup_plans.push(raw::CleanupPlan { id, span: at, actions });
        self.cleanup_actions += action_count;
        Some(id)
    }

    fn condition(&mut self, id: u32, bool_ty: Ty) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        debug_assert_eq!(bool_ty.category, TypeCategory::Bool);
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } => {
                Some(self.emit(bool_ty, at, raw::InstructionKind::BoolLiteral(value))?.0)
            }
            RawExpressionKind::Reference { name } => {
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("binding '{}' is not declared in this function", name.text),
                        "reference one exact preceding bool binding",
                    );
                    return None;
                };
                if binding.ty != bool_ty {
                    self.errors.at(
                        "ZRYNA-M3012",
                        span(self.input.sources(), name.span),
                        "owned Vec control-flow condition must have exact bool type",
                        "use a bool literal or preceding exact bool binding",
                    );
                    return None;
                }
                Some(
                    self.emit(
                        bool_ty,
                        at,
                        raw::InstructionKind::CopyFromPlace { place: binding.place },
                    )?
                    .0,
                )
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "owned Vec control-flow condition must be a bool literal or reference",
                    "use one exact Copy bool condition",
                );
                None
            }
        }
    }

    fn lower_local(&mut self, statement: &syntax::RawStatementSyntax) -> Option<()> {
        let RawStatementKind::LocalDeclaration { mutable, name, type_syntax, initializer, .. } =
            &statement.kind
        else {
            return None;
        };
        if self.bindings.keys().any(|existing| existing.eq_ignore_ascii_case(&name.text)) {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' collides under portable ASCII case folding", name.text),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        let ty = semantic_type(
            self.file,
            *type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        if !matches!(ty.category, TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String)
            && ty != self.vec_ty
        {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), statement.span),
                "local type is outside this private Vec slice",
                "use the exact Vec type or its bool, i32, or String element type",
            );
            return None;
        }
        let local_span = span(self.input.sources(), statement.span);
        if !self.reserve_local_commit(local_span) {
            return None;
        }
        let Some(value) = self.value(*initializer, ty) else {
            self.release_local_commit();
            return None;
        };
        let place = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let initialize = raw::Instruction {
            result: None,
            span: local_span,
            kind: raw::InstructionKind::InitializePlace { place, value },
        };
        self.release_local_commit();
        self.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: local_span,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local = self.next_local.checked_add(1)?;
        if !self.cfg.emit(initialize, self.errors) {
            return None;
        }
        if !ty.is_copy() && !self.rename_owner(value, place) {
            self.errors.at(
                "ZRYNA-M3014",
                local_span,
                "owned local initializer has no available owner",
                "initialize the local from one available owned value",
            );
            return None;
        }
        self.bindings.insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
        Some(())
    }

    fn lower_push_effect(
        &mut self,
        expression_id: u32,
        incoming: Option<&OwnedVecBranchState>,
    ) -> Option<()> {
        self.lower_push_effect_with_policy(expression_id, incoming, false)
    }

    fn lower_push_effect_with_policy(
        &mut self,
        expression_id: u32,
        incoming: Option<&OwnedVecBranchState>,
        allow_incoming_target: bool,
    ) -> Option<()> {
        let expression = self.expression(expression_id)?.clone();
        let at = span(self.input.sources(), expression.span);
        let RawExpressionKind::VecPush { vector, value, .. } = expression.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "only push(vector, value) is admitted as a Vec effect statement",
                "use push on one mutable initialized Vec local",
            );
            return None;
        };
        let vector_expression = self.expression(vector)?.clone();
        let RawExpressionKind::Reference { name } = vector_expression.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), vector_expression.span),
                "push requires an addressable Vec local",
                "push into one mutable initialized Vec local",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("Vec binding '{}' is not declared in this function", name.text),
                "reference one exact preceding mutable Vec local",
            );
            return None;
        };
        if let Some((message, help)) =
            Self::push_scope_error(incoming, binding.place, allow_incoming_target)
        {
            self.errors.at("ZRYNA-M3015", span(self.input.sources(), name.span), message, help);
            return None;
        }
        if binding.ty != self.vec_ty {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), name.span),
                "push target has the wrong exact Vec type",
                "push into the function's exact Vec type",
            );
            return None;
        }
        if vec_push_target_invalid(binding.mutable, self.owners.contains(binding.place)) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "push target is immutable, uninitialized, or already moved",
                "push into one mutable initialized available Vec local",
            );
            return None;
        }
        let reserved_actions = self.preflight_push_cleanup(value, at)?;
        if !reserve_owned_commit_transition(&mut self.cfg, at, self.errors) {
            return None;
        }
        if !self.reserve_cleanup_capacity(reserved_actions, at) {
            release_owned_commit_transition(&mut self.cfg);
            return None;
        }
        let Some(value) = self.value(value, self.element) else {
            self.release_cleanup_capacity(reserved_actions);
            release_owned_commit_transition(&mut self.cfg);
            return None;
        };
        let consumed = self.owners.owner(value);
        self.release_cleanup_capacity(reserved_actions);
        release_owned_commit_transition(&mut self.cfg);
        let cleanup = self.push_instruction_cleanup(at, None)?;
        if !self.emit_effect(
            at,
            raw::InstructionKind::VecPush { vector: binding.place, value, cleanup },
        ) {
            return None;
        }
        if let Some(owner) = consumed
            && let Some(delta) = self.owners.consume_owner(owner)
        {
            apply_owner_delta(&mut self.known_string_bytes, delta);
        }
        Some(())
    }

    fn lower_loop_push(
        &mut self,
        expression_id: u32,
        incoming: &OwnedVecBranchState,
        at: Span,
    ) -> Option<()> {
        self.lower_push_effect_with_policy(expression_id, Some(incoming), true)?;
        self.bindings = incoming.bindings.clone();
        if self.owners != incoming.owners || self.known_string_bytes != incoming.known_string_bytes
        {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "Vec push loop does not restore the exact header owner state",
                "retain the same outer Vec place and consume only the pushed element",
            );
            return None;
        }
        Some(())
    }

    fn restore_branch_scope(&mut self, incoming: &OwnedVecBranchState, at: Span) -> Option<()> {
        if !self.owners.pending().starts_with(incoming.owners.pending()) {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "owned Vec branch changed an incoming owner",
                "leave every outer owned value unchanged on both branch paths",
            );
            return None;
        }
        let branch_owners = self.owners.pending()[incoming.owners.pending().len()..].to_vec();
        if resource_budget_violation(
            self.cleanup_actions,
            branch_owners.len(),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce branch-local owned values or fallible Vec operations",
            );
            return None;
        }
        if !self.cfg.preflight_transitions(branch_owners.len(), at, self.errors) {
            return None;
        }
        for owner in branch_owners.into_iter().rev() {
            let drop = raw::Instruction {
                result: None,
                span: at,
                kind: raw::InstructionKind::DropPlace { place: owner },
            };
            if !self.cfg.preflight_emit(&drop, self.errors) || !self.cfg.emit(drop, self.errors) {
                return None;
            }
            self.cleanup_actions = self.cleanup_actions.checked_add(1)?;
            let delta = self.owners.consume_owner(owner)?;
            apply_owner_delta(&mut self.known_string_bytes, delta);
        }
        self.bindings = incoming.bindings.clone();
        if self.owners != incoming.owners || self.known_string_bytes != incoming.known_string_bytes
        {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "owned Vec branch does not restore the incoming ownership state",
                "drop branch locals and leave every outer owned value unchanged",
            );
            return None;
        }
        Some(())
    }

    fn drop_non_carried(&mut self, carried: raw::PlaceId, at: Span) -> Option<()> {
        let dropped = self
            .owners
            .pending()
            .iter()
            .copied()
            .filter(|owner| *owner != carried)
            .collect::<Vec<_>>();
        if resource_budget_violation(
            self.cleanup_actions,
            dropped.len(),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "terminal Vec arm cleanup exceeds the per-function M3 limit",
                "reduce owned temporaries in the returning branch expression",
            );
            return None;
        }
        if !self.cfg.preflight_transitions(dropped.len(), at, self.errors) {
            return None;
        }
        for owner in dropped.into_iter().rev() {
            if !self.emit_effect(at, raw::InstructionKind::DropPlace { place: owner }) {
                return None;
            }
            self.cleanup_actions = self.cleanup_actions.checked_add(1)?;
            let delta = self.owners.consume_owner(owner)?;
            apply_owner_delta(&mut self.known_string_bytes, delta);
        }
        Some(())
    }

    fn lower_branch(
        &mut self,
        block_id: Option<u32>,
        incoming: &OwnedVecBranchState,
        at: Span,
    ) -> Option<()> {
        let mut scope_span = at;
        if let Some(block_id) = block_id {
            let block = usize::try_from(block_id)
                .ok()
                .and_then(|index| self.function.body.blocks.get(index))?
                .clone();
            scope_span = span(self.input.sources(), block.span);
            for statement_id in block.statements {
                let statement = usize::try_from(statement_id)
                    .ok()
                    .and_then(|index| self.function.body.statements.get(index))?
                    .clone();
                match statement.kind {
                    RawStatementKind::LocalDeclaration { initializer, .. } => {
                        if let Some(reference_span) = self.incoming_move_span(initializer, incoming)
                        {
                            self.errors.at(
                                "ZRYNA-M3015",
                                span(self.input.sources(), reference_span),
                                "owned Vec loop or branch cannot move an incoming owner",
                                "clone the incoming value or construct the local independently",
                            );
                            return None;
                        }
                        self.lower_local(&statement)?;
                    }
                    RawStatementKind::ExpressionStatement { expression, .. } => {
                        self.lower_push_effect(expression, Some(incoming))?;
                    }
                    _ => {
                        self.errors.at(
                            "ZRYNA-M3016",
                            span(self.input.sources(), statement.span),
                            "this branch statement is outside the bounded owned Vec if slice",
                            "use branch-local typed declarations and push only into branch-local Vec values",
                        );
                        return None;
                    }
                }
            }
        }
        self.restore_branch_scope(incoming, scope_span)
    }

    fn incoming_move_span(&self, id: u32, incoming: &OwnedVecBranchState) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if incoming.bindings.get(&name.text).is_some_and(|binding| {
                    !binding.ty.is_copy() && incoming.owners.contains(binding.place)
                }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text != "concat" => {
                arguments.iter().find_map(|argument| self.incoming_move_span(*argument, incoming))
            }
            RawExpressionKind::VecConstruction { elements, .. } => {
                elements.iter().find_map(|element| self.incoming_move_span(*element, incoming))
            }
            _ => None,
        }
    }

    fn target_consumption_span(
        &self,
        id: u32,
        target: raw::PlaceId,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && self.bindings.get(&name.text).is_some_and(|binding| {
                        binding.place == target && self.owners.contains(binding.place)
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.target_consumption_span(*value, target, false)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                arguments
                    .iter()
                    .find_map(|argument| self.target_consumption_span(*argument, target, false))
            }
            RawExpressionKind::Call { arguments, .. } => arguments
                .iter()
                .find_map(|argument| self.target_consumption_span(*argument, target, true)),
            RawExpressionKind::VecConstruction { elements, .. } => elements
                .iter()
                .find_map(|element| self.target_consumption_span(*element, target, true)),
            _ => None,
        }
    }

    fn push_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        if resource_budget_violation(
            self.cleanup_plans.len(),
            self.reserved_cleanup_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup sites exceed the per-function M3 limit of {}",
                    ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                ),
                "reduce fallible private Vec operations",
            );
            return None;
        }
        let pending = self.owners.pending();
        let excluded_present = excluded.is_some_and(|place| self.owners.contains(place));
        let action_count = pending.len() - usize::from(excluded_present);
        if resource_budget_violation(
            self.cleanup_actions,
            self.reserved_cleanup_actions.saturating_add(action_count),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce simultaneously live owned values or fallible private Vec operations",
            );
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).unwrap_or(u32::MAX));
        self.cleanup_plans.push(raw::CleanupPlan {
            id,
            span: at,
            actions: self
                .owners
                .pending()
                .iter()
                .rev()
                .copied()
                .filter(|place| Some(*place) != excluded)
                .map(raw::DropAction::DropPlace)
                .collect(),
        });
        self.cleanup_actions += action_count;
        Some(id)
    }

    fn push_instruction_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        if !self.cfg.preflight_transition(at, self.errors) {
            return None;
        }
        self.push_cleanup(at, excluded)
    }

    fn emit(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<(raw::ValueId, Option<raw::PlaceId>)> {
        if !ty.is_copy() && !self.preflight_place(at) {
            return None;
        }
        let value = raw::ValueId(self.next_value);
        let instruction = raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind,
        };
        if !self.cfg.preflight_emit(&instruction, self.errors) {
            return None;
        }
        self.next_value = self.next_value.checked_add(1)?;
        if !self.cfg.emit(instruction, self.errors) {
            return None;
        }
        if ty.is_copy() {
            return Some((value, None));
        }
        let owner = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
        self.places.push(raw::Place {
            id: owner,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        let _ = self.owners.register(value, owner);
        Some((value, Some(owner)))
    }

    fn emit_effect(&mut self, at: Span, kind: raw::InstructionKind) -> bool {
        self.cfg.emit(raw::Instruction { result: None, span: at, kind }, self.errors)
    }

    fn rename_owner(&mut self, value: raw::ValueId, target: raw::PlaceId) -> bool {
        let Some(delta) = self.owners.rename(value, target) else { return false };
        apply_owner_delta(&mut self.known_string_bytes, delta);
        true
    }

    fn replace_owner(&mut self, value: raw::ValueId, target: raw::PlaceId) -> bool {
        let Some(delta) = self.owners.replace(value, target) else { return false };
        apply_owner_delta(&mut self.known_string_bytes, delta);
        true
    }

    fn transfer_owner(&mut self, value: raw::ValueId) -> bool {
        let Some(delta) = self.owners.transfer(value) else { return false };
        apply_owner_delta(&mut self.known_string_bytes, delta);
        true
    }

    fn string_place_for_read(&mut self, id: u32) -> Option<(raw::PlaceId, u64)> {
        let expression = self.expression(id)?.clone();
        if let RawExpressionKind::Reference { name } = expression.kind {
            let Some(binding) = self.bindings.get(&name.text).cloned() else {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), name.span),
                    format!("String operand '{}' is not declared", name.text),
                    "reference one exact preceding String local",
                );
                return None;
            };
            if binding.ty.category != TypeCategory::String {
                self.errors.at(
                    "ZRYNA-M3012",
                    span(self.input.sources(), name.span),
                    "String operand has the wrong exact type",
                    "use one exact String value",
                );
                return None;
            }
            if !self.owners.contains(binding.place) {
                self.errors.at(
                    "ZRYNA-M3014",
                    span(self.input.sources(), name.span),
                    format!("String value '{}' was already moved", name.text),
                    "use each owned String only while it remains available",
                );
                return None;
            }
            Some((binding.place, *self.known_string_bytes.get(&binding.place)?))
        } else {
            let string = self
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == TypeCategory::String)
                .copied()?;
            let value = self.value(id, string)?;
            let owner = self.owners.owner(value)?;
            Some((owner, *self.known_string_bytes.get(&owner)?))
        }
    }

    #[allow(clippy::too_many_lines)]
    fn value(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        if expected.category == TypeCategory::String
            && !self.preflight_string_expression(id, expected, at)
        {
            return None;
        }
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } if expected.category == TypeCategory::Bool => {
                Some(self.emit(expected, at, raw::InstructionKind::BoolLiteral(value))?.0)
            }
            RawExpressionKind::I32Literal { spelling }
                if expected.category == TypeCategory::I32 =>
            {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3013",
                        at,
                        "Vec element integer is outside i32",
                        "use an in-range i32 element",
                    );
                    None
                })?;
                Some(self.emit(expected, at, raw::InstructionKind::I32Literal(value))?.0)
            }
            RawExpressionKind::StringLiteral { spelling }
                if expected.category == TypeCategory::String =>
            {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes().to_vec();
                let byte_count = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let (value, owner) = self.emit(
                    expected,
                    at,
                    raw::InstructionKind::StringFromUtf8 { bytes, cleanup },
                )?;
                self.known_string_bytes.insert(owner?, byte_count);
                Some(value)
            }
            RawExpressionKind::Reference { name } => {
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), name.span),
                        format!("Vec operand '{}' is not declared", name.text),
                        "reference one exact preceding typed local",
                    );
                    return None;
                };
                if binding.ty != expected {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), name.span),
                        "Vec operand has the wrong exact element or container type",
                        "use the exact declared Vec element type",
                    );
                    return None;
                }
                if expected.is_copy() {
                    Some(
                        self.emit(
                            expected,
                            at,
                            raw::InstructionKind::CopyFromPlace { place: binding.place },
                        )?
                        .0,
                    )
                } else {
                    if !self.owners.contains(binding.place) {
                        self.errors.at(
                            "ZRYNA-M3014",
                            span(self.input.sources(), name.span),
                            format!("owned value '{}' was already moved", name.text),
                            "move each owned value at most once",
                        );
                        return None;
                    }
                    let (value, owner) = self.emit(
                        expected,
                        at,
                        raw::InstructionKind::MoveFromPlace { place: binding.place },
                    )?;
                    let owner = owner?;
                    let delta = self.owners.rehome_move_result(value, binding.place)?;
                    debug_assert_eq!(delta, OwnerDelta::Renamed { from: binding.place, to: owner });
                    apply_owner_delta(&mut self.known_string_bytes, delta);
                    Some(value)
                }
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::String => {
                let (source, bytes) = self.string_place_for_read(value)?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let (value, owner) = self.emit(
                    expected,
                    at,
                    raw::InstructionKind::StringClone { place: source, cleanup },
                )?;
                self.known_string_bytes.insert(owner?, bytes);
                Some(value)
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::Vec => {
                self.clone_vec(value, expected, at)
            }
            RawExpressionKind::Call { callee, arguments, .. }
                if expected.category == TypeCategory::String && callee.text == "concat" =>
            {
                let [left, right] = arguments.as_slice() else {
                    self.errors.at(
                        "ZRYNA-M3012",
                        span(self.input.sources(), callee.span),
                        "String concat requires exactly two operands",
                        "call concat(left, right) with two available String values",
                    );
                    return None;
                };
                let (left, left_bytes) = self.string_place_for_read(*left)?;
                let (right, right_bytes) = self.string_place_for_read(*right)?;
                let Some(bytes) = checked_string_concat_bytes(left_bytes, right_bytes) else {
                    self.errors.at(
                        "ZRYNA-M3012",
                        at,
                        "String concatenation exceeds the sealed runtime byte limit",
                        "reduce the statically known concatenated String size",
                    );
                    return None;
                };
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let (value, owner) = self.emit(
                    expected,
                    at,
                    raw::InstructionKind::StringConcat { left, right, cleanup },
                )?;
                self.known_string_bytes.insert(owner?, bytes);
                Some(value)
            }
            RawExpressionKind::Call { callee, arguments, .. }
                if expected.category == TypeCategory::Vec =>
            {
                self.direct_call(&callee, &arguments, expected, at)
            }
            RawExpressionKind::VecConstruction { type_syntax, elements, .. }
                if expected.category == TypeCategory::Vec =>
            {
                let constructed = semantic_type(
                    self.file,
                    type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?;
                if constructed != expected {
                    self.errors.at(
                        "ZRYNA-M3013",
                        at,
                        "Vec construction type differs from its contextual type",
                        "construct the exact annotated Vec type",
                    );
                    return None;
                }
                let reserved_actions = self.preflight_construct_cleanup(&elements, at)?;
                self.cfg.reserve_values(1, at, self.errors)?;
                if !self.reserve_local_place(at) {
                    self.cfg.release_values(1);
                    return None;
                }
                if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
                    self.release_local_place();
                    self.cfg.release_values(1);
                    return None;
                }
                if !self.reserve_cleanup_capacity(reserved_actions, at) {
                    self.cfg.release_transitions(1);
                    self.release_local_place();
                    self.cfg.release_values(1);
                    return None;
                }
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    let Some(value) = self.value(element, self.element) else {
                        self.release_cleanup_capacity(reserved_actions);
                        self.cfg.release_transitions(1);
                        self.release_local_place();
                        self.cfg.release_values(1);
                        return None;
                    };
                    values.push(value);
                }
                let consumed =
                    values.iter().filter_map(|value| self.owners.owner(*value)).collect::<Vec<_>>();
                self.release_cleanup_capacity(reserved_actions);
                self.cfg.release_transitions(1);
                self.release_local_place();
                self.cfg.release_values(1);
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let result = self
                    .emit(
                        expected,
                        at,
                        raw::InstructionKind::VecConstruct { elements: values, cleanup },
                    )?
                    .0;
                for owner in consumed {
                    if let Some(delta) = self.owners.consume_owner(owner) {
                        apply_owner_delta(&mut self.known_string_bytes, delta);
                    }
                }
                Some(result)
            }
            RawExpressionKind::Index { base, index, .. } => {
                if expected != self.element || !expected.is_copy() {
                    self.errors.at(
                        "ZRYNA-M3013",
                        at,
                        "Vec indexing is admitted only for the exact Copy element type",
                        "index Vec<bool> or Vec<i32> and return that exact scalar type",
                    );
                    return None;
                }
                let base_expression = self.expression(base)?.clone();
                let RawExpressionKind::Reference { name } = base_expression.kind else {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), base_expression.span),
                        "Vec indexing requires an addressable local Vec",
                        "index one initialized Vec local",
                    );
                    return None;
                };
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("Vec binding '{}' is not declared in this function", name.text),
                        "reference one exact preceding Vec local",
                    );
                    return None;
                };
                if binding.ty.category != TypeCategory::Vec || !self.owners.contains(binding.place)
                {
                    self.errors.at(
                        "ZRYNA-M3014",
                        span(self.input.sources(), name.span),
                        "indexed Vec is unavailable or already moved",
                        "index one initialized available Vec local",
                    );
                    return None;
                }
                let i32_ty = self
                    .node_types
                    .iter()
                    .flatten()
                    .find(|ty| ty.category == TypeCategory::I32)
                    .copied()?;
                let index = self.value(index, i32_ty)?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                Some(
                    self.emit(
                        expected,
                        at,
                        raw::InstructionKind::VecIndexCopy { place: binding.place, index, cleanup },
                    )?
                    .0,
                )
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3013",
                    at,
                    "expression is outside private straight-line Vec lowering",
                    "use exact bool, i32, or String elements and private Vec moves",
                );
                None
            }
        }
    }

    fn resolve_owned_callee(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        expected: Ty,
    ) -> Option<FunctionSignature> {
        let signature = match self.catalog.resolve(self.module, &callee.text) {
            FunctionResolution::Exact(signature) => signature.clone(),
            FunctionResolution::WrongCase => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("call name '{}' has the wrong portable ASCII case", callee.text),
                    "use the callee's exact declared spelling",
                );
                return None;
            }
            FunctionResolution::Missing => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("function '{}' is not declared in this module", callee.text),
                    "call one exact private same-module Vec function",
                );
                return None;
            }
        };
        let exact_parameters = !signature.has_borrow_parameters()
            && (signature.parameters.is_empty()
                || signature.parameters.as_slice() == [self.vec_ty]);
        if !signature.private
            || signature.result != expected
            || expected != self.vec_ty
            || !exact_parameters
        {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "call signature is outside the sealed Vec producer/identity checkpoint",
                "call a private zero-argument producer or one-exact-Vec identity function",
            );
            return None;
        }
        Some(signature)
    }

    #[allow(clippy::too_many_lines)]
    fn direct_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let signature = self.resolve_owned_callee(callee, expected)?;
        if arguments.len() != signature.parameters.len() {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameters.len()
                ),
                "pass the exact declared Vec argument",
            );
            return None;
        }
        let diagnostics_before_preflight = self.errors.len();
        let preparation = arguments.first().and_then(|argument| {
            self.estimate_vec_preparation(*argument, self.vec_ty, self.owners.pending().len(), at)
        });
        if self.errors.len() != diagnostics_before_preflight {
            return None;
        }
        let moves_existing_owner = arguments.first().is_some_and(|argument| {
            self.expression(*argument).is_some_and(|expression| {
                matches!(&expression.kind, RawExpressionKind::Reference { name }
                if self.bindings.get(&name.text).is_some_and(|binding| {
                    binding.ty == self.vec_ty && self.owners.contains(binding.place)
                }))
            })
        });
        let reserved_actions = if let Some(preparation) = preparation {
            let Some(actions) = preparation.end_pending.checked_sub(arguments.len()) else {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "Vec call preparation underflows its checked owner estimate",
                    "reduce nested owned Vec call arguments",
                );
                return None;
            };
            if !self.preflight_string_sequence_with_enclosing_cleanup(
                preparation.resources,
                actions,
                at,
            ) {
                return None;
            }
            actions
        } else {
            cleanup_actions_after_transfer(self.owners.pending().len(), moves_existing_owner)
        };
        self.cfg.reserve_values(1, at, self.errors)?;
        if !self.reserve_local_place(at) {
            self.cfg.release_values(1);
            return None;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if !self.reserve_cleanup_capacity(reserved_actions, at) {
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        let prepared = (|| {
            let mut lowered = Vec::with_capacity(arguments.len());
            let mut transferred = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let value = self.value(*argument, self.vec_ty)?;
                let Some(owner) = self.owners.owner(value) else {
                    self.errors.at(
                        "ZRYNA-M3014",
                        at,
                        "owned Vec call argument has no available owner",
                        "pass each Vec value exactly once",
                    );
                    return None;
                };
                transferred.push((value, owner));
                lowered.push(raw::CallArgument::Value(value));
            }
            Some((lowered, transferred))
        })();
        let Some((lowered, transferred)) = prepared else {
            self.release_cleanup_capacity(reserved_actions);
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        };
        let cleanup = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).ok()?);
        for (value, owner) in transferred {
            if !self.transfer_owner(value) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "owned Vec call argument has no unique available owner",
                    "pass each Vec value exactly once",
                );
                self.release_cleanup_capacity(reserved_actions);
                self.cfg.release_transitions(1);
                self.release_local_place();
                self.cfg.release_values(1);
                return None;
            }
            debug_assert!(!self.owners.contains(owner));
        }
        self.release_cleanup_capacity(reserved_actions);
        self.cfg.release_transitions(1);
        self.release_local_place();
        self.cfg.release_values(1);
        let committed_cleanup = self.push_cleanup(at, None)?;
        debug_assert_eq!(committed_cleanup, cleanup);
        Some(
            self.emit(
                expected,
                at,
                raw::InstructionKind::DirectCall {
                    callee: signature.id,
                    arguments: lowered,
                    cleanup: committed_cleanup,
                },
            )?
            .0,
        )
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_vec_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    catalog: &'a FunctionCatalog,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let mut vec_ty = (result.category == TypeCategory::Vec).then_some(result);
    let mut parameter_types = Vec::with_capacity(function.parameters.len());
    for parameter in &function.parameters {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            errors,
        )?;
        if ty.category == TypeCategory::Vec {
            if vec_ty.is_some_and(|found| found != ty) {
                errors.at(
                    "ZRYNA-M3013",
                    span(input.sources(), parameter.span),
                    "function uses more than one exact Vec type",
                    "use one exact Vec<bool>, Vec<i32>, or Vec<String> type",
                );
                return None;
            }
            vec_ty = Some(ty);
        }
        parameter_types.push(ty);
    }
    for statement in &function.body.statements {
        if let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind {
            let ty =
                semantic_type(file, type_syntax, module, declarations, graph, node_types, errors)?;
            if ty.category == TypeCategory::Vec {
                if vec_ty.is_some_and(|found| found != ty) {
                    errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), statement.span),
                        "function uses more than one exact Vec type",
                        "use one exact Vec<bool>, Vec<i32>, or Vec<String> type",
                    );
                    return None;
                }
                vec_ty = Some(ty);
            }
        }
    }
    let Some(vec_ty) = vec_ty else {
        errors.at(
            "ZRYNA-M3013",
            span(input.sources(), function.span),
            "private Vec operation has no exact declared Vec type",
            "declare and initialize one exact Vec<bool>, Vec<i32>, or Vec<String> local",
        );
        return None;
    };
    if parameter_types.len() > 1
        || parameter_types
            .iter()
            .any(|parameter| *parameter != vec_ty && parameter.category != TypeCategory::Bool)
    {
        errors.at(
            "ZRYNA-M3013",
            span(input.sources(), function.span),
            "private Vec functions admit at most one exact Vec or bool parameter",
            "use a zero-argument producer, one-Vec identity, or one-bool branch function",
        );
        return None;
    }
    let element_layout = layouts.type_by_id(vec_ty.layout)?.referenced_type()?;
    let element = node_types.iter().flatten().find(|ty| ty.layout == element_layout).copied()?;
    if !matches!(element.category, TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String) {
        errors.at(
            "ZRYNA-M3013",
            span(input.sources(), function.span),
            "this Vec element type is outside the bounded owned-data slice",
            "use Vec<bool>, Vec<i32>, or Vec<String>",
        );
        return None;
    }
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let cfg = OwnedCfgState::single_block(span(input.sources(), function.body.span), errors)?;
    let mut lowerer = PrivateVecLowerer {
        input,
        file,
        function,
        module,
        declarations,
        graph,
        node_types,
        catalog,
        vec_ty,
        element,
        errors,
        bindings: BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState::default(),
        known_string_bytes: BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (index, (parameter, ty)) in function.parameters.iter().zip(parameter_types).enumerate() {
        let parameter_span = span(input.sources(), parameter.span);
        if !preflight_owned_place_capacity(lowerer.places.len(), 1, parameter_span, lowerer.errors)
        {
            return None;
        }
        if lowerer.bindings.keys().any(|name| name.eq_ignore_ascii_case(&parameter.name.text)) {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!(
                    "parameter '{}' collides under portable ASCII case folding",
                    parameter.name.text
                ),
                "give every parameter one portable case-insensitive unique name",
            );
            return None;
        }
        let value = raw::ValueId(lowerer.next_value);
        let parameter_definition =
            raw::ValueDefinition { id: value, ty: ty.ir, span: parameter_span };
        lowerer.cfg.seed_function_parameter(&parameter_definition, lowerer.errors)?;
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        parameters.push(parameter_definition);
        let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        if !ty.is_copy() {
            let _ = lowerer.owners.register_parameter(place);
        }
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    }
    if root_is_terminal_if(function) {
        if result != vec_ty
            || lowerer.bindings.values().any(|binding| binding.ty.category != TypeCategory::Bool)
        {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), function.span),
                "terminal owned Vec if admits an exact Vec result and one optional bool parameter",
                "return one exact Vec value from both branches",
            );
            return None;
        }
        let terminal = terminal_owned_if(function, input.sources(), lowerer.errors)?;
        let bool_ty =
            node_types.iter().flatten().find(|ty| ty.category == TypeCategory::Bool).copied()?;
        if !lowerer.cfg.preflight_skeleton(3, 4, terminal.span, lowerer.errors) {
            return None;
        }
        lowerer.cfg.reserve_values(1, terminal.span, lowerer.errors)?;
        if !lowerer.reserve_local_place(terminal.span) {
            lowerer.cfg.release_values(1);
            return None;
        }
        if !lowerer.reserve_cleanup_capacity(0, terminal.span) {
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        let Some(condition) = lowerer.condition(terminal.condition, bool_ty) else {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        };
        let then_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        let else_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        let join_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        if !lowerer.cfg.terminate(
            raw::SpannedTerminator {
                span: terminal.span,
                kind: raw::Terminator::Branch {
                    condition,
                    when_true: raw::Edge { target: then_id, arguments: Vec::new() },
                    when_false: raw::Edge { target: else_id, arguments: Vec::new() },
                },
            },
            lowerer.errors,
        ) {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        let incoming = OwnedVecBranchState {
            bindings: lowerer.bindings.clone(),
            owners: lowerer.owners.clone(),
            known_string_bytes: lowerer.known_string_bytes.clone(),
        };
        let arms_lowered = (|| {
            for (block, expression, arm_span) in [
                (then_id, terminal.then_value, terminal.then_span),
                (else_id, terminal.else_value, terminal.else_span),
            ] {
                lowerer.cfg.begin_block(block, Vec::new(), arm_span, lowerer.errors)?;
                let value = lowerer.value(expression, result)?;
                let Some(carried) = lowerer.owners.owner(value) else {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        arm_span,
                        "terminal Vec arm result has no available owner",
                        "return one newly produced exact Vec value",
                    );
                    return None;
                };
                lowerer.drop_non_carried(carried, arm_span)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: arm_span,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: vec![value],
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings.clone();
                lowerer.owners = incoming.owners.clone();
                lowerer.known_string_bytes = incoming.known_string_bytes.clone();
            }
            Some(())
        })();
        if arms_lowered.is_none() {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        lowerer.release_cleanup_capacity(0);
        lowerer.release_local_place();
        lowerer.cfg.release_values(1);
        let joined = raw::ValueId(lowerer.next_value);
        let joined_definition =
            raw::ValueDefinition { id: joined, ty: result.ir, span: terminal.span };
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        lowerer.cfg.begin_block(join_id, vec![joined_definition], terminal.span, lowerer.errors)?;
        let joined_owner = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: joined_owner,
            ty: result.ir,
            span: terminal.span,
            kind: raw::PlaceKind::Temporary(joined),
        });
        let _ = lowerer.owners.register(joined, joined_owner);
        let cleanup = lowerer.push_cleanup(terminal.span, Some(joined_owner))?;
        if !lowerer.cfg.terminate(
            raw::SpannedTerminator {
                span: terminal.span,
                kind: raw::Terminator::Return { value: joined, cleanup },
            },
            lowerer.errors,
        ) {
            return None;
        }
        let blocks = lowerer.cfg.finish(terminal.span, lowerer.errors)?;
        return Some(raw::Function {
            id: raw::FunctionId {
                module: raw::ModuleId(u32::try_from(module).ok()?),
                declaration: u32::try_from(declaration).ok()?,
            },
            entry_export: None,
            span: span(input.sources(), function.span),
            parameters,
            borrow_parameters: Vec::new(),
            result: result.ir,
            places: lowerer.places,
            blocks,
            cleanup_plans: lowerer.cleanup_plans,
        });
    }
    let mut returned = None;
    let mut saw_if = false;
    let mut saw_loop = false;
    for statement_id in &root.statements {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        match &statement.kind {
            RawStatementKind::LocalDeclaration { .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "owned Vec control flow must immediately precede the final return",
                        "move every outer declaration before the single top-level control-flow statement",
                    );
                    return None;
                }
                lowerer.lower_local(statement)?;
            }
            RawStatementKind::Return { value, .. } => {
                returned =
                    Some((lowerer.value(*value, result)?, span(input.sources(), statement.span)));
            }
            RawStatementKind::Assignment { target, value, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "owned Vec control-flow lowering excludes assignment after its exit",
                        "leave the joined outer owned state unchanged and return it directly",
                    );
                    return None;
                }
                let target_expression = lowerer.expression(*target)?.clone();
                let RawExpressionKind::Reference { name } = target_expression.kind else {
                    lowerer.errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), target_expression.span),
                        "owned assignment requires one root local target",
                        "assign only to an initialized mutable String or exact Vec local",
                    );
                    return None;
                };
                let Some(binding) = lowerer.bindings.get(&name.text).cloned() else {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!("owned assignment target '{}' is not declared", name.text),
                        "assign one exact preceding local",
                    );
                    return None;
                };
                if binding.ty.is_copy()
                    || !matches!(binding.ty.category, TypeCategory::String | TypeCategory::Vec)
                    || (binding.ty.category == TypeCategory::Vec && binding.ty != vec_ty)
                {
                    lowerer.errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), name.span),
                        "assignment target is outside the exact supported owned type",
                        "assign only to String or the function's exact Vec type",
                    );
                    return None;
                }
                if !binding.mutable || !lowerer.owners.contains(binding.place) {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), name.span),
                        "owned assignment target is immutable, uninitialized, or already moved",
                        "assign only to an initialized mutable available owned local",
                    );
                    return None;
                }
                if let Some(reference_span) =
                    lowerer.target_consumption_span(*value, binding.place, true)
                {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), reference_span),
                        "owned assignment cannot consume its destination while preparing its replacement",
                        "prepare a distinct owned value before replacement",
                    );
                    return None;
                }
                let assignment_span = span(input.sources(), statement.span);
                if !reserve_owned_commit_transition(
                    &mut lowerer.cfg,
                    assignment_span,
                    lowerer.errors,
                ) {
                    return None;
                }
                let Some(prepared) = lowerer.value(*value, binding.ty) else {
                    release_owned_commit_transition(&mut lowerer.cfg);
                    return None;
                };
                release_owned_commit_transition(&mut lowerer.cfg);
                if !lowerer.emit_effect(
                    assignment_span,
                    raw::InstructionKind::ReplacePlace { place: binding.place, value: prepared },
                ) {
                    return None;
                }
                if !lowerer.replace_owner(prepared, binding.place) {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), statement.span),
                        "owned assignment replacement has no distinct prepared owner",
                        "replace from one available independently prepared owned value",
                    );
                    return None;
                }
            }
            RawStatementKind::ExpressionStatement { expression, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "owned Vec control-flow lowering excludes effects after its exit",
                        "leave the joined outer owned state unchanged and return it directly",
                    );
                    return None;
                }
                lowerer.lower_push_effect(*expression, None)?;
            }
            RawStatementKind::If { condition, then_block, else_clause, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "nested or repeated owned Vec if statements are not supported",
                        "use exactly one top-level if before the final return",
                    );
                    return None;
                }
                saw_if = true;
                let bool_ty = node_types
                    .iter()
                    .flatten()
                    .find(|ty| ty.category == TypeCategory::Bool)
                    .copied()?;
                let condition = lowerer.condition(*condition, bool_ty)?;
                let at = span(input.sources(), statement.span);
                let then_id = lowerer.cfg.reserve_block(at, lowerer.errors)?;
                let else_id = lowerer.cfg.reserve_block(at, lowerer.errors)?;
                let join_id = lowerer.cfg.reserve_block(at, lowerer.errors)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: then_id, arguments: Vec::new() },
                            when_false: raw::Edge { target: else_id, arguments: Vec::new() },
                        },
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                let incoming = OwnedVecBranchState {
                    bindings: lowerer.bindings.clone(),
                    owners: lowerer.owners.clone(),
                    known_string_bytes: lowerer.known_string_bytes.clone(),
                };
                lowerer.cfg.begin_block(then_id, Vec::new(), at, lowerer.errors)?;
                lowerer.lower_branch(Some(*then_block), &incoming, at)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings.clone();
                lowerer.owners = incoming.owners.clone();
                lowerer.known_string_bytes = incoming.known_string_bytes.clone();
                lowerer.cfg.begin_block(else_id, Vec::new(), at, lowerer.errors)?;
                lowerer.lower_branch(
                    else_clause.as_ref().map(|clause| clause.block),
                    &incoming,
                    at,
                )?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings;
                lowerer.owners = incoming.owners;
                lowerer.known_string_bytes = incoming.known_string_bytes;
                lowerer.cfg.begin_block(join_id, Vec::new(), at, lowerer.errors)?;
            }
            RawStatementKind::While { condition, body_block, .. } => {
                if saw_if || saw_loop {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), statement.span),
                        "nested or repeated owned Vec loops are not supported",
                        "use exactly one top-level while before the final return",
                    );
                    return None;
                }
                saw_loop = true;
                let bool_ty = node_types
                    .iter()
                    .flatten()
                    .find(|ty| ty.category == TypeCategory::Bool)
                    .copied()?;
                let at = span(input.sources(), statement.span);
                if !preflight_owned_loop_exit(
                    function,
                    *statement_id,
                    input.sources(),
                    lowerer.errors,
                ) {
                    return None;
                }
                if !preflight_owned_loop_body(
                    function,
                    *body_block,
                    true,
                    input.sources(),
                    lowerer.errors,
                ) {
                    return None;
                }
                let body = usize::try_from(*body_block)
                    .ok()
                    .and_then(|index| function.body.blocks.get(index))?;
                let push = match body.statements.as_slice() {
                    [effect_id] => usize::try_from(*effect_id)
                        .ok()
                        .and_then(|index| function.body.statements.get(index))
                        .and_then(|statement| match statement.kind {
                            RawStatementKind::ExpressionStatement { expression, .. } => {
                                Some(expression)
                            }
                            _ => None,
                        }),
                    _ => None,
                };
                if push.is_some() && lowerer.owners.pending().len() != 1 {
                    lowerer.errors.at(
                        "ZRYNA-M3016",
                        at,
                        "Vec mutation loop requires exactly one incoming owned root",
                        "declare one mutable outer exact Vec before the loop",
                    );
                    return None;
                }
                if !lowerer.cfg.preflight_skeleton(3, 4, at, lowerer.errors) {
                    return None;
                }
                let header_id = lowerer.cfg.reserve_block(at, lowerer.errors).expect("preflight");
                let body_id = lowerer.cfg.reserve_block(at, lowerer.errors).expect("preflight");
                let exit_id = lowerer.cfg.reserve_block(at, lowerer.errors).expect("preflight");
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: header_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                let incoming = OwnedVecBranchState {
                    bindings: lowerer.bindings.clone(),
                    owners: lowerer.owners.clone(),
                    known_string_bytes: lowerer.known_string_bytes.clone(),
                };
                lowerer.cfg.begin_block(header_id, Vec::new(), at, lowerer.errors)?;
                let condition = lowerer.condition(*condition, bool_ty)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: body_id, arguments: Vec::new() },
                            when_false: raw::Edge { target: exit_id, arguments: Vec::new() },
                        },
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.cfg.begin_block(body_id, Vec::new(), at, lowerer.errors)?;
                if let Some(push) = push {
                    lowerer.lower_loop_push(push, &incoming, at)?;
                } else {
                    lowerer.lower_branch(Some(*body_block), &incoming, at)?;
                }
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: header_id,
                            arguments: Vec::new(),
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings;
                lowerer.owners = incoming.owners;
                lowerer.known_string_bytes = incoming.known_string_bytes;
                lowerer.cfg.begin_block(exit_id, Vec::new(), at, lowerer.errors)?;
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3013",
                    span(input.sources(), statement.span),
                    "statement is outside private straight-line Vec lowering",
                    "use typed locals and one final Vec return",
                );
                return None;
            }
        }
    }
    let (returned, return_span) = returned?;
    let return_owner = lowerer.owners.owner(returned);
    let cleanup = lowerer.push_cleanup(return_span, return_owner)?;
    if !lowerer.cfg.terminate(
        raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: returned, cleanup },
        },
        lowerer.errors,
    ) {
        return None;
    }
    let blocks = lowerer.cfg.finish(return_span, lowerer.errors)?;
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters: Vec::new(),
        result: result.ir,
        places: lowerer.places,
        blocks,
        cleanup_plans: lowerer.cleanup_plans,
    })
}

fn verify_single_final_return(
    function: &syntax::RawFunctionSyntax,
    sources: &SourceMap,
    errors: &mut Errors<'_>,
) -> Option<()> {
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let mut first_return = None;
    for (position, statement_id) in root.statements.iter().enumerate() {
        let Some(statement) = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))
        else {
            continue;
        };
        if matches!(statement.kind, RawStatementKind::Return { .. }) {
            if first_return.replace(position).is_some() {
                errors.at(
                    "ZRYNA-M3010",
                    span(sources, statement.span),
                    "function contains more than one return",
                    "use exactly one return as the final root-block statement",
                );
                return None;
            }
        } else if first_return.is_some() {
            errors.at(
                "ZRYNA-M3010",
                span(sources, statement.span),
                "statement appears after the function return",
                "make the single return the final root-block statement",
            );
            return None;
        }
    }
    if first_return == root.statements.len().checked_sub(1) {
        Some(())
    } else {
        errors.at(
            "ZRYNA-M3010",
            span(sources, function.body.span),
            "function must end with exactly one value return",
            "make one return the final root-block statement",
        );
        None
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_enum_match_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let root =
        usize::try_from(function.body.root_block).ok().and_then(|i| function.body.blocks.get(i))?;
    let [statement_id] = root.statements.as_slice() else {
        return None;
    };
    let statement =
        usize::try_from(*statement_id).ok().and_then(|i| function.body.statements.get(i))?;
    let RawStatementKind::Return { value: returned, .. } = statement.kind else {
        return None;
    };
    let match_expr =
        usize::try_from(returned).ok().and_then(|i| function.body.expressions.get(i))?;
    let RawExpressionKind::Match { scrutinee, arms, .. } = &match_expr.kind else {
        return None;
    };

    let mut parameters = Vec::with_capacity(function.parameters.len());
    let mut places = Vec::with_capacity(function.parameters.len() + arms.len());
    let mut bindings: BTreeMap<String, Binding> = BTreeMap::new();
    let mut next_value = 0_u32;
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            errors,
        )?;
        require_current_type_only_boundary(
            ty,
            span(input.sources(), parameter.span),
            function.export_span.is_some(),
            errors,
        )?;
        parameters.push(raw::ValueDefinition {
            id: raw::ValueId(next_value),
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
        });
        next_value = next_value.checked_add(1)?;
        let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
        places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        if bindings.keys().any(|name| name.eq_ignore_ascii_case(&parameter.name.text)) {
            errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!("parameter '{}' is declared more than once", parameter.name.text),
                "give every parameter one exact name",
            );
            return None;
        }
        bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    }
    let scrutinee_expr =
        usize::try_from(*scrutinee).ok().and_then(|i| function.body.expressions.get(i))?;
    let RawExpressionKind::Reference { name: scrutinee_name } = &scrutinee_expr.kind else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expr.span),
            "enum match scrutinee must be an addressable place",
            "match a parameter or initialized local enum place",
        );
        return None;
    };
    let scrutinee_binding = bindings.get(&scrutinee_name.text).cloned().or_else(|| {
        errors.at(
            "ZRYNA-M3002",
            span(input.sources(), scrutinee_name.span),
            format!("name '{}' is not declared", scrutinee_name.text),
            "match one declared enum place",
        );
        None
    })?;
    let record = layouts.type_by_id(scrutinee_binding.ty.layout)?;
    let nominal = record.nominal_identity().or_else(|| {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expr.span),
            "match scrutinee is not a nominal enum",
            "match a value of one exact declared enum type",
        );
        None
    })?;
    if record.category() != TypeCategory::Enum {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expr.span),
            "match scrutinee is not an enum",
            "match a value of one exact declared enum type",
        );
        return None;
    }
    let nominal_module = usize::try_from(nominal.0).ok()?;
    let nominal_declaration = usize::try_from(nominal.1).ok()?;
    let enum_decl = declarations
        .iter()
        .find(|d| d.module == nominal_module && d.declaration == nominal_declaration)?;
    let RawDataDeclarationKind::Enum { variants, .. } =
        &input.syntax().files()[enum_decl.module].data_declarations()[enum_decl.declaration].kind
    else {
        return None;
    };
    if arms.len() != variants.len() {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), match_expr.span),
            format!(
                "enum match has {} arms but '{}' has {} variants",
                arms.len(),
                enum_decl.name,
                variants.len()
            ),
            "provide every variant exactly once and no extra arms",
        );
        return None;
    }

    let mut blocks = vec![raw::Block {
        id: raw::BlockId(0),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: Vec::new(),
    }];
    let mut enum_arms = Vec::with_capacity(arms.len());
    let mut cleanup_plans = Vec::with_capacity(arms.len());
    let mut seen = vec![false; variants.len()];
    let variant_ordinal = |arm: &syntax::RawMatchArm| {
        variants
            .iter()
            .position(|variant| variant.name.text == arm.variant.text)
            .unwrap_or(usize::MAX)
    };
    let mut ordered_arms = arms.iter().collect::<Vec<_>>();
    ordered_arms.sort_by_key(|arm| variant_ordinal(arm));
    for arm in ordered_arms {
        if arm.type_name.text != enum_decl.name {
            errors.at(
                "ZRYNA-M3009",
                span(input.sources(), arm.type_name.span),
                "match arm names a different enum",
                "use the scrutinee's exact enum name on every arm",
            );
            return None;
        }
        let (ordinal, variant) =
            variants.iter().enumerate().find(|(_, v)| v.name.text == arm.variant.text).or_else(
                || {
                    errors.at(
                        "ZRYNA-M3009",
                        span(input.sources(), arm.variant.span),
                        format!("enum '{}' has no variant '{}'", enum_decl.name, arm.variant.text),
                        "use every declared variant exactly once",
                    );
                    None
                },
            )?;
        if seen[ordinal] {
            errors.at(
                "ZRYNA-M3009",
                span(input.sources(), arm.variant.span),
                format!("variant '{}' appears more than once", arm.variant.text),
                "provide every variant exactly once",
            );
            return None;
        }
        seen[ordinal] = true;
        let block_id = raw::BlockId(u32::try_from(blocks.len()).ok()?);
        let mut arm_bindings = bindings.clone();
        match (variant.payload_type, &arm.binding) {
            (None, None) => {}
            (Some(payload_type), Some(binding)) => {
                let payload_ty = semantic_type(
                    file,
                    payload_type,
                    module,
                    declarations,
                    graph,
                    node_types,
                    errors,
                )?;
                let payload_place = raw::PlaceId(u32::try_from(places.len()).ok()?);
                places.push(raw::Place {
                    id: payload_place,
                    ty: payload_ty.ir,
                    span: span(input.sources(), binding.span),
                    kind: raw::PlaceKind::EnumPayload {
                        base: scrutinee_binding.place,
                        variant: u32::try_from(ordinal).ok()?,
                    },
                });
                if arm_bindings.keys().any(|name| name.eq_ignore_ascii_case(&binding.text)) {
                    errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), binding.span),
                        format!(
                            "match binding '{}' collides under portable ASCII case folding",
                            binding.text
                        ),
                        "choose a binding distinct from parameters and locals",
                    );
                    return None;
                }
                arm_bindings.insert(
                    binding.text.clone(),
                    Binding { ty: payload_ty, place: payload_place, mutable: false },
                );
            }
            _ => {
                errors.at(
                    "ZRYNA-M3009",
                    span(input.sources(), arm.span),
                    "match payload binding does not match the declared variant",
                    "bind exactly one name only on a payload variant",
                );
                return None;
            }
        }
        let arm_expr =
            usize::try_from(arm.value).ok().and_then(|i| function.body.expressions.get(i))?;
        let arm_span = span(input.sources(), arm_expr.span);
        let (arm_ty, instruction_kind) = match &arm_expr.kind {
            RawExpressionKind::I32Literal { spelling } => {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    errors.at(
                        "ZRYNA-M3008",
                        arm_span,
                        "match-arm integer literal is outside i32",
                        "use an i32 literal",
                    );
                    None
                })?;
                let ty = node_types.iter().flatten().find(|ty| ty.category == TypeCategory::I32)?;
                let ty = *ty;
                (ty, raw::InstructionKind::I32Literal(value))
            }
            RawExpressionKind::BoolLiteral { value } => {
                let ty =
                    node_types.iter().flatten().find(|ty| ty.category == TypeCategory::Bool)?;
                let ty = *ty;
                (ty, raw::InstructionKind::BoolLiteral(*value))
            }
            RawExpressionKind::Reference { name } => {
                let binding = arm_bindings.get(&name.text).cloned().or_else(|| {
                    errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!("match-arm name '{}' is not declared", name.text),
                        "reference the payload binding or a parameter",
                    );
                    None
                })?;
                (binding.ty, raw::InstructionKind::CopyFromPlace { place: binding.place })
            }
            _ => {
                errors.at(
                    "ZRYNA-M3009",
                    arm_span,
                    "nested aggregate operations in match arms are outside the M3 match oracle",
                    "return a scalar literal, parameter, or payload binding from each arm",
                );
                return None;
            }
        };
        if arm_ty.layout != result.layout {
            errors.at(
                "ZRYNA-M3009",
                arm_span,
                "enum match arms do not all have the declared result type",
                "make every arm produce one exact common type",
            );
            return None;
        }
        let value_id = raw::ValueId(next_value);
        next_value = next_value.checked_add(1)?;
        let definition = raw::ValueDefinition { id: value_id, ty: arm_ty.ir, span: arm_span };
        blocks.push(raw::Block {
            id: block_id,
            parameters: Vec::new(),
            instructions: vec![raw::Instruction {
                result: Some(definition),
                span: arm_span,
                kind: instruction_kind,
            }],
            terminators: Vec::new(),
        });
        enum_arms.push(raw::EnumArm {
            variant: u32::try_from(ordinal).ok()?,
            edge: raw::Edge { target: block_id, arguments: Vec::new() },
        });
        let arm_block = blocks.last_mut().expect("just pushed match arm block");
        let cleanup = raw::CleanupPlanId(u32::try_from(cleanup_plans.len()).ok()?);
        cleanup_plans.push(raw::CleanupPlan {
            id: cleanup,
            span: span(input.sources(), arm.span),
            actions: Vec::new(),
        });
        arm_block.terminators.push(raw::SpannedTerminator {
            span: span(input.sources(), arm.span),
            kind: raw::Terminator::Return { value: value_id, cleanup },
        });
    }
    blocks[0].terminators.push(raw::SpannedTerminator {
        span: span(input.sources(), match_expr.span),
        kind: raw::Terminator::EnumMatch { place: scrutinee_binding.place, arms: enum_arms },
    });
    if function.export_span.is_some() {
        errors.at(
            "ZRYNA-M3010",
            span(input.sources(), function.span),
            "public aggregate signatures are outside scalar ABI v1",
            "keep enum-match functions internal and export only bool/i32 signatures",
        );
        return None;
    }
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters: Vec::new(),
        result: result.ir,
        places,
        blocks,
        cleanup_plans,
    })
}

impl FunctionLowerer<'_, '_, '_> {
    fn binding_name_exists(&self, candidate: &str) -> bool {
        self.bindings
            .keys()
            .chain(self.borrow_bindings.keys())
            .any(|name| name.eq_ignore_ascii_case(candidate))
    }

    fn borrow_reference(&self, id: u32) -> Option<BorrowBinding> {
        let expression =
            usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))?;
        let RawExpressionKind::Reference { name } = &expression.kind else {
            return None;
        };
        self.borrow_bindings.get(&name.text).copied()
    }

    fn push_place(&mut self, ty: Ty, span: Span, kind: raw::PlaceKind) -> raw::PlaceId {
        let id = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
        self.places.push(raw::Place { id, ty: ty.ir, span, kind });
        id
    }
    fn emit(
        &mut self,
        result_ty: Option<Ty>,
        span: Span,
        kind: raw::InstructionKind,
    ) -> Option<raw::ValueId> {
        let result = result_ty.map(|ty| {
            let id = raw::ValueId(self.values);
            self.values += 1;
            raw::ValueDefinition { id, ty: ty.ir, span }
        });
        let id = result.map(|v| v.id);
        self.instructions.push(raw::Instruction { result, span, kind });
        id
    }
    fn require_type(&mut self, expected: Ty, actual: Ty, at: Span, what: &str) -> Option<()> {
        if expected.layout == actual.layout {
            Some(())
        } else {
            self.errors.at(
                "ZRYNA-M3007",
                at,
                format!("{what} has a different exact aggregate type"),
                "use a value with the exact declared type",
            );
            None
        }
    }
    fn value(&mut self, id: u32) -> Option<(Ty, raw::ValueId)> {
        let expr = usize::try_from(id).ok().and_then(|i| self.function.body.expressions.get(i))?;
        let at = span(self.input.sources(), expr.span);
        match &expr.kind {
            RawExpressionKind::Reference { name } => {
                if let Some(binding) = self.borrow_bindings.get(&name.text).copied() {
                    let value = self.emit(
                        Some(binding.ty),
                        at,
                        raw::InstructionKind::BorrowRead { borrow: binding.borrow },
                    )?;
                    return Some((binding.ty, value));
                }
                let (ty, place, _) = self.place(id)?;
                let value =
                    self.emit(Some(ty), at, raw::InstructionKind::CopyFromPlace { place })?;
                Some((ty, value))
            }
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                let (ty, place, _) = self.place(id)?;
                let value =
                    self.emit(Some(ty), at, raw::InstructionKind::CopyFromPlace { place })?;
                Some((ty, value))
            }
            RawExpressionKind::BoolLiteral { value } => {
                let ty = self.primitive(TypeCategory::Bool)?;
                let id = self.emit(Some(ty), at, raw::InstructionKind::BoolLiteral(*value))?;
                Some((ty, id))
            }
            RawExpressionKind::I32Literal { spelling } => {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3008",
                        at,
                        format!("integer literal '{spelling}' is outside i32"),
                        "use a decimal i32 literal",
                    );
                    None
                })?;
                let ty = self.primitive(TypeCategory::I32)?;
                let id = self.emit(Some(ty), at, raw::InstructionKind::I32Literal(value))?;
                Some((ty, id))
            }
            RawExpressionKind::StructConstruction { type_name, fields, .. } => {
                self.struct_value(type_name, fields, at)
            }
            RawExpressionKind::EnumConstruction { type_name, variant, payload, .. } => {
                self.enum_value(type_name, variant, *payload, at)
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. } => {
                self.array_value(*type_syntax, elements, at)
            }
            RawExpressionKind::Call { callee, arguments, .. } => {
                self.direct_call(callee, arguments, at)
            }
            RawExpressionKind::Negation { operand, .. } => self.unary_i32(*operand, at),
            RawExpressionKind::Addition { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, |lhs, rhs| raw::InstructionKind::I32Add {
                    lhs,
                    rhs,
                })
            }
            RawExpressionKind::Subtraction { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, |lhs, rhs| raw::InstructionKind::I32Sub {
                    lhs,
                    rhs,
                })
            }
            RawExpressionKind::Multiplication { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, |lhs, rhs| raw::InstructionKind::I32Mul {
                    lhs,
                    rhs,
                })
            }
            RawExpressionKind::Equal { lhs, rhs, .. } => self.compare(*lhs, *rhs, at, false),
            RawExpressionKind::NotEqual { lhs, rhs, .. } => self.compare(*lhs, *rhs, at, true),
            RawExpressionKind::LessThan { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 0),
            RawExpressionKind::LessEqual { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 1),
            RawExpressionKind::GreaterThan { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 2),
            RawExpressionKind::GreaterEqual { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 3),
            _ => {
                self.errors.at(
                    "ZRYNA-M3008",
                    at,
                    "expression is outside deterministic aggregate M3",
                    "use Copy construction, projection, or scalar operations",
                );
                None
            }
        }
    }
    fn lower_direct_call_arguments(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[u32],
    ) -> Option<Vec<raw::CallArgument>> {
        let mut values = vec![None; signature.parameters.len()];
        let mut borrows = vec![None; signature.borrow_parameters.len()];
        let mut exclusive_arguments = Vec::new();
        for (argument, order) in arguments.iter().zip(&signature.parameter_order) {
            let argument_span =
                span(self.input.sources(), self.function.body.expressions[*argument as usize].span);
            match *order {
                FunctionParameterOrder::Value(index) => {
                    let expected = *signature.parameters.get(usize::try_from(index).ok()?)?;
                    let (actual, value) = self.value(*argument)?;
                    self.require_type(expected, actual, argument_span, "call argument")?;
                    *values.get_mut(usize::try_from(index).ok()?)? = Some(value);
                }
                FunctionParameterOrder::Borrow(index) => {
                    let expected =
                        *signature.borrow_parameters.get(usize::try_from(index).ok()?)?;
                    let Some(actual) = self.borrow_reference(*argument) else {
                        self.errors.at(
                            "ZRYNA-M3016",
                            argument_span,
                            "borrow arguments must forward an in-scope borrow parameter",
                            "pass an exact Borrow or BorrowMut parameter; lexical call borrows are not enabled",
                        );
                        return None;
                    };
                    if actual.ty.layout != expected.referent.layout
                        || actual.access != expected.access
                    {
                        self.errors.at(
                            "ZRYNA-M3016",
                            argument_span,
                            "borrow argument does not match the callee referent and access",
                            "pass an exact borrow parameter with the declared referent and shared or exclusive access",
                        );
                        return None;
                    }
                    if expected.access == raw::BorrowAccess::Exclusive
                        && exclusive_arguments.contains(&actual.borrow)
                    {
                        self.errors.at(
                            "ZRYNA-M3016",
                            argument_span,
                            "exclusive borrow arguments cannot reuse the same authority",
                            "pass distinct nonoverlapping exclusive borrow parameters",
                        );
                        return None;
                    }
                    if expected.access == raw::BorrowAccess::Exclusive {
                        exclusive_arguments.push(actual.borrow);
                    }
                    *borrows.get_mut(usize::try_from(index).ok()?)? = Some(actual.borrow);
                }
            }
        }
        let mut lowered = Vec::with_capacity(arguments.len());
        for value in values {
            lowered.push(raw::CallArgument::Value(value?));
        }
        for borrow in borrows {
            lowered.push(raw::CallArgument::Borrow(borrow?));
        }
        Some(lowered)
    }

    fn direct_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let signature = match self.catalog.resolve(self.module, &callee.text) {
            FunctionResolution::Exact(signature) => signature.clone(),
            FunctionResolution::WrongCase => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("call name '{}' has the wrong portable ASCII case", callee.text),
                    "use the callee's exact declared spelling",
                );
                return None;
            }
            FunctionResolution::Missing => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("function '{}' is not declared in this module", callee.text),
                    "call one exact private same-module function",
                );
                return None;
            }
        };
        if !signature.private {
            self.errors.at(
                "ZRYNA-M3008",
                span(self.input.sources(), callee.span),
                "this checkpoint admits calls only to private same-module functions",
                "keep the called function internal",
            );
            return None;
        }
        if !signature.result.is_copy() || signature.parameters.iter().any(|ty| !ty.is_copy()) {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned direct-call transfer is outside the current Copy-call checkpoint",
                "call only exact bool, i32, or Copy aggregate signatures",
            );
            return None;
        }
        if arguments.len() != signature.parameter_order.len() {
            self.errors.at(
                "ZRYNA-M3008",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameter_order.len()
                ),
                "pass one argument for every exact declared parameter",
            );
            return None;
        }
        let lowered = self.lower_direct_call_arguments(&signature, arguments)?;
        let cleanup = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).ok()?);
        self.cleanup_plans.push(raw::CleanupPlan { id: cleanup, span: at, actions: Vec::new() });
        let value = self.emit(
            Some(signature.result),
            at,
            raw::InstructionKind::DirectCall { callee: signature.id, arguments: lowered, cleanup },
        )?;
        Some((signature.result, value))
    }
    fn place(&mut self, id: u32) -> Option<(Ty, raw::PlaceId, bool)> {
        let expr = usize::try_from(id).ok().and_then(|i| self.function.body.expressions.get(i))?;
        match &expr.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).map(|b| (b.ty, b.place, b.mutable)).or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("name '{}' is not declared", name.text),
                        "reference one exact parameter, local, or match payload binding",
                    );
                    None
                })
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let (base_ty, base_place, mutable) = self.place(*base)?;
                let (ordinal, ty) =
                    self.field(base_ty, &field.text, span(self.input.sources(), field.span))?;
                let key = (base_place.0, 0, ordinal);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let place = self.push_place(
                        ty,
                        span(self.input.sources(), expr.span),
                        raw::PlaceKind::StructField { base: base_place, ordinal },
                    );
                    self.projections.insert(key, place);
                    place
                };
                Some((ty, place, mutable))
            }
            RawExpressionKind::Index { base, index, .. } => {
                let (base_ty, base_place, mutable) = self.place(*base)?;
                let (ordinal, ty) = self.constant_index(base_ty, *index)?;
                let key = (base_place.0, 1, ordinal);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let place = self.push_place(
                        ty,
                        span(self.input.sources(), expr.span),
                        raw::PlaceKind::FixedArrayConstant { base: base_place, index: ordinal },
                    );
                    self.projections.insert(key, place);
                    place
                };
                Some((ty, place, mutable))
            }
            RawExpressionKind::StructConstruction { .. }
            | RawExpressionKind::EnumConstruction { .. }
            | RawExpressionKind::FixedArrayConstruction { .. } => {
                let (ty, value) = self.value(id)?;
                let place = self.push_place(
                    ty,
                    span(self.input.sources(), expr.span),
                    raw::PlaceKind::Temporary(value),
                );
                Some((ty, place, false))
            }
            _ => {
                self.errors.at("ZRYNA-M3006", span(self.input.sources(), expr.span), "projection base is not an addressable aggregate place", "project from a parameter, local, aggregate constructor, field, or fixed-array element");
                None
            }
        }
    }
    fn primitive(&self, category: TypeCategory) -> Option<Ty> {
        self.node_types.iter().flatten().find(|ty| ty.category == category).copied()
    }
    fn decl_ty(&self, name: &str) -> Option<Ty> {
        let decl = self.declarations.iter().find(|d| d.module == self.module && d.name == name)?;
        self.node_types[usize::try_from(decl.node.0).ok()?]
    }
    fn field(&mut self, base: Ty, name: &str, use_span: Span) -> Option<(u32, Ty)> {
        let nominal = self.layouts.type_by_id(base.layout)?.nominal_identity()?;
        let decl = self.declarations.iter().find(|d| {
            (u32::try_from(d.module).ok(), u32::try_from(d.declaration).ok())
                == (Some(nominal.0), Some(nominal.1))
        })?;
        let raw_decl =
            &self.input.syntax().files()[decl.module].data_declarations()[decl.declaration];
        let RawDataDeclarationKind::Struct { fields, .. } = &raw_decl.kind else {
            self.errors.at(
                "ZRYNA-M3006",
                decl.span,
                "field access requires a struct",
                "project fields only from a struct place",
            );
            return None;
        };
        fields
            .iter()
            .enumerate()
            .find(|(_, f)| f.name.text == name)
            .and_then(|(ordinal, f)| {
                semantic_type(
                    self.file,
                    f.type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )
                .map(|ty| (u32::try_from(ordinal).unwrap_or(u32::MAX), ty))
            })
            .or_else(|| {
                self.errors.at(
                    "ZRYNA-M3006",
                    use_span,
                    format!("struct '{}' has no field '{name}'", decl.name),
                    "use one exact declared field name",
                );
                None
            })
    }
    fn constant_index(&mut self, base: Ty, index_expr: u32) -> Option<(u32, Ty)> {
        let expr =
            usize::try_from(index_expr).ok().and_then(|i| self.function.body.expressions.get(i))?;
        let RawExpressionKind::I32Literal { spelling } = &expr.kind else {
            self.errors.at(
                "ZRYNA-M3006",
                span(self.input.sources(), expr.span),
                "fixed-array indices must be compile-time i32 literals",
                "use a nonnegative literal within the fixed-array length",
            );
            return None;
        };
        let index = spelling.parse::<u32>().ok().or_else(|| {
            self.errors.at(
                "ZRYNA-M3006",
                span(self.input.sources(), expr.span),
                "fixed-array index is negative or outside u32",
                "use a nonnegative constant index",
            );
            None
        })?;
        let record = self.layouts.type_by_id(base.layout)?;
        let length = record.array_length()?;
        if u64::from(index) >= length {
            self.errors.at(
                "ZRYNA-M3006",
                span(self.input.sources(), expr.span),
                format!("fixed-array index {index} is outside length {length}"),
                "use an index less than the exact fixed-array length",
            );
            return None;
        }
        let element = record.referenced_type()?;
        let ty = self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        Some((index, ty))
    }
    fn struct_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        fields: &[syntax::RawFieldInitializer],
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let ty = self.decl_ty(&name.text).or_else(|| {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                format!("'{}' is not a local aggregate type", name.text),
                "construct an exact declared struct",
            );
            None
        })?;
        let decl =
            self.declarations.iter().find(|d| d.module == self.module && d.name == name.text)?;
        let RawDataDeclarationKind::Struct { fields: declared, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3005",
                at,
                "struct construction names an enum",
                "use enum variant construction for an enum",
            );
            return None;
        };
        let mut initializers = BTreeMap::new();
        for field in fields {
            let (field_name, expression) = match &field.kind {
                RawFieldInitializerKind::Shorthand { name, value }
                | RawFieldInitializerKind::Explicit { name, value, .. } => (&name.text, *value),
            };
            if declared.iter().all(|candidate| candidate.name.text != *field_name) {
                self.errors.at(
                    "ZRYNA-M3005",
                    span(self.input.sources(), field.span),
                    format!("struct '{}' has no field '{field_name}'", name.text),
                    "initialize exactly the declared field set",
                );
                return None;
            }
            if initializers
                .insert(field_name.clone(), (expression, span(self.input.sources(), field.span)))
                .is_some()
            {
                self.errors.at(
                    "ZRYNA-M3005",
                    span(self.input.sources(), field.span),
                    format!("field '{field_name}' is initialized more than once"),
                    "initialize every declared field exactly once",
                );
                return None;
            }
        }
        let mut values = Vec::with_capacity(declared.len());
        for declared_field in declared {
            let Some((expression, field_span)) =
                initializers.get(&declared_field.name.text).copied()
            else {
                self.errors.at(
                    "ZRYNA-M3005",
                    at,
                    format!("field '{}' is not initialized", declared_field.name.text),
                    "initialize every declared field exactly once",
                );
                return None;
            };
            let value = self.value(expression)?;
            let expected = semantic_type(
                self.file,
                declared_field.type_syntax,
                self.module,
                self.declarations,
                self.graph,
                self.node_types,
                self.errors,
            )?;
            self.require_type(expected, value.0, field_span, "struct field")?;
            values.push(value.1);
        }
        let id = self.emit(
            Some(ty),
            at,
            raw::InstructionKind::StructConstruct { fields: values, cleanup: None },
        )?;
        Some((ty, id))
    }
    fn enum_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        variant_name: &syntax::RawIdentifierSyntax,
        payload: Option<u32>,
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let ty = self.decl_ty(&name.text).or_else(|| {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                format!("'{}' is not a module-local enum type", name.text),
                "construct one exact declared enum variant",
            );
            None
        })?;
        let decl =
            self.declarations.iter().find(|d| d.module == self.module && d.name == name.text)?;
        let RawDataDeclarationKind::Enum { variants, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3005",
                at,
                "enum construction names a struct",
                "construct a declared enum variant",
            );
            return None;
        };
        let (ordinal, variant) =
            variants.iter().enumerate().find(|(_, v)| v.name.text == variant_name.text).or_else(
                || {
                    self.errors.at(
                        "ZRYNA-M3005",
                        span(self.input.sources(), variant_name.span),
                        format!("enum '{}' has no variant '{}'", name.text, variant_name.text),
                        "use one exact declared variant",
                    );
                    None
                },
            )?;
        let payload_value = match (variant.payload_type, payload) {
            (None, None) => None,
            (Some(expected), Some(value)) => {
                let payload_span = usize::try_from(value)
                    .ok()
                    .and_then(|index| self.function.body.expressions.get(index))
                    .map_or(at, |expression| span(self.input.sources(), expression.span));
                let expected = semantic_type(
                    self.file,
                    expected,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?;
                let value = self.value(value)?;
                self.require_type(expected, value.0, payload_span, "enum payload")?;
                Some(value.1)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3005",
                    at,
                    "enum payload presence does not match the declared variant",
                    "supply exactly one payload only for a payload variant",
                );
                return None;
            }
        };
        let id = self.emit(
            Some(ty),
            at,
            raw::InstructionKind::EnumConstruct {
                variant: u32::try_from(ordinal).ok()?,
                payload: payload_value,
                cleanup: None,
            },
        )?;
        Some((ty, id))
    }
    fn array_value(
        &mut self,
        type_syntax: u32,
        elements: &[u32],
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let ty = semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        let record = self.layouts.type_by_id(ty.layout)?;
        let length = record.array_length()?;
        if u64::try_from(elements.len()).ok()? != length {
            self.errors.at(
                "ZRYNA-M3005",
                at,
                format!(
                    "fixed-array constructor has {} elements but its type requires {length}",
                    elements.len()
                ),
                "provide exactly the fixed-array length",
            );
            return None;
        }
        let element = record.referenced_type()?;
        let element_ty =
            self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        let mut values = Vec::with_capacity(elements.len());
        for expression in elements {
            let element_span = usize::try_from(*expression)
                .ok()
                .and_then(|index| self.function.body.expressions.get(index))
                .map_or(at, |value| span(self.input.sources(), value.span));
            let value = self.value(*expression)?;
            self.require_type(element_ty, value.0, element_span, "fixed-array element")?;
            values.push(value.1);
        }
        let id = self.emit(
            Some(ty),
            at,
            raw::InstructionKind::FixedArrayConstruct { elements: values, cleanup: None },
        )?;
        Some((ty, id))
    }
    fn unary_i32(&mut self, operand: u32, at: Span) -> Option<(Ty, raw::ValueId)> {
        let expected = self.primitive(TypeCategory::I32)?;
        let value = self.value(operand)?;
        self.require_type(expected, value.0, at, "negation operand")?;
        let id =
            self.emit(Some(expected), at, raw::InstructionKind::I32Neg { operand: value.1 })?;
        Some((expected, id))
    }
    fn binary_i32(
        &mut self,
        lhs: u32,
        rhs: u32,
        at: Span,
        make: impl FnOnce(raw::ValueId, raw::ValueId) -> raw::InstructionKind,
    ) -> Option<(Ty, raw::ValueId)> {
        let expected = self.primitive(TypeCategory::I32)?;
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        self.require_type(expected, lhs.0, at, "left operand")?;
        self.require_type(expected, rhs.0, at, "right operand")?;
        let id = self.emit(Some(expected), at, make(lhs.1, rhs.1))?;
        Some((expected, id))
    }
    fn compare(&mut self, lhs: u32, rhs: u32, at: Span, not: bool) -> Option<(Ty, raw::ValueId)> {
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        self.require_type(lhs.0, rhs.0, at, "comparison")?;
        if !matches!(lhs.0.category, TypeCategory::Bool | TypeCategory::I32) {
            self.errors.at(
                "ZRYNA-M3008",
                at,
                "equality is scalar-only in aggregate M3",
                "compare bool or i32 projections rather than whole aggregates",
            );
            return None;
        }
        let result = self.primitive(TypeCategory::Bool)?;
        let kind = if not {
            raw::InstructionKind::Ne { lhs: lhs.1, rhs: rhs.1 }
        } else {
            raw::InstructionKind::Eq { lhs: lhs.1, rhs: rhs.1 }
        };
        let id = self.emit(Some(result), at, kind)?;
        Some((result, id))
    }
    fn rel(&mut self, lhs: u32, rhs: u32, at: Span, op: u8) -> Option<(Ty, raw::ValueId)> {
        let i32_ty = self.primitive(TypeCategory::I32)?;
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        self.require_type(i32_ty, lhs.0, at, "relational operand")?;
        self.require_type(i32_ty, rhs.0, at, "relational operand")?;
        let kind = match op {
            0 => raw::InstructionKind::I32LtS { lhs: lhs.1, rhs: rhs.1 },
            1 => raw::InstructionKind::I32LeS { lhs: lhs.1, rhs: rhs.1 },
            2 => raw::InstructionKind::I32GtS { lhs: lhs.1, rhs: rhs.1 },
            _ => raw::InstructionKind::I32GeS { lhs: lhs.1, rhs: rhs.1 },
        };
        let result = self.primitive(TypeCategory::Bool)?;
        let id = self.emit(Some(result), at, kind)?;
        Some((result, id))
    }
}

#[allow(dead_code)]
fn authenticated_type_capabilities(
    input: SemanticInput<'_>,
    module: usize,
    type_syntax: u32,
) -> Result<Ty, Vec<Diagnostic>> {
    let mut errors = Errors::new(input.sources());
    semantic_preflight(input, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let (graph, declarations) = build_graph(input, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let linear = layout::verify(&graph, input.sources(), StorageTarget::Linear32V1)?;
    let linux = layout::verify(&graph, input.sources(), StorageTarget::LinuxX8664V1)?;
    if linear.universe_identity() != linux.universe_identity() {
        errors.global(
            "ZRYNA-M3004",
            "the independently derived layout universes disagree",
            "reduce the owned type graph and report this deterministic compiler failure",
        );
        return Err(errors.finish());
    }
    let node_types = map_node_types(&graph, &linear, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }
    let Some(file) = input.syntax().files().get(module) else {
        errors.global(
            "ZRYNA-M3002",
            "the requested type module is outside the authenticated project",
            "resolve types only inside one authenticated source module",
        );
        return Err(errors.finish());
    };
    semantic_type(file, type_syntax, module, &declarations, &graph, &node_types, &mut errors)
        .ok_or_else(|| errors.finish())
}

#[cfg(test)]
mod tests;
