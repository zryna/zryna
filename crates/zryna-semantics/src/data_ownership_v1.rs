//! Aggregate semantic lowering for the isolated `DataOwnershipV1` profile.
//!
//! This boundary accepts only authenticated protocol-v4 syntax, derives both layout authorities
//! itself, and returns only verifier-sealed IR. Raw layout and IR claims never cross the API.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::data_ownership_v1::{self as ir, RuntimeContractIdentity, raw};
use zryna_layout::{self as layout, StorageTarget, TypeCategory, raw as raw_layout};
use zryna_ownership_runtime_abi as ownership_runtime_abi;
use zryna_source::{FileId, MAX_SOURCE_FILES, SourceMap, Span, UntrustedSpan};
use zryna_syntax::v4::{
    self as syntax, RawDataDeclarationKind, RawExpressionKind, RawFieldInitializerKind,
    RawStatementKind, RawTypeSyntaxKind,
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

#[derive(Clone)]
struct Decl {
    module: usize,
    declaration: usize,
    name: String,
    node: raw_layout::NodeId,
    span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Ty {
    layout: layout::TypeId,
    ir: raw::TypeId,
    category: TypeCategory,
    drop_kind: u32,
    runtime_kind: u32,
    cloneable: bool,
}

#[derive(Clone)]
struct FunctionSignature {
    id: raw::FunctionId,
    name: String,
    parameters: Vec<Ty>,
    result: Ty,
    private: bool,
}

struct FunctionCatalog {
    modules: Vec<Vec<Option<FunctionSignature>>>,
}

enum FunctionResolution<'a> {
    Exact(&'a FunctionSignature),
    WrongCase,
    Missing,
}

impl FunctionCatalog {
    fn resolve(&self, module: usize, name: &str) -> FunctionResolution<'_> {
        let Some(signatures) = self.modules.get(module) else {
            return FunctionResolution::Missing;
        };
        if let Some(signature) =
            signatures.iter().flatten().find(|signature| signature.name == name)
        {
            return FunctionResolution::Exact(signature);
        }
        if signatures.iter().flatten().any(|signature| signature.name.eq_ignore_ascii_case(name)) {
            FunctionResolution::WrongCase
        } else {
            FunctionResolution::Missing
        }
    }
}

impl Ty {
    const fn is_copy(self) -> bool {
        self.drop_kind == 0
    }

    const fn is_clone(self) -> bool {
        self.cloneable
    }
}

#[derive(Default)]
struct TypeInterners {
    arrays: BTreeMap<(u32, u64), raw_layout::NodeId>,
    vectors: BTreeMap<u32, raw_layout::NodeId>,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProgramCfgBudgetLimit {
    Blocks,
    Edges,
}

fn raw_function_value_count(function: &raw::Function) -> Option<usize> {
    function.blocks.iter().try_fold(function.parameters.len(), |total, block| {
        let total = total.checked_add(block.parameters.len())?;
        total.checked_add(
            block.instructions.iter().filter(|instruction| instruction.result.is_some()).count(),
        )
    })
}

fn accumulate_generated_value_function(
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

fn generated_cfg_budget_violation(
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

fn raw_terminator_edge_count(terminator: &raw::Terminator) -> usize {
    match terminator {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
        raw::Terminator::Jump(_) => 1,
        raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
        raw::Terminator::EnumMatch { arms, .. } => arms.len(),
    }
}

fn accumulate_generated_cfg_function(
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

fn semantic_preflight(input: SemanticInput<'_>, errors: &mut Errors<'_>) {
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

fn build_function_catalog(
    input: SemanticInput<'_>,
    declarations: &[Decl],
    graph: &raw_layout::Graph,
    node_types: &[Option<Ty>],
    errors: &mut Errors<'_>,
) -> FunctionCatalog {
    let mut modules = Vec::with_capacity(input.syntax().files().len());
    for (module, file) in input.syntax().files().iter().enumerate() {
        let mut names = BTreeMap::<String, Span>::new();
        let mut signatures = Vec::with_capacity(file.functions().len());
        for (declaration, function) in file.functions().iter().enumerate() {
            let name_span = span(input.sources(), function.name.span);
            let folded = function.name.text.to_ascii_lowercase();
            if function.name.text.eq_ignore_ascii_case("concat") {
                errors.at(
                    "ZRYNA-M3002",
                    name_span,
                    "function name 'concat' collides with the sealed String builtin",
                    "rename the function so ordinary calls remain unambiguous",
                );
            }
            if names.insert(folded, name_span).is_some() {
                errors.at(
                    "ZRYNA-M3002",
                    name_span,
                    format!(
                        "function '{}' collides under portable ASCII case folding",
                        function.name.text
                    ),
                    "give every module-local function one portable case-insensitive unique name",
                );
            }
            let parameters = function
                .parameters
                .iter()
                .map(|parameter| {
                    semantic_type(
                        file,
                        parameter.type_syntax,
                        module,
                        declarations,
                        graph,
                        node_types,
                        errors,
                    )
                })
                .collect::<Option<Vec<_>>>();
            let result = semantic_type(
                file,
                function.result_type,
                module,
                declarations,
                graph,
                node_types,
                errors,
            );
            signatures.push(parameters.zip(result).map(|(parameters, result)| FunctionSignature {
                id: raw::FunctionId {
                    module: raw::ModuleId(u32::try_from(module).unwrap_or(u32::MAX)),
                    declaration: u32::try_from(declaration).unwrap_or(u32::MAX),
                },
                name: function.name.text.clone(),
                parameters,
                result,
                private: function.export_span.is_none(),
            }));
        }
        modules.push(signatures);
    }
    FunctionCatalog { modules }
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

fn string_byte_budget_violation(program_bytes: usize, literal_bytes: usize) -> bool {
    program_bytes
        .checked_add(literal_bytes)
        .is_none_or(|total| total > ir::MAX_STRING_LITERAL_BYTES)
}

fn checked_string_concat_bytes(left: u64, right: u64) -> Option<u64> {
    left.checked_add(right).filter(|total| *total <= ownership_runtime_abi::MAX_STRING_BYTES)
}

fn aggregate_operand_budget_violation(current: usize, additional: usize) -> bool {
    current.checked_add(additional).is_none_or(|total| total > ir::MAX_AGGREGATE_OPERANDS)
}

fn preflight_aggregate_operand_total(
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

fn resource_budget_violation(current: usize, extra: usize, maximum: usize) -> bool {
    current.checked_add(extra).is_none_or(|total| total > maximum)
}

fn aggregate_transition_budget_violation(
    current: usize,
    reserved: usize,
    additional: usize,
) -> bool {
    reserved.checked_add(additional).is_none_or(|extra| {
        resource_budget_violation(current, extra, ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
    })
}

fn aggregate_clone_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
) -> bool {
    let Some(prefix_actions) = pending.checked_add(1) else { return true };
    let Some(new_actions) = pending.checked_add(prefix_actions) else { return true };
    resource_budget_violation(values, 1, ir::MAX_VALUES_PER_FUNCTION)
        || resource_budget_violation(places, 1, ir::MAX_PLACES_PER_FUNCTION)
        || resource_budget_violation(transitions, 1, ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
        || resource_budget_violation(cleanup_plans, 2, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || resource_budget_violation(
            cleanup_actions,
            new_actions,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        )
}

fn cleanup_action_budget_violation(current: usize, pending: usize, excluded_present: bool) -> bool {
    let actions = pending.saturating_sub(usize::from(excluded_present));
    resource_budget_violation(current, actions, ir::MAX_DROP_ACTIONS_PER_FUNCTION)
}

#[cfg(test)]
fn owned_call_cleanup_budget_violation(
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
    transferred: usize,
) -> bool {
    cleanup_plans >= ir::MAX_CLEANUP_PLANS_PER_FUNCTION
        || pending.checked_sub(transferred).is_none_or(|survivors| {
            resource_budget_violation(cleanup_actions, survivors, ir::MAX_DROP_ACTIONS_PER_FUNCTION)
        })
}

const fn vec_push_target_invalid(mutable: bool, available: bool) -> bool {
    !mutable || !available
}

const fn cleanup_actions_after_preparation(pending: usize, creates_owner: bool) -> usize {
    pending.saturating_add(creates_owner as usize)
}

const fn cleanup_actions_after_transfer(pending: usize, transfers_existing: bool) -> usize {
    pending.saturating_sub(transfers_existing as usize)
}

const fn cleanup_actions_after_additions(pending: usize, additional_owners: usize) -> usize {
    pending.saturating_add(additional_owners)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OwnedStringPreparationEstimate {
    end_pending: usize,
    peak_pending: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    values: usize,
    places: usize,
    transitions: usize,
    transfers_existing: bool,
    root_cleanup_actions: Option<usize>,
}

#[derive(Clone, Copy)]
enum OwnedStringEstimateContext {
    Value,
    Read,
}

#[derive(Clone, Copy, Debug)]
enum OwnedStringEstimateError {
    Unsupported,
    Unavailable(UntrustedSpan),
    Overflow,
}

enum OwnedStringEstimateOutcome {
    Estimated(OwnedStringPreparationEstimate),
    Unsupported,
}

fn add_estimate_counts(
    left: OwnedStringPreparationEstimate,
    right: OwnedStringPreparationEstimate,
) -> Result<OwnedStringPreparationEstimate, OwnedStringEstimateError> {
    Ok(OwnedStringPreparationEstimate {
        end_pending: right.end_pending,
        peak_pending: left.peak_pending.max(right.peak_pending),
        cleanup_plans: left
            .cleanup_plans
            .checked_add(right.cleanup_plans)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        cleanup_actions: left
            .cleanup_actions
            .checked_add(right.cleanup_actions)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        values: left.values.checked_add(right.values).ok_or(OwnedStringEstimateError::Overflow)?,
        places: left.places.checked_add(right.places).ok_or(OwnedStringEstimateError::Overflow)?,
        transitions: left
            .transitions
            .checked_add(right.transitions)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        transfers_existing: left.transfers_existing || right.transfers_existing,
        root_cleanup_actions: right.root_cleanup_actions,
    })
}

fn estimate_owned_string_leaf(
    expression: &syntax::RawExpressionSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    string_ty: Ty,
    pending: usize,
    context: OwnedStringEstimateContext,
) -> Option<Result<OwnedStringPreparationEstimate, OwnedStringEstimateError>> {
    let empty = || OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    };
    match &expression.kind {
        RawExpressionKind::Reference { name } => {
            let binding = bindings.get(&name.text).filter(|binding| binding.ty == string_ty)?;
            if !owners.contains(binding.place) {
                return Some(Err(OwnedStringEstimateError::Unavailable(name.span)));
            }
            if matches!(context, OwnedStringEstimateContext::Read) {
                return Some(Ok(empty()));
            }
            Some(
                pending
                    .checked_add(1)
                    .map(|peak_pending| OwnedStringPreparationEstimate {
                        transfers_existing: true,
                        values: 1,
                        places: 1,
                        transitions: 1,
                        peak_pending,
                        ..empty()
                    })
                    .ok_or(OwnedStringEstimateError::Overflow),
            )
        }
        RawExpressionKind::StringLiteral { .. } => Some(
            pending
                .checked_add(1)
                .map(|end| OwnedStringPreparationEstimate {
                    end_pending: end,
                    peak_pending: end,
                    cleanup_plans: 1,
                    cleanup_actions: pending,
                    values: 1,
                    places: 1,
                    transitions: 1,
                    transfers_existing: false,
                    root_cleanup_actions: Some(pending),
                })
                .ok_or(OwnedStringEstimateError::Overflow),
        ),
        _ => None,
    }
}

fn estimate_owned_string_expression(
    function: &syntax::RawFunctionSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    string_ty: Ty,
    id: u32,
    pending: usize,
    context: OwnedStringEstimateContext,
) -> Result<OwnedStringPreparationEstimate, OwnedStringEstimateError> {
    let expression = usize::try_from(id)
        .ok()
        .and_then(|index| function.body.expressions.get(index))
        .ok_or(OwnedStringEstimateError::Unsupported)?;
    if let Some(estimate) =
        estimate_owned_string_leaf(expression, bindings, owners, string_ty, pending, context)
    {
        return estimate;
    }
    let empty = || OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    };
    match &expression.kind {
        RawExpressionKind::Clone { value, .. } => {
            let child = estimate_owned_string_expression(
                function,
                bindings,
                owners,
                string_ty,
                *value,
                pending,
                OwnedStringEstimateContext::Read,
            )?;
            let end = child.end_pending.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?;
            Ok(OwnedStringPreparationEstimate {
                end_pending: end,
                peak_pending: child.peak_pending.max(end),
                cleanup_plans: child
                    .cleanup_plans
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                cleanup_actions: child
                    .cleanup_actions
                    .checked_add(child.end_pending)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                values: child.values.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                places: child.places.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                transitions: child
                    .transitions
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                transfers_existing: child.transfers_existing,
                root_cleanup_actions: Some(child.end_pending),
            })
        }
        RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
            let mut total = empty();
            for argument in arguments {
                let child = estimate_owned_string_expression(
                    function,
                    bindings,
                    owners,
                    string_ty,
                    *argument,
                    total.end_pending,
                    OwnedStringEstimateContext::Read,
                )?;
                total = add_estimate_counts(total, child)?;
            }
            let cleanup_at_root = total.end_pending;
            let end = cleanup_at_root.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?;
            Ok(OwnedStringPreparationEstimate {
                end_pending: end,
                peak_pending: total.peak_pending.max(end),
                cleanup_plans: total
                    .cleanup_plans
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                cleanup_actions: total
                    .cleanup_actions
                    .checked_add(cleanup_at_root)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                values: total.values.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                places: total.places.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                transitions: total
                    .transitions
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                transfers_existing: total.transfers_existing,
                root_cleanup_actions: Some(cleanup_at_root),
            })
        }
        RawExpressionKind::Call { arguments, .. } => estimate_owned_string_call_arguments(
            function, bindings, owners, string_ty, arguments, pending,
        ),
        _ => Err(OwnedStringEstimateError::Unsupported),
    }
}

fn estimate_owned_string_call_arguments(
    function: &syntax::RawFunctionSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    string_ty: Ty,
    arguments: &[u32],
    pending: usize,
) -> Result<OwnedStringPreparationEstimate, OwnedStringEstimateError> {
    let mut total = OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    };
    for argument in arguments {
        let child = estimate_owned_string_expression(
            function,
            bindings,
            owners,
            string_ty,
            *argument,
            total.end_pending,
            OwnedStringEstimateContext::Value,
        )?;
        total = add_estimate_counts(total, child)?;
        total.end_pending =
            total.end_pending.checked_sub(1).ok_or(OwnedStringEstimateError::Overflow)?;
    }
    let cleanup_at_root = total.end_pending;
    let end = cleanup_at_root.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?;
    Ok(OwnedStringPreparationEstimate {
        end_pending: end,
        peak_pending: total.peak_pending.max(end),
        cleanup_plans: total
            .cleanup_plans
            .checked_add(1)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        cleanup_actions: total
            .cleanup_actions
            .checked_add(cleanup_at_root)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        values: total.values.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
        places: total.places.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
        transitions: total.transitions.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
        transfers_existing: total.transfers_existing,
        root_cleanup_actions: Some(cleanup_at_root),
    })
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

#[derive(Clone, Copy)]
struct VecPreparationEstimate {
    end_pending: usize,
    resources: OwnedStringPreparationEstimate,
}

fn empty_owned_string_estimate(pending: usize) -> OwnedStringPreparationEstimate {
    OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueBudgetLimit {
    Function,
    Program,
}

fn value_budget_violation(
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
fn derived_value_count(function: &syntax::RawFunctionSyntax) -> usize {
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

#[allow(clippy::too_many_lines)]
fn build_graph(
    input: SemanticInput<'_>,
    errors: &mut Errors<'_>,
) -> (raw_layout::Graph, Vec<Decl>) {
    let mut graph =
        raw_layout::Graph { modules: Vec::new(), types: Vec::new(), program_roots: Vec::new() };
    graph.types.push(raw_layout::TypeNode {
        id: raw_layout::NodeId(0),
        span: None,
        kind: raw_layout::TypeKind::Bool,
    });
    graph.types.push(raw_layout::TypeNode {
        id: raw_layout::NodeId(1),
        span: None,
        kind: raw_layout::TypeKind::I32,
    });
    graph.types.push(raw_layout::TypeNode {
        id: raw_layout::NodeId(2),
        span: None,
        kind: raw_layout::TypeKind::String,
    });
    let mut declarations = Vec::new();
    for (module, file) in input.syntax().files().iter().enumerate() {
        graph.modules.push(raw_layout::Module {
            id: raw_layout::ModuleId(u32::try_from(module).unwrap_or(u32::MAX)),
            source_file: file.id(),
            data_declarations: u32::try_from(file.data_declarations().len()).unwrap_or(u32::MAX),
        });
        let mut names = BTreeMap::new();
        for (declaration, value) in file.data_declarations().iter().enumerate() {
            let (name, name_span) = declaration_name(value, input.sources());
            if names.insert(name.to_ascii_lowercase(), name_span).is_some() {
                errors.at(
                    "ZRYNA-M3002",
                    name_span,
                    format!("aggregate type '{name}' is declared more than once"),
                    "give every aggregate declaration one exact module-local name",
                );
            }
            match &value.kind {
                RawDataDeclarationKind::Struct { fields, .. } => {
                    if fields.is_empty() {
                        errors.at(
                            "ZRYNA-M3004",
                            span(input.sources(), value.span),
                            "empty structs are outside aggregate M3",
                            "declare at least one Copy field",
                        );
                    }
                    verify_member_names(
                        fields.iter().map(|field| (&field.name.text, field.name.span)),
                        input.sources(),
                        "field",
                        errors,
                    );
                }
                RawDataDeclarationKind::Enum { variants, .. } => {
                    if variants.is_empty() {
                        errors.at(
                            "ZRYNA-M3004",
                            span(input.sources(), value.span),
                            "empty enums are outside aggregate M3",
                            "declare at least one enum variant",
                        );
                    }
                    verify_member_names(
                        variants.iter().map(|variant| (&variant.name.text, variant.name.span)),
                        input.sources(),
                        "variant",
                        errors,
                    );
                }
            }
            let node = raw_layout::NodeId(u32::try_from(graph.types.len()).unwrap_or(u32::MAX));
            graph.types.push(raw_layout::TypeNode {
                id: node,
                span: Some(span(input.sources(), value.span)),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(u32::try_from(module).unwrap_or(u32::MAX)),
                    declaration: u32::try_from(declaration).unwrap_or(u32::MAX),
                    fields: Vec::new(),
                },
            });
            declarations.push(Decl {
                module,
                declaration,
                name: name.to_owned(),
                node,
                span: span(input.sources(), value.span),
            });
        }
    }
    let mut interners = TypeInterners::default();
    for module in 0..input.syntax().files().len() {
        let file = &input.syntax().files()[module];
        for declaration in 0..file.data_declarations().len() {
            let decl = declarations
                .iter()
                .find(|d| d.module == module && d.declaration == declaration)
                .expect("preallocated declaration");
            let kind = match &file.data_declarations()[declaration].kind {
                RawDataDeclarationKind::Struct { fields, .. } => raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(u32::try_from(module).unwrap_or(u32::MAX)),
                    declaration: u32::try_from(declaration).unwrap_or(u32::MAX),
                    fields: fields
                        .iter()
                        .enumerate()
                        .filter_map(|(ordinal, field)| {
                            resolve_graph_type(
                                file,
                                field.type_syntax,
                                module,
                                &declarations,
                                &mut graph,
                                &mut interners,
                                errors,
                            )
                            .map(|ty| raw_layout::Field {
                                ordinal: u32::try_from(ordinal).unwrap_or(u32::MAX),
                                ty,
                            })
                        })
                        .collect(),
                },
                RawDataDeclarationKind::Enum { variants, .. } => raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(u32::try_from(module).unwrap_or(u32::MAX)),
                    declaration: u32::try_from(declaration).unwrap_or(u32::MAX),
                    variants: variants
                        .iter()
                        .enumerate()
                        .map(|(ordinal, variant)| raw_layout::Variant {
                            ordinal: u32::try_from(ordinal).unwrap_or(u32::MAX),
                            payload: variant.payload_type.and_then(|id| {
                                resolve_graph_type(
                                    file,
                                    id,
                                    module,
                                    &declarations,
                                    &mut graph,
                                    &mut interners,
                                    errors,
                                )
                            }),
                        })
                        .collect(),
                },
            };
            graph.types[usize::try_from(decl.node.0).expect("node index")].kind = kind;
        }
        for function in file.functions() {
            for parameter in &function.parameters {
                add_root(
                    file,
                    parameter.type_syntax,
                    module,
                    &declarations,
                    &mut graph,
                    &mut interners,
                    errors,
                );
            }
            add_root(
                file,
                function.result_type,
                module,
                &declarations,
                &mut graph,
                &mut interners,
                errors,
            );
            for statement in &function.body.statements {
                if let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind {
                    add_root(
                        file,
                        type_syntax,
                        module,
                        &declarations,
                        &mut graph,
                        &mut interners,
                        errors,
                    );
                }
            }
            for expression in &function.body.expressions {
                if let RawExpressionKind::FixedArrayConstruction { type_syntax, .. }
                | RawExpressionKind::VecConstruction { type_syntax, .. } = expression.kind
                {
                    add_root(
                        file,
                        type_syntax,
                        module,
                        &declarations,
                        &mut graph,
                        &mut interners,
                        errors,
                    );
                }
            }
        }
    }
    graph.program_roots.sort_by_key(|id| id.0);
    graph.program_roots.dedup();
    (graph, declarations)
}

fn add_root(
    file: &syntax::SourceUnit,
    id: u32,
    module: usize,
    declarations: &[Decl],
    graph: &mut raw_layout::Graph,
    interners: &mut TypeInterners,
    errors: &mut Errors<'_>,
) {
    if let Some(node) = resolve_graph_type(file, id, module, declarations, graph, interners, errors)
    {
        graph.program_roots.push(node);
    }
}

fn resolve_graph_type(
    file: &syntax::SourceUnit,
    id: u32,
    module: usize,
    declarations: &[Decl],
    graph: &mut raw_layout::Graph,
    interners: &mut TypeInterners,
    errors: &mut Errors<'_>,
) -> Option<raw_layout::NodeId> {
    let ty = usize::try_from(id).ok().and_then(|i| file.type_syntax().get(i))?;
    match &ty.kind {
        RawTypeSyntaxKind::Named { name } if name.text == "bool" => Some(raw_layout::NodeId(0)),
        RawTypeSyntaxKind::Named { name } if name.text == "i32" => Some(raw_layout::NodeId(1)),
        RawTypeSyntaxKind::String { .. } => Some(raw_layout::NodeId(2)),
        RawTypeSyntaxKind::Named { name } => declarations
            .iter()
            .find(|d| d.module == module && d.name == name.text)
            .map(|d| d.node)
            .or_else(|| {
                errors.at(
                    "ZRYNA-M3002",
                    span(errors.sources, name.span),
                    format!("type '{}' does not name a module-local aggregate", name.text),
                    "use bool, i32, or an exact aggregate declaration name",
                );
                None
            }),
        RawTypeSyntaxKind::FixedArray { element, length, .. } => {
            let element =
                resolve_graph_type(file, *element, module, declarations, graph, interners, errors)?;
            let length = u64::from(*length);
            if let Some(id) = interners.arrays.get(&(element.0, length)).copied() {
                return Some(id);
            }
            let id = raw_layout::NodeId(u32::try_from(graph.types.len()).ok()?);
            graph.types.push(raw_layout::TypeNode {
                id,
                span: None,
                kind: raw_layout::TypeKind::FixedArray { element, length },
            });
            interners.arrays.insert((element.0, length), id);
            Some(id)
        }
        RawTypeSyntaxKind::Vec { argument, .. } => {
            let element = resolve_graph_type(
                file,
                *argument,
                module,
                declarations,
                graph,
                interners,
                errors,
            )?;
            if let Some(id) = interners.vectors.get(&element.0).copied() {
                return Some(id);
            }
            let id = raw_layout::NodeId(u32::try_from(graph.types.len()).ok()?);
            graph.types.push(raw_layout::TypeNode {
                id,
                span: None,
                kind: raw_layout::TypeKind::Vec { element },
            });
            interners.vectors.insert(element.0, id);
            Some(id)
        }
        RawTypeSyntaxKind::Missing => {
            errors.at(
                "ZRYNA-M3002",
                span(errors.sources, ty.span),
                "an exact aggregate type annotation is required",
                "write bool, i32, a Copy aggregate name, or FixedArray<T, N>",
            );
            None
        }
        _ => {
            errors.at(
                "ZRYNA-M3003",
                span(errors.sources, ty.span),
                "heap, handle, and borrow types are outside aggregate M3",
                "use only Copy bool, i32, structs, enums, and fixed arrays",
            );
            None
        }
    }
}

fn map_node_types(
    graph: &raw_layout::Graph,
    layouts: &layout::VerifiedLayouts,
    errors: &mut Errors<'_>,
) -> Vec<Option<Ty>> {
    let mut result: Vec<Option<Ty>> = vec![None; graph.types.len()];
    for node in &graph.types {
        let found = match &node.kind {
            raw_layout::TypeKind::Bool => {
                layouts.types().find(|t| t.category() == TypeCategory::Bool)
            }
            raw_layout::TypeKind::I32 => {
                layouts.types().find(|t| t.category() == TypeCategory::I32)
            }
            raw_layout::TypeKind::String => {
                layouts.types().find(|t| t.category() == TypeCategory::String)
            }
            raw_layout::TypeKind::Struct { module, declaration, .. }
            | raw_layout::TypeKind::Enum { module, declaration, .. } => {
                layouts.types().find(|t| t.nominal_identity() == Some((module.0, *declaration)))
            }
            raw_layout::TypeKind::FixedArray { element, length } => {
                let element_index = usize::try_from(element.0).ok();
                let element_id =
                    element_index.and_then(|i| result.get(i)).and_then(|v| *v).map(|v| v.layout);
                layouts.types().find(|t| {
                    t.category() == TypeCategory::FixedArray
                        && t.array_length() == Some(*length)
                        && t.referenced_type() == element_id
                })
            }
            raw_layout::TypeKind::Vec { element } => {
                let element_index = usize::try_from(element.0).ok();
                let element_id =
                    element_index.and_then(|i| result.get(i)).and_then(|v| *v).map(|v| v.layout);
                layouts.types().find(|ty| {
                    ty.category() == TypeCategory::Vec && ty.referenced_type() == element_id
                })
            }
            _ => None,
        };
        if let Some(found) = found {
            let index = usize::try_from(node.id.0).expect("bounded node");
            result[index] = Some(Ty {
                layout: found.id(),
                ir: raw::TypeId(found.id().index()),
                category: found.category(),
                drop_kind: found.drop_kind(),
                runtime_kind: found.runtime_kind(),
                cloneable: false,
            });
        } else {
            errors.global(
                "ZRYNA-M3004",
                format!("derived layout type node #{} has no sealed identity", node.id.0),
                "reduce the aggregate graph and report this deterministic compiler failure",
            );
        }
    }
    let clone_capabilities = derive_clone_capabilities(graph);
    for (index, cloneable) in clone_capabilities.into_iter().enumerate() {
        if let Some(ty) = result[index].as_mut() {
            ty.cloneable = cloneable;
        }
    }
    result
}

fn derive_clone_capabilities(graph: &raw_layout::Graph) -> Vec<bool> {
    let mut capabilities = graph
        .types
        .iter()
        .map(|node| !matches!(node.kind, raw_layout::TypeKind::Borrow { .. }))
        .collect::<Vec<_>>();
    loop {
        let previous = capabilities.clone();
        for node in &graph.types {
            let child = |id: raw_layout::NodeId| {
                usize::try_from(id.0)
                    .ok()
                    .and_then(|index| previous.get(index))
                    .copied()
                    .unwrap_or(false)
            };
            capabilities[node.id.0 as usize] = match &node.kind {
                raw_layout::TypeKind::Bool
                | raw_layout::TypeKind::I32
                | raw_layout::TypeKind::String
                | raw_layout::TypeKind::Shared { .. }
                | raw_layout::TypeKind::Weak { .. } => true,
                raw_layout::TypeKind::Struct { fields, .. } => {
                    fields.iter().all(|field| child(field.ty))
                }
                raw_layout::TypeKind::Enum { variants, .. } => {
                    variants.iter().all(|variant| variant.payload.is_none_or(child))
                }
                raw_layout::TypeKind::FixedArray { element, .. }
                | raw_layout::TypeKind::Vec { element } => child(*element),
                raw_layout::TypeKind::Borrow { .. } => false,
            };
        }
        if capabilities == previous {
            return capabilities;
        }
    }
}

fn declaration_name<'a>(
    value: &'a syntax::RawDataDeclaration,
    sources: &SourceMap,
) -> (&'a str, Span) {
    let name = match &value.kind {
        RawDataDeclarationKind::Struct { name, .. } | RawDataDeclarationKind::Enum { name, .. } => {
            name
        }
    };
    (&name.text, span(sources, name.span))
}

fn verify_member_names<'a>(
    names: impl Iterator<Item = (&'a String, UntrustedSpan)>,
    sources: &SourceMap,
    label: &'static str,
    errors: &mut Errors<'_>,
) {
    let mut seen = BTreeMap::new();
    for (name, raw_span) in names {
        if seen.insert(name.to_ascii_lowercase(), raw_span).is_some() {
            errors.at(
                "ZRYNA-M3002",
                span(sources, raw_span),
                format!("{label} '{name}' collides under portable ASCII case folding"),
                "give every member a portable case-insensitive unique name",
            );
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding {
    ty: Ty,
    place: raw::PlaceId,
    mutable: bool,
}

struct FunctionLowerer<'a, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'a syntax::RawFunctionSyntax,
    module: usize,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    places: Vec<raw::Place>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    instructions: Vec<raw::Instruction>,
    cleanup_plans: Vec<raw::CleanupPlan>,
    values: u32,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
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
        projections: BTreeMap::new(),
        places: Vec::new(),
        instructions: Vec::new(),
        cleanup_plans: Vec::new(),
        values: 0,
    };
    let mut parameters = Vec::new();
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
        require_current_type_only_boundary(
            ty,
            span(input.sources(), parameter.span),
            function.export_span.is_some(),
            lowerer.errors,
        )?;
        let value = raw::ValueId(lowerer.values);
        lowerer.values += 1;
        parameters.push(raw::ValueDefinition {
            id: value,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
        });
        let place = lowerer.push_place(
            ty,
            span(input.sources(), parameter.span),
            raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        );
        if lowerer.bindings.keys().any(|name| name.eq_ignore_ascii_case(&parameter.name.text)) {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!("parameter '{}' is declared more than once", parameter.name.text),
                "give each parameter one exact name",
            );
        } else {
            lowerer
                .bindings
                .insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
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
                } else {
                    lowerer
                        .bindings
                        .insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
                }
            }
            RawStatementKind::Assignment { target, value, .. } => {
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
        borrow_parameters: Vec::new(),
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

#[derive(Clone, Copy)]
struct TerminalOwnedIf {
    condition: u32,
    then_value: u32,
    then_span: Span,
    else_value: u32,
    else_span: Span,
    span: Span,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnedCfgBudgetLimit {
    Blocks,
    Edges,
    Transitions,
    Values,
}

fn owned_cfg_budget_violation(
    blocks: usize,
    edges: usize,
    transitions: usize,
) -> Option<OwnedCfgBudgetLimit> {
    if blocks > ir::MAX_BLOCKS_PER_FUNCTION {
        Some(OwnedCfgBudgetLimit::Blocks)
    } else if edges > ir::MAX_CFG_EDGES_PER_FUNCTION {
        Some(OwnedCfgBudgetLimit::Edges)
    } else if transitions > ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION {
        Some(OwnedCfgBudgetLimit::Transitions)
    } else {
        None
    }
}

fn dense_owned_value_id(count: usize) -> Option<raw::ValueId> {
    u32::try_from(count).ok().map(raw::ValueId)
}

fn owned_value_budget_violation(current: usize, additional: usize) -> bool {
    resource_budget_violation(current, additional, ir::MAX_VALUES_PER_FUNCTION)
}

fn owned_place_budget_violation(current: usize, additional: usize) -> bool {
    resource_budget_violation(current, additional, ir::MAX_PLACES_PER_FUNCTION)
}

fn preflight_owned_place_capacity(
    current: usize,
    additional: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    if !owned_place_budget_violation(current, additional) {
        return true;
    }
    errors.at(
        "ZRYNA-M3201",
        at,
        format!(
            "derived places exceed the per-function M3 limit of {}",
            ir::MAX_PLACES_PER_FUNCTION
        ),
        "reduce owned parameters, expressions, and local declarations",
    );
    false
}

fn preflight_owned_place_capacity_with_reserved(
    current: usize,
    reserved: usize,
    additional: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    let Some(committed_and_reserved) = current.checked_add(reserved) else {
        return preflight_owned_place_capacity(usize::MAX, 1, at, errors);
    };
    preflight_owned_place_capacity(committed_and_reserved, additional, at, errors)
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

struct PrivateStringLowerer<'a, 'e> {
    input: SemanticInput<'a>,
    function: &'a syntax::RawFunctionSyntax,
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

impl PrivateStringLowerer<'_, '_> {
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
    function: &'a syntax::RawFunctionSyntax,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnerDelta {
    Registered { owner: raw::PlaceId },
    Renamed { from: raw::PlaceId, to: raw::PlaceId },
    Replaced { prepared: raw::PlaceId, target: raw::PlaceId },
    Transferred { owner: raw::PlaceId },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct OwnerState {
    pending: Vec<raw::PlaceId>,
    value_owners: BTreeMap<raw::ValueId, raw::PlaceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnedStringBranchState {
    bindings: BTreeMap<String, Binding>,
    owners: OwnerState,
    known_bytes: BTreeMap<raw::PlaceId, Option<u64>>,
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

impl OwnerState {
    fn pending(&self) -> &[raw::PlaceId] {
        &self.pending
    }

    fn contains(&self, owner: raw::PlaceId) -> bool {
        self.pending.contains(&owner)
    }

    fn owner(&self, value: raw::ValueId) -> Option<raw::PlaceId> {
        self.value_owners.get(&value).copied()
    }

    fn register(&mut self, value: raw::ValueId, owner: raw::PlaceId) -> Option<OwnerDelta> {
        if self.value_owners.contains_key(&value)
            || self.value_owners.values().any(|candidate| *candidate == owner)
            || self.pending.contains(&owner)
        {
            return None;
        }
        self.pending.push(owner);
        self.value_owners.insert(value, owner);
        Some(OwnerDelta::Registered { owner })
    }

    fn register_parameter(&mut self, owner: raw::PlaceId) -> Option<OwnerDelta> {
        if self.pending.contains(&owner)
            || self.value_owners.values().any(|candidate| *candidate == owner)
        {
            return None;
        }
        self.pending.push(owner);
        Some(OwnerDelta::Registered { owner })
    }

    fn rehome_move_result(
        &mut self,
        value: raw::ValueId,
        from: raw::PlaceId,
    ) -> Option<OwnerDelta> {
        let to = self.owner(value)?;
        if from == to {
            return None;
        }
        let from_slot = self.pending.iter().position(|place| *place == from)?;
        let to_slot = self.pending.iter().position(|place| *place == to)?;
        self.pending.remove(to_slot);
        let from_slot = from_slot - usize::from(to_slot < from_slot);
        self.pending[from_slot] = to;
        Some(OwnerDelta::Renamed { from, to })
    }

    fn rename(&mut self, value: raw::ValueId, to: raw::PlaceId) -> Option<OwnerDelta> {
        let from = self.owner(value)?;
        if from == to || self.pending.contains(&to) {
            return None;
        }
        let slot = self.pending.iter().position(|place| *place == from)?;
        self.pending[slot] = to;
        self.value_owners.remove(&value);
        Some(OwnerDelta::Renamed { from, to })
    }

    fn replace(&mut self, value: raw::ValueId, target: raw::PlaceId) -> Option<OwnerDelta> {
        let prepared = self.owner(value)?;
        if prepared == target {
            return None;
        }
        let target_slot = self.pending.iter().position(|place| *place == target)?;
        let prepared_slot = self.pending.iter().position(|place| *place == prepared)?;
        self.pending[prepared_slot] = target;
        self.pending.remove(target_slot);
        self.value_owners.remove(&value);
        Some(OwnerDelta::Replaced { prepared, target })
    }

    fn transfer(&mut self, value: raw::ValueId) -> Option<OwnerDelta> {
        let owner = self.owner(value)?;
        let slot = self.pending.iter().position(|place| *place == owner)?;
        self.pending.remove(slot);
        self.value_owners.remove(&value);
        Some(OwnerDelta::Transferred { owner })
    }

    fn consume_owner(&mut self, owner: raw::PlaceId) -> Option<OwnerDelta> {
        let slot = self.pending.iter().position(|place| *place == owner)?;
        self.pending.remove(slot);
        self.value_owners.retain(|_, candidate| *candidate != owner);
        Some(OwnerDelta::Transferred { owner })
    }
}

fn apply_owner_delta<T>(known: &mut BTreeMap<raw::PlaceId, T>, delta: OwnerDelta) {
    match delta {
        OwnerDelta::Registered { .. } => {}
        OwnerDelta::Renamed { from, to } => {
            if let Some(bytes) = known.remove(&from) {
                known.insert(to, bytes);
            }
        }
        OwnerDelta::Replaced { prepared, target } => {
            known.remove(&target);
            if let Some(bytes) = known.remove(&prepared) {
                known.insert(target, bytes);
            }
        }
        OwnerDelta::Transferred { owner } => {
            known.remove(&owner);
        }
    }
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

struct PrivateOwnedAggregateLowerer<'a, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'a syntax::RawFunctionSyntax,
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
    reserved_transitions: usize,
    owners: OwnerState,
    next_value: u32,
    next_local: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OwnedAggregatePlace {
    ty: Ty,
    place: raw::PlaceId,
    root: raw::PlaceId,
    mutable: bool,
    is_root: bool,
}

impl PrivateOwnedAggregateLowerer<'_, '_> {
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

    fn projected_value(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
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
        if expected.category != TypeCategory::String {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "this checkpoint moves only owned String projections",
                "move a String field or constant String fixed-array element",
            );
            return None;
        }
        let value = self.emit(
            expected,
            at,
            raw::InstructionKind::MoveFromPlace { place: projection.place },
        )?;
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
                self.projected_value(id, expected)
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_private_owned_aggregate_function<'a>(
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
                let value = lowerer.value(*initializer, ty)?;
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
                    span: span(input.sources(), statement.span),
                    kind: raw::PlaceKind::Local(lowerer.next_local),
                });
                lowerer.next_local += 1;
                if !lowerer.emit_effect(
                    span(input.sources(), statement.span),
                    raw::InstructionKind::InitializePlace { place, value },
                ) {
                    return None;
                }
                if !ty.is_copy() && lowerer.owners.rename(value, place).is_none() {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        span(input.sources(), statement.span),
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
                returned =
                    Some((lowerer.value(*value, result)?, span(input.sources(), statement.span)));
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

struct PrivateVecLowerer<'a, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'a syntax::RawFunctionSyntax,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnedVecBranchState {
    bindings: BTreeMap<String, Binding>,
    owners: OwnerState,
    known_string_bytes: BTreeMap<raw::PlaceId, u64>,
}

impl PrivateVecLowerer<'_, '_> {
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
        let exact_parameters =
            signature.parameters.is_empty() || signature.parameters.as_slice() == [self.vec_ty];
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
    function: &'a syntax::RawFunctionSyntax,
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
    function: &'a syntax::RawFunctionSyntax,
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

impl FunctionLowerer<'_, '_> {
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
            RawExpressionKind::Reference { .. }
            | RawExpressionKind::FieldAccess { .. }
            | RawExpressionKind::Index { .. } => {
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
        if arguments.len() != signature.parameters.len() {
            self.errors.at(
                "ZRYNA-M3008",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameters.len()
                ),
                "pass one argument for every exact declared parameter",
            );
            return None;
        }
        let mut lowered = Vec::with_capacity(arguments.len());
        for (argument, expected) in arguments.iter().zip(&signature.parameters) {
            let (actual, value) = self.value(*argument)?;
            self.require_type(
                *expected,
                actual,
                span(self.input.sources(), self.function.body.expressions[*argument as usize].span),
                "call argument",
            )?;
            lowered.push(raw::CallArgument::Value(value));
        }
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

fn semantic_type(
    file: &syntax::SourceUnit,
    id: u32,
    module: usize,
    declarations: &[Decl],
    graph: &raw_layout::Graph,
    node_types: &[Option<Ty>],
    errors: &mut Errors<'_>,
) -> Option<Ty> {
    let mut scratch = graph.clone();
    let mut interners = TypeInterners::default();
    for node in &graph.types {
        match node.kind {
            raw_layout::TypeKind::FixedArray { element, length } => {
                interners.arrays.insert((element.0, length), node.id);
            }
            raw_layout::TypeKind::Vec { element } => {
                interners.vectors.insert(element.0, node.id);
            }
            _ => {}
        }
    }
    let node =
        resolve_graph_type(file, id, module, declarations, &mut scratch, &mut interners, errors)?;
    node_types.get(usize::try_from(node.0).ok()?).and_then(|v| *v)
}

struct Errors<'a> {
    sources: &'a SourceMap,
    diagnostics: Vec<Diagnostic>,
    exhausted: bool,
}
impl<'a> Errors<'a> {
    fn new(sources: &'a SourceMap) -> Self {
        Self { sources, diagnostics: Vec::new(), exhausted: false }
    }
    fn at(
        &mut self,
        code: &'static str,
        span: Span,
        message: impl Into<String>,
        guidance: &'static str,
    ) {
        self.push(Diagnostic::error_at(code, span, message, guidance));
    }
    fn global(&mut self, code: &'static str, message: impl Into<String>, guidance: &'static str) {
        self.push(Diagnostic::error(code, None, message, guidance));
    }
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.exhausted {
            return;
        }
        if self.diagnostics.len() < MAX_SEMANTIC_DIAGNOSTICS - 1 {
            self.diagnostics.push(diagnostic);
        } else {
            let message = format!(
                "semantic analysis reached its diagnostic limit of {MAX_SEMANTIC_DIAGNOSTICS}"
            );
            let guidance = "fix the retained diagnostics before compiling again";
            self.diagnostics.push(if let Some(at) = diagnostic.primary_span() {
                Diagnostic::error_at("ZRYNA-M3202", at, message, guidance)
            } else {
                Diagnostic::error("ZRYNA-M3202", None, message, guidance)
            });
            self.exhausted = true;
        }
    }
    fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
    fn len(&self) -> usize {
        self.diagnostics.len()
    }
    fn finish(mut self) -> Vec<Diagnostic> {
        self.diagnostics.sort_by(compare_diagnostics);
        self.diagnostics
    }
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    match (left.primary_span(), right.primary_span()) {
        (Some(l), Some(r)) => {
            (l.file().index(), l.start(), l.end(), left.code(), left.message(), left.guidance())
                .cmp(&(
                    r.file().index(),
                    r.start(),
                    r.end(),
                    right.code(),
                    right.message(),
                    right.guidance(),
                ))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => (left.code(), left.message(), left.guidance()).cmp(&(
            right.code(),
            right.message(),
            right.guidance(),
        )),
    }
}

#[cfg(test)]
mod tests;
