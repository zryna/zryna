//! Aggregate semantic lowering for the isolated `DataOwnershipV1` profile.
//!
//! This boundary accepts only authenticated protocol-v4 syntax, derives both layout authorities
//! itself, and returns only verifier-sealed IR. Raw layout and IR claims never cross the API.

use std::{cmp::Ordering, collections::BTreeMap};

use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::data_ownership_v1::{self as ir, RuntimeContractIdentity, raw};
use zryna_layout::{self as layout, StorageTarget, TypeCategory, raw as raw_layout};
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

/// Successful M3 lowering always carries mandatory verifier authority.
pub type SemanticResult = Result<ir::VerifiedProgram, Vec<Diagnostic>>;

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
    let node_types = map_node_types(&graph, &linear, &mut errors);
    if !errors.is_empty() {
        return Err(errors.finish());
    }

    let mut modules = Vec::with_capacity(input.syntax().files().len());
    for (module_index, file) in input.syntax().files().iter().enumerate() {
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
            if let Some(lowered) = lower_function(
                input,
                module_index,
                function_index,
                function,
                &declarations,
                &graph,
                &node_types,
                &linear,
                &mut errors,
            ) {
                functions.push(lowered);
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
    ir::verify(
        raw::Program {
            authorities: claims,
            entry_module: raw::ModuleId(input.entry().index()),
            modules,
        },
        input.sources(),
        input.entry(),
        linear,
        linux,
    )
}

fn semantic_preflight(input: SemanticInput<'_>, errors: &mut Errors<'_>) {
    let mut declarations = 0_usize;
    let mut program_values = 0_usize;
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
            let values = derived_value_count(function);
            let violation = value_budget_violation(program_values, values);
            if violation == Some(ValueBudgetLimit::Function) {
                errors.at(
                    "ZRYNA-M3201",
                    span(input.sources(), function.span),
                    format!(
                        "derived values exceed the per-function M3 limit of {}",
                        ir::MAX_VALUES_PER_FUNCTION
                    ),
                    "reduce parameters or result-producing expressions",
                );
                return;
            }
            if violation == Some(ValueBudgetLimit::Program) {
                errors.at(
                    "ZRYNA-M3201",
                    span(input.sources(), function.span),
                    format!(
                        "derived values exceed the program M3 limit of {}",
                        ir::MAX_VALUES_PER_PROGRAM
                    ),
                    "reduce functions, parameters, or result-producing expressions",
                );
                return;
            }
            program_values = program_values.saturating_add(values);
        }
    }
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

fn derived_value_count(function: &syntax::RawFunctionSyntax) -> usize {
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
            RawExpressionKind::FixedArrayConstruction { elements, .. } => {
                elements.iter().map(|id| value(body, *id)).sum()
            }
            RawExpressionKind::Match { arms, .. } => return arms.len(),
            _ => 0,
        };
        children.saturating_add(1)
    }
    let mut count = function.parameters.len();
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return count;
    };
    for statement in &root.statements {
        let Some(statement) =
            usize::try_from(*statement).ok().and_then(|index| function.body.statements.get(index))
        else {
            continue;
        };
        count = count.saturating_add(match statement.kind {
            RawStatementKind::LocalDeclaration { initializer, .. } => {
                value(&function.body, initializer)
            }
            RawStatementKind::Assignment { target, value: rhs, .. } => {
                place(&function.body, target).saturating_add(value(&function.body, rhs))
            }
            RawStatementKind::Return { value: returned, .. } => value(&function.body, returned),
            _ => 0,
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
    let mut arrays = BTreeMap::new();
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
                                &mut arrays,
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
                                    &mut arrays,
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
                    &mut arrays,
                    errors,
                );
            }
            add_root(
                file,
                function.result_type,
                module,
                &declarations,
                &mut graph,
                &mut arrays,
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
                        &mut arrays,
                        errors,
                    );
                }
            }
            for expression in &function.body.expressions {
                if let RawExpressionKind::FixedArrayConstruction { type_syntax, .. } =
                    expression.kind
                {
                    add_root(
                        file,
                        type_syntax,
                        module,
                        &declarations,
                        &mut graph,
                        &mut arrays,
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
    arrays: &mut BTreeMap<(u32, u64), raw_layout::NodeId>,
    errors: &mut Errors<'_>,
) {
    if let Some(node) = resolve_graph_type(file, id, module, declarations, graph, arrays, errors) {
        graph.program_roots.push(node);
    }
}

fn resolve_graph_type(
    file: &syntax::SourceUnit,
    id: u32,
    module: usize,
    declarations: &[Decl],
    graph: &mut raw_layout::Graph,
    arrays: &mut BTreeMap<(u32, u64), raw_layout::NodeId>,
    errors: &mut Errors<'_>,
) -> Option<raw_layout::NodeId> {
    let ty = usize::try_from(id).ok().and_then(|i| file.type_syntax().get(i))?;
    match &ty.kind {
        RawTypeSyntaxKind::Named { name } if name.text == "bool" => Some(raw_layout::NodeId(0)),
        RawTypeSyntaxKind::Named { name } if name.text == "i32" => Some(raw_layout::NodeId(1)),
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
                resolve_graph_type(file, *element, module, declarations, graph, arrays, errors)?;
            let length = u64::from(*length);
            if let Some(id) = arrays.get(&(element.0, length)).copied() {
                return Some(id);
            }
            let id = raw_layout::NodeId(u32::try_from(graph.types.len()).ok()?);
            graph.types.push(raw_layout::TypeNode {
                id,
                span: None,
                kind: raw_layout::TypeKind::FixedArray { element, length },
            });
            arrays.insert((element.0, length), id);
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
            _ => None,
        };
        if let Some(found) = found {
            let index = usize::try_from(node.id.0).expect("bounded node");
            result[index] = Some(Ty {
                layout: found.id(),
                ir: raw::TypeId(found.id().index()),
                category: found.category(),
            });
        } else {
            errors.global(
                "ZRYNA-M3004",
                format!("derived layout type node #{} has no sealed identity", node.id.0),
                "reduce the aggregate graph and report this deterministic compiler failure",
            );
        }
    }
    result
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

#[derive(Clone)]
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
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    places: Vec<raw::Place>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    instructions: Vec<raw::Instruction>,
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
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    verify_single_final_return(function, input.sources(), errors)?;
    let result =
        semantic_type(file, function.result_type, module, declarations, graph, node_types, errors)?;
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
        errors,
        bindings: BTreeMap::new(),
        places: Vec::new(),
        projections: BTreeMap::new(),
        instructions: Vec::new(),
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
                    raw::InstructionKind::ReplacePlace {
                        place,
                        value: value.1,
                        cleanup: raw::CleanupPlanId(0),
                    },
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
    let cleanup = raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span: span(input.sources(), function.body.span),
        actions: Vec::new(),
    };
    let block = raw::Block {
        id: raw::BlockId(0),
        parameters: Vec::new(),
        instructions: lowerer.instructions,
        terminators: vec![raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: return_value, cleanup: raw::CleanupPlanId(0) },
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
        cleanup_plans: vec![cleanup],
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
                let ty =
                    layouts.types().find(|t| t.category() == TypeCategory::I32).map(|t| Ty {
                        layout: t.id(),
                        ir: raw::TypeId(t.id().index()),
                        category: TypeCategory::I32,
                    })?;
                (ty, raw::InstructionKind::I32Literal(value))
            }
            RawExpressionKind::BoolLiteral { value } => {
                let ty =
                    layouts.types().find(|t| t.category() == TypeCategory::Bool).map(|t| Ty {
                        layout: t.id(),
                        ir: raw::TypeId(t.id().index()),
                        category: TypeCategory::Bool,
                    })?;
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
        arm_block.terminators.push(raw::SpannedTerminator {
            span: span(input.sources(), arm.span),
            kind: raw::Terminator::Return { value: value_id, cleanup: raw::CleanupPlanId(0) },
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
    let cleanup = raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span: span(input.sources(), function.body.span),
        actions: Vec::new(),
    };
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
        cleanup_plans: vec![cleanup],
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
        self.layouts.types().find(|v| v.category() == category).map(|v| Ty {
            layout: v.id(),
            ir: raw::TypeId(v.id().index()),
            category,
        })
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
        let record = self.layouts.type_by_id(element)?;
        Some((
            index,
            Ty { layout: element, ir: raw::TypeId(element.index()), category: record.category() },
        ))
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
        let er = self.layouts.type_by_id(element)?;
        let element_ty =
            Ty { layout: element, ir: raw::TypeId(element.index()), category: er.category() };
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
    let mut arrays = graph
        .types
        .iter()
        .filter_map(|node| {
            if let raw_layout::TypeKind::FixedArray { element, length } = node.kind {
                Some(((element.0, length), node.id))
            } else {
                None
            }
        })
        .collect();
    let node =
        resolve_graph_type(file, id, module, declarations, &mut scratch, &mut arrays, errors)?;
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
            self.diagnostics.push(Diagnostic::error(
                "ZRYNA-M3202",
                None,
                format!(
                    "semantic analysis reached its diagnostic limit of {MAX_SEMANTIC_DIAGNOSTICS}"
                ),
                "fix the retained diagnostics before compiling again",
            ));
            self.exhausted = true;
        }
    }
    fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
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
