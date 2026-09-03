//! Aggregate semantic lowering for the isolated `DataOwnershipV1` profile.
//!
//! This boundary accepts only authenticated protocol-v4 syntax, derives both layout authorities
//! itself, and returns only verifier-sealed IR. Raw layout and IR claims never cross the API.

use std::collections::BTreeMap;

use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::data_ownership_v1::{self as ir, RuntimeContractIdentity, raw};
use zryna_layout::{self as layout, StorageTarget, TypeCategory, raw as raw_layout};
use zryna_ownership_runtime_abi as ownership_runtime_abi;
use zryna_source::{FileId, MAX_SOURCE_FILES, SourceMap, Span, UntrustedSpan};
use zryna_syntax::v4::{
    self as syntax, RawDataDeclarationKind, RawExpressionKind, RawStatementKind, RawTypeSyntaxKind,
};

mod aggregate_resource_formulas;
mod borrow_call_preflight;
mod borrow_call_resources;
mod borrow_forwarding;
mod copy_enum_match;
mod copy_lowering;
mod diagnostics;
mod function_catalog;
mod global_resource_limits;
mod layout_graph;
mod owned_aggregate_lowering;
mod owned_cfg_state;
mod owned_control_flow_resources;
mod owned_control_flow_shape;
mod owned_enum_payload_move;
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

#[cfg(test)]
use aggregate_resource_formulas::{
    PartialTransferBudgetViolation, aggregate_clone_budget_violation,
    partial_assignment_budget_preflight, partial_return_budget_preflight,
    partial_transfer_budget_preflight, projected_aggregate_clone_budget_violation,
    projected_string_clone_budget_violation,
};
#[cfg(test)]
use aggregate_resource_formulas::{
    projected_aggregate_assignment_budget_violation,
    projected_aggregate_clone_assignment_budget_violation,
    projected_subobject_assignment_budget_violation,
};
#[cfg(test)]
use aggregate_resource_formulas::{
    projected_subobject_move_budget_violation, projected_subobject_return_budget_violation,
};
use borrow_call_resources::preflight_program_borrow_calls;
use copy_enum_match::lower_enum_match_function;
use copy_lowering::{BorrowBinding, FunctionLowerer};
use diagnostics::Errors;
#[cfg(test)]
use function_catalog::FunctionSignature;
use function_catalog::{FunctionCatalog, FunctionParameterOrder, build_function_catalog};
#[cfg(test)]
use global_resource_limits::checked_string_concat_bytes;
use global_resource_limits::{
    accumulate_generated_cfg_function, accumulate_generated_value_function, semantic_preflight,
};
#[cfg(test)]
use global_resource_limits::{
    aggregate_operand_budget_violation, aggregate_transition_budget_violation,
};
use layout_graph::{Decl, build_graph, semantic_type};
use owned_aggregate_lowering::{
    is_private_owned_aggregate_candidate, lower_private_owned_aggregate_function,
};
#[cfg(test)]
use owned_control_flow_resources::enum_payload_move_resource_violation;
#[cfg(test)]
use owned_control_flow_resources::{
    preflight_owned_place_capacity, preflight_owned_place_capacity_with_reserved,
};
use owned_control_flow_shape::is_terminal_owned_phi_candidate;
#[cfg(test)]
use owned_control_flow_shape::{preflight_owned_loop_body, preflight_owned_loop_exit};
use owned_enum_payload_move::{
    is_private_owned_enum_payload_move_candidate, lower_private_owned_enum_payload_move_function,
};
#[cfg(test)]
use owned_lowering_resources::{OwnedCleanupAccounting, OwnedCleanupActionContext};
use owned_root_borrow_planning::{
    is_direct_owned_root_borrow_candidate, plan_private_owned_root_borrow_syntax,
};
use owned_root_borrow_postprocessing::postprocess_private_owned_root_borrow_function;
use owned_string_lowering::lower_private_string_function;
use owned_vec_lowering::lower_private_vec_function;
use owner_state::OwnerState;
use root_borrow_function_lowering::lower_private_root_borrow_function;
#[cfg(test)]
use string_vec_resource_estimates::{
    OwnedStringEstimateContext, OwnedStringPreparationEstimate, cleanup_actions_after_additions,
    cleanup_actions_after_preparation, cleanup_actions_after_transfer,
    estimate_owned_string_expression, vec_push_target_invalid,
};
use type_model::{
    Binding, OwnedProjectionShapeEntry, OwnedStaticProjectionKind, Ty, map_node_types,
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
    if is_private_owned_aggregate_candidate(function, result) {
        return lower_private_owned_aggregate_function(
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
