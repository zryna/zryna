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
mod borrow_call_preflight;
mod borrow_call_resources;
mod borrow_forwarding;
mod diagnostics;
mod function_catalog;
mod global_resource_limits;
mod layout_graph;
mod owned_cfg_state;
mod owned_control_flow_resources;
mod owned_control_flow_shape;
mod owned_lowering_resources;
mod owned_root_borrow_planning;
mod owned_root_borrow_postprocessing;
mod owned_string_lowering;
mod owned_vec_lowering;
mod owner_state;
mod root_borrow_arm_planning;
mod root_borrow_call_planning;
mod root_borrow_execution;
mod root_borrow_function_lowering;
mod root_borrow_shape_planning;
mod root_borrow_straight_planning;
mod root_borrow_value_planning;
mod string_vec_resource_estimates;
mod type_model;

use aggregate_resource_formulas::{
    PartialTransferBudgetViolation, aggregate_clone_budget_violation,
    partial_assignment_budget_preflight, partial_return_budget_preflight,
    partial_transfer_budget_preflight, projected_aggregate_assignment_budget_violation,
    projected_aggregate_clone_assignment_budget_violation,
    projected_aggregate_clone_budget_violation, projected_string_clone_budget_violation,
    projected_subobject_assignment_budget_violation, projected_subobject_move_budget_violation,
    projected_subobject_return_budget_violation,
};
use borrow_call_resources::preflight_program_borrow_calls;
use diagnostics::Errors;
use function_catalog::{
    FunctionCatalog, FunctionParameterOrder, FunctionSignature, build_function_catalog,
};
#[cfg(test)]
use global_resource_limits::checked_string_concat_bytes;
use global_resource_limits::{
    accumulate_generated_cfg_function, accumulate_generated_value_function,
    aggregate_operand_budget_violation, aggregate_transition_budget_violation, semantic_preflight,
};
use layout_graph::{Decl, build_graph, semantic_type};
use owned_cfg_state::OwnedCfgState;
#[cfg(test)]
use owned_control_flow_resources::preflight_owned_place_capacity_with_reserved;
use owned_control_flow_resources::{
    enum_payload_move_resource_violation, preflight_owned_place_capacity,
};
use owned_control_flow_shape::{
    is_terminal_owned_phi_candidate, root_is_terminal_if, terminal_owned_if,
};
#[cfg(test)]
use owned_control_flow_shape::{preflight_owned_loop_body, preflight_owned_loop_exit};
#[cfg(test)]
use owned_lowering_resources::{OwnedCleanupAccounting, OwnedCleanupActionContext};
use owned_lowering_resources::{
    push_aggregate_clone_prefix_cleanup, push_aggregate_reverse_cleanup,
};
use owned_root_borrow_planning::{
    is_direct_owned_root_borrow_candidate, plan_private_owned_root_borrow_syntax,
};
use owned_root_borrow_postprocessing::postprocess_private_owned_root_borrow_function;
use owned_string_lowering::lower_private_string_function;
use owner_state::{OwnedVecBranchState, OwnerState};
use root_borrow_function_lowering::lower_private_root_borrow_function;
#[cfg(test)]
use string_vec_resource_estimates::{
    OwnedStringEstimateContext, OwnedStringPreparationEstimate, cleanup_actions_after_additions,
    cleanup_actions_after_preparation, cleanup_actions_after_transfer,
    estimate_owned_string_expression, vec_push_target_invalid,
};
use type_model::{
    Binding, OwnedAggregatePlace, OwnedAggregatePlacePreflight, OwnedProjectionShapeEntry,
    OwnedStaticProjectionKind, ProjectedAggregateAssignmentSource, ProjectedAggregateMoveContext,
    Ty, map_node_types,
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
    preflight_program_borrow_calls(input, &catalog, &mut errors);
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

#[allow(clippy::too_many_arguments)]
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
    let lowered = lower_function_impl(
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
    postprocess_private_owned_root_borrow_function(
        span(input.sources(), function.span),
        &plan,
        lowered,
        errors,
    )
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
            catalog,
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
        push_aggregate_reverse_cleanup(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &self.owners,
            at,
            excluded,
            self.errors,
        )
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
        push_aggregate_clone_prefix_cleanup(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &self.owners,
            result_owner,
            at,
        )
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
    let mut lowerer = owned_vec_lowering::PrivateVecLowerer {
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
                lowerer.lower_root_local(statement, saw_if || saw_loop)?;
            }
            RawStatementKind::Return { value, .. } => {
                returned =
                    Some((lowerer.value(*value, result)?, span(input.sources(), statement.span)));
            }
            RawStatementKind::Assignment { target, value, .. } => {
                lowerer.lower_root_assignment(statement, *target, *value, saw_if || saw_loop)?;
            }
            RawStatementKind::ExpressionStatement { expression, .. } => {
                lowerer.lower_root_push_effect(statement, *expression, saw_if || saw_loop)?;
            }
            RawStatementKind::If { .. } => {
                lowerer.lower_root_if(statement, &mut saw_if, saw_loop)?;
            }
            RawStatementKind::While { .. } => {
                lowerer.lower_root_while(*statement_id, statement, saw_if, &mut saw_loop)?;
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
        borrows: Vec<Option<raw::BorrowId>>,
    ) -> Option<Vec<raw::CallArgument>> {
        let mut values = vec![None; signature.parameters.len()];
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
                FunctionParameterOrder::Borrow(_) => {}
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
        let signature = self.resolve_copy_call(callee, arguments, at)?;
        let borrows = self.preflight_copy_borrow_call(&signature, arguments, at)?;
        let snapshot = self.mutation_snapshot();
        let expected_after_rollback = snapshot.clone();
        let Some(lowered) = self.lower_direct_call_arguments(&signature, arguments, borrows) else {
            self.restore_mutation_snapshot(snapshot);
            debug_assert_eq!(self.mutation_snapshot(), expected_after_rollback);
            return None;
        };
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
