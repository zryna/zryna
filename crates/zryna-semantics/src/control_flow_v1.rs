//! Control-flow semantic analysis for the isolated `ControlFlowV1` profile.
//!
//! This module is intentionally separate from the stable protocol-v2/M1 surface. It accepts only
//! an exact source-map-bound protocol-v3 snapshot, reconstructs and validates the complete module
//! graph, owns every source-language name and type decision, and returns only verifier-sealed IR.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, VecDeque},
};

use zryna_abi::{AbiViolationKind, raw as raw_abi, verify_v1};
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::{
    Type,
    control_flow_v1::{self as ir, raw},
};
use zryna_source::{FileId, NormalizedSourcePath, SourceMap, Span, resolve_explicit_zry_import};
use zryna_syntax::v3::{
    self as syntax, BlockId, ExpressionId, ExpressionKind, FunctionBodySyntax, FunctionSyntax,
    StatementKind, TypeSyntax, TypeSyntaxKind,
};

/// Maximum retained M2 semantic diagnostics, including the terminal budget diagnostic.
pub const MAX_SEMANTIC_DIAGNOSTICS: usize = 256;
/// Maximum direct call sites admitted by M2 semantic analysis.
pub const MAX_CALL_EDGES: usize = 65_536;
/// Maximum static direct-call depth admitted by M2 semantic analysis.
pub const MAX_STATIC_CALL_DEPTH: usize = 128;

const _: () = {
    assert!(syntax::MAX_FUNCTIONS_PER_MODULE <= ir::MAX_FUNCTIONS_PER_MODULE);
    assert!(syntax::MAX_FUNCTIONS_PER_PROJECT <= ir::MAX_FUNCTIONS_PER_PROGRAM);
    assert!(syntax::MAX_PARAMETERS_PER_FUNCTION <= ir::MAX_PARAMETERS_PER_FUNCTION);
    assert!(syntax::MAX_PARAMETERS_PER_PROJECT <= ir::MAX_PARAMETERS_PER_PROGRAM);
    assert!(syntax::MAX_EXPRESSIONS_PER_FUNCTION <= ir::MAX_VALUES_PER_FUNCTION);
    assert!(syntax::MAX_EXPRESSIONS_PER_PROJECT <= ir::MAX_VALUES_PER_PROGRAM);
    assert!(MAX_CALL_EDGES == ir::MAX_CALL_EDGES);
    assert!(MAX_STATIC_CALL_DEPTH == ir::MAX_STATIC_CALL_DEPTH);
};

/// Exact authenticated inputs for control-flow M2 semantics.
///
/// Raw protocol-v3 claims cannot enter this boundary:
///
/// ```compile_fail
/// fn bypass<'a>(
///     raw: &'a zryna_syntax::v3::RawProjectSyntaxSnapshot,
///     sources: &'a zryna_source::SourceMap,
///     entry: zryna_source::FileId,
/// ) -> Option<zryna_semantics::control_flow_v1::SemanticInput<'a>> {
///     zryna_semantics::control_flow_v1::SemanticInput::try_new(raw, sources, entry)
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SemanticInput<'a> {
    syntax: &'a syntax::ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
    entry: FileId,
}

impl<'a> SemanticInput<'a> {
    /// Binds verified v3 syntax and an independently selected entry file to one exact source map.
    ///
    /// Returns `None` for a mismatched source authority, provider error, unknown entry file, or an
    /// entry not represented exactly once by the verified snapshot.
    #[must_use]
    pub fn try_new(
        syntax: &'a syntax::ProjectSyntaxSnapshot,
        sources: &'a SourceMap,
        entry: FileId,
    ) -> Option<Self> {
        (syntax.is_bound_to(sources)
            && sources.source(entry).is_some()
            && syntax.files().iter().filter(|file| file.id() == entry).count() == 1
            && syntax
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.severity() != Severity::Error))
        .then_some(Self { syntax, sources, entry })
    }

    /// Returns the exact verified syntax snapshot.
    #[must_use]
    pub const fn syntax(self) -> &'a syntax::ProjectSyntaxSnapshot {
        self.syntax
    }

    /// Returns the authoritative source map.
    #[must_use]
    pub const fn sources(self) -> &'a SourceMap {
        self.sources
    }

    /// Returns the independently selected entry file.
    #[must_use]
    pub const fn entry(self) -> FileId {
        self.entry
    }
}

/// Successful M2 semantics always carries mandatory verifier authority.
pub type SemanticResult = Result<ir::VerifiedProgram, Vec<Diagnostic>>;

#[derive(Clone)]
struct Signature {
    parameters: Vec<Option<Type>>,
    result: Option<Type>,
}

#[derive(Clone)]
struct FunctionSymbol {
    id: raw::FunctionId,
    name: String,
    name_span: Span,
    declaration_span: Span,
    exported: bool,
    signature: Signature,
}

#[derive(Default)]
struct ModuleSymbols {
    functions: Vec<FunctionSymbol>,
    local_functions: BTreeMap<String, usize>,
    callables: BTreeMap<String, raw::FunctionId>,
    imports: Vec<ImportEdge>,
}

#[derive(Clone, Copy)]
struct ImportEdge {
    target: usize,
    span: Span,
}

#[derive(Clone, Copy)]
struct TypedValue {
    ty: Type,
    value: raw::ValueId,
}

#[derive(Clone)]
struct Binding {
    name: String,
    ty: Type,
    mutable: bool,
    value: raw::ValueId,
    span: Span,
    depth: usize,
    parameter: bool,
}

#[derive(Clone, Copy)]
struct CallEdge {
    caller: usize,
    callee: usize,
    span: Span,
}

/// Resolves strict M2 names and types, lowers straight-line bodies, and immediately invokes the
/// mandatory `ControlFlowV1` verifier.
///
/// No raw or partially valid IR is exposed. `if` and `while` remain rejected until Issue #50.
///
/// # Errors
///
/// Returns deterministic bounded M2 semantic diagnostics or mandatory IR verifier diagnostics.
pub fn lower(input: SemanticInput<'_>) -> SemanticResult {
    let sources = input.sources();
    let snapshot = input.syntax();
    let mut errors = Errors::new(sources);
    let Some(mut modules) = run_phase(&mut errors, |errors| collect_signatures(snapshot, errors))
    else {
        return Err(errors.finish());
    };
    let paths = snapshot
        .files()
        .iter()
        .enumerate()
        .map(|(index, file)| (file.path().clone(), index))
        .collect::<BTreeMap<_, _>>();
    if run_phase(&mut errors, |errors| {
        resolve_imports(snapshot, &paths, &mut modules, errors);
    })
    .is_none()
    {
        return Err(errors.finish());
    }
    if run_phase(&mut errors, |errors| {
        verify_module_graph(snapshot, input.entry(), &modules, errors);
    })
    .is_none()
    {
        return Err(errors.finish());
    }
    if run_phase(&mut errors, |errors| verify_entry_exports(input.entry(), &modules, errors))
        .is_none()
    {
        return Err(errors.finish());
    }

    let offsets = function_offsets(&modules);
    let mut call_edges = Vec::new();
    let mut raw_modules = Vec::with_capacity(snapshot.files().len());
    'modules: for (module_index, file) in snapshot.files().iter().enumerate() {
        let mut functions = Vec::with_capacity(file.functions().len());
        for (function_index, function) in file.functions().iter().enumerate() {
            let symbol = &modules[module_index].functions[function_index];
            if let Some(lowered) = lower_function(
                function,
                symbol,
                &modules[module_index].callables,
                &modules,
                &offsets,
                &mut call_edges,
                &mut errors,
                file.id() == input.entry(),
            ) {
                functions.push(lowered);
            }
            if errors.is_exhausted() {
                break 'modules;
            }
        }
        raw_modules.push(raw::Module {
            id: raw::ModuleId(u32::try_from(module_index).unwrap_or(u32::MAX)),
            source_file: file.id(),
            functions,
        });
    }
    if !errors.is_exhausted() {
        verify_call_graph(&modules, &call_edges, &mut errors);
    }
    if !errors.is_empty() {
        return Err(errors.finish());
    }

    ir::verify(
        raw::Program { entry_module: raw::ModuleId(input.entry().index()), modules: raw_modules },
        sources,
        input.entry(),
    )
}

fn run_phase<T>(errors: &mut Errors<'_>, phase: impl FnOnce(&mut Errors<'_>) -> T) -> Option<T> {
    if errors.is_exhausted() {
        return None;
    }
    let result = phase(errors);
    (!errors.is_exhausted()).then_some(result)
}

fn collect_signatures(
    snapshot: &syntax::ProjectSyntaxSnapshot,
    errors: &mut Errors<'_>,
) -> Vec<ModuleSymbols> {
    let mut modules = Vec::with_capacity(snapshot.files().len());
    'modules: for (module_index, file) in snapshot.files().iter().enumerate() {
        let mut module = ModuleSymbols::default();
        for (declaration, function) in file.functions().iter().enumerate() {
            if errors.is_exhausted() {
                break 'modules;
            }
            let mut parameters = Vec::with_capacity(function.parameters().len());
            let mut parameter_names = BTreeMap::<String, Span>::new();
            for parameter in function.parameters() {
                if errors.is_exhausted() {
                    break 'modules;
                }
                let parameter_type = lower_type(parameter.type_syntax(), "parameter", errors);
                if errors.is_exhausted() {
                    break 'modules;
                }
                parameters.push(parameter_type);
                if parameter_names
                    .insert(parameter.name().text().to_owned(), parameter.name().span())
                    .is_some()
                {
                    errors.at(
                        "ZRYNA-M2003",
                        parameter.name().span(),
                        format!(
                            "parameter '{}' is declared more than once",
                            parameter.name().text()
                        ),
                        "give every parameter one exact unique name within its function",
                    );
                }
            }
            if errors.is_exhausted() {
                break 'modules;
            }
            let result = lower_type(function.result_type(), "result", errors);
            if errors.is_exhausted() {
                break 'modules;
            }
            let symbol = FunctionSymbol {
                id: raw::FunctionId {
                    module: raw::ModuleId(u32::try_from(module_index).unwrap_or(u32::MAX)),
                    declaration: u32::try_from(declaration).unwrap_or(u32::MAX),
                },
                name: function.name().text().to_owned(),
                name_span: function.name().span(),
                declaration_span: function.span(),
                exported: function.export_span().is_some(),
                signature: Signature { parameters, result },
            };
            if module.local_functions.insert(symbol.name.clone(), declaration).is_some() {
                errors.at(
                    "ZRYNA-M2001",
                    symbol.name_span,
                    format!("function '{}' is declared more than once in this module", symbol.name),
                    "give every top-level function one exact unique module name",
                );
            } else {
                module.callables.insert(symbol.name.clone(), symbol.id);
            }
            if errors.is_exhausted() {
                break 'modules;
            }
            module.functions.push(symbol);
        }
        modules.push(module);
    }
    modules
}

fn resolve_imports(
    snapshot: &syntax::ProjectSyntaxSnapshot,
    paths: &BTreeMap<NormalizedSourcePath, usize>,
    modules: &mut [ModuleSymbols],
    errors: &mut Errors<'_>,
) {
    'modules: for (module_index, file) in snapshot.files().iter().enumerate() {
        for import in file.imports() {
            if errors.is_exhausted() {
                break 'modules;
            }
            let Ok(target_path) =
                resolve_explicit_zry_import(file.path(), import.specifier().text())
            else {
                errors.at(
                    "ZRYNA-M2010",
                    import.specifier().token_span(),
                    "module import does not resolve under the explicit portable .zry grammar",
                    "use one explicit relative lowercase .zry path inside the workspace",
                );
                continue;
            };
            let Some(&target_module) = paths.get(&target_path) else {
                errors.at(
                    "ZRYNA-M2010",
                    import.specifier().token_span(),
                    format!(
                        "module '{target_path}' is absent from the authenticated source closure"
                    ),
                    "compile the complete driver-authenticated module closure",
                );
                continue;
            };
            modules[module_index]
                .imports
                .push(ImportEdge { target: target_module, span: import.specifier().token_span() });
            for binding in import.bindings() {
                if errors.is_exhausted() {
                    break 'modules;
                }
                let target = modules[target_module]
                    .local_functions
                    .get(binding.imported().text())
                    .copied()
                    .and_then(|index| modules[target_module].functions.get(index));
                let Some(target) = target.filter(|target| target.exported) else {
                    errors.at(
                        "ZRYNA-M2010",
                        binding.imported().span(),
                        format!(
                            "module '{}' does not export function '{}'",
                            target_path,
                            binding.imported().text()
                        ),
                        "import one explicitly exported function from the resolved module",
                    );
                    continue;
                };
                let local = binding.local().text().to_owned();
                if modules[module_index].callables.insert(local.clone(), target.id).is_some() {
                    errors.at(
                        "ZRYNA-M2011",
                        binding.local().span(),
                        format!("callable name '{local}' collides in this module"),
                        "use an exact unique import alias that does not match another import or function",
                    );
                }
            }
        }
    }
}

fn verify_module_graph(
    snapshot: &syntax::ProjectSyntaxSnapshot,
    entry: FileId,
    modules: &[ModuleSymbols],
    errors: &mut Errors<'_>,
) {
    let Some(entry_index) =
        usize::try_from(entry.index()).ok().filter(|index| *index < modules.len())
    else {
        errors.global(
            "ZRYNA-M2010",
            "the selected entry module is outside the authenticated module table",
            "select one exact file from the final authenticated source map",
        );
        return;
    };
    let mut state = vec![0_u8; modules.len()];
    let mut reachable = vec![false; modules.len()];
    let mut stack = vec![(entry_index, 0_usize)];
    state[entry_index] = 1;
    reachable[entry_index] = true;
    while let Some((module, edge_index)) = stack.last_mut() {
        if errors.is_exhausted() {
            return;
        }
        if *edge_index == modules[*module].imports.len() {
            state[*module] = 2;
            stack.pop();
            continue;
        }
        let edge = modules[*module].imports[*edge_index];
        *edge_index += 1;
        reachable[edge.target] = true;
        match state[edge.target] {
            0 => {
                state[edge.target] = 1;
                stack.push((edge.target, 0));
            }
            1 => errors.at(
                "ZRYNA-M2010",
                edge.span,
                "the resolved module import graph contains a cycle",
                "remove the cyclic relative import chain",
            ),
            _ => {}
        }
    }
    for (index, is_reachable) in reachable.into_iter().enumerate() {
        if errors.is_exhausted() {
            return;
        }
        if !is_reachable {
            errors.path(
                "ZRYNA-M2010",
                snapshot.files()[index].path().as_str(),
                "authenticated source closure contains a module unreachable from the selected entry",
                "pass one complete source-map-bound verified snapshot",
            );
        }
    }
}

fn verify_entry_exports(entry: FileId, modules: &[ModuleSymbols], errors: &mut Errors<'_>) {
    let Some(module) = usize::try_from(entry.index()).ok().and_then(|index| modules.get(index))
    else {
        return;
    };
    let mut exported = Vec::new();
    let exports = module
        .functions
        .iter()
        .filter(|function| function.exported)
        .filter_map(|function| {
            let parameters = function
                .signature
                .parameters
                .iter()
                .copied()
                .map(|ty| ty.map(abi_type))
                .collect::<Option<Vec<_>>>()?;
            let result = abi_type(function.signature.result?);
            exported.push(function);
            Some(raw_abi::Export::new(
                function.name.clone(),
                raw_abi::Signature::new(parameters, result),
            ))
        })
        .collect();
    let Err(violations) = verify_v1(raw_abi::Module::new(exports)) else {
        return;
    };
    for violation in violations {
        if errors.is_exhausted() {
            return;
        }
        let function = violation.export_index().and_then(|index| exported.get(index)).copied();
        let (message, guidance) = match violation.kind() {
            AbiViolationKind::InvalidLogicalName => (
                "entry export is not a valid scalar ABI logical name".to_owned(),
                "use 1 to 128 ASCII bytes matching [A-Za-z_][A-Za-z0-9_]* and avoid reserved bindings",
            ),
            AbiViolationKind::DuplicateLogicalName { .. } => (
                "entry export duplicates another exact logical name".to_owned(),
                "give every entry export one exact unique name",
            ),
            AbiViolationKind::PortableNameCollision { .. } => (
                "entry exports collide under the portable target identity".to_owned(),
                "choose names that remain unique when ASCII case is ignored",
            ),
            AbiViolationKind::TooManyExports
            | AbiViolationKind::TooManyParameters
            | AbiViolationKind::TooManyParametersInModule
            | AbiViolationKind::ViolationBudgetExceeded => {
                errors.limit(
                    "ZRYNA-M2201",
                    "entry exports exceed the deterministic scalar ABI budget",
                    "reduce exported declarations before semantic analysis",
                );
                return;
            }
            AbiViolationKind::UnsupportedScalarType => continue,
        };
        if let Some(function) = function {
            errors.at("ZRYNA-M2022", function.name_span, message, guidance);
        } else {
            errors.global("ZRYNA-M2022", message, guidance);
        }
    }
}

fn abi_type(ty: Type) -> raw_abi::Type {
    match ty {
        Type::Bool => raw_abi::Type::Bool,
        Type::I32 => raw_abi::Type::I32,
        Type::Unit => raw_abi::Type::Unit,
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_function(
    syntax: &FunctionSyntax,
    symbol: &FunctionSymbol,
    callables: &BTreeMap<String, raw::FunctionId>,
    modules: &[ModuleSymbols],
    offsets: &[usize],
    call_edges: &mut Vec<CallEdge>,
    errors: &mut Errors<'_>,
    entry_module: bool,
) -> Option<raw::Function> {
    let mut parameters = syntax
        .parameters()
        .iter()
        .zip(symbol.signature.parameters.iter().copied())
        .enumerate()
        .map(|(index, (parameter, ty))| {
            Some(raw::ValueDefinition {
                id: raw::ValueId(u32::try_from(index).ok()?),
                ty: ty?,
                span: parameter.name().span(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let result = symbol.signature.result?;
    let caller = offsets[symbol.id.module.0 as usize] + symbol.id.declaration as usize;
    let mut lowerer = FunctionLowerer {
        body: syntax.body(),
        callables,
        modules,
        offsets,
        caller,
        errors,
        call_edges,
        bindings: Vec::new(),
        expression_values: vec![None; syntax.body().expressions().len()],
        blocks: vec![empty_block(0)],
        current_block: 0,
        cfg_edges: 0,
        next_value: u32::try_from(parameters.len()).unwrap_or(u32::MAX),
    };
    for (parameter, definition) in syntax.parameters().iter().zip(&parameters) {
        lowerer.bindings.push(Binding {
            name: parameter.name().text().to_owned(),
            ty: definition.ty,
            mutable: false,
            value: definition.id,
            span: parameter.name().span(),
            depth: 0,
            parameter: true,
        });
    }
    let open = lowerer.lower_syntax_block(syntax.body().root_block(), 0, result);
    if lowerer.errors.is_exhausted() {
        return None;
    }
    if open {
        lowerer.errors.at(
            "ZRYNA-M2009",
            syntax.body().span(),
            "function can fall through without returning its declared result",
            "end every reachable function path with one typed return",
        );
    }
    if !lowerer.errors.is_empty() {
        return None;
    }
    prune_dead_block_parameters(&mut lowerer.blocks);
    if lowerer.blocks.iter().any(|block| block.parameters.len() > ir::MAX_BLOCK_PARAMETERS) {
        lowerer.errors.limit_at(
            "ZRYNA-M2201",
            syntax.body().span(),
            "live mutable bindings exceed the deterministic merge/header limit",
            "reduce simultaneously live mutable bindings across control-flow edges",
        );
        return None;
    }
    if !canonicalize_values(&mut parameters, &mut lowerer.blocks, lowerer.errors) {
        return None;
    }
    Some(raw::Function {
        id: symbol.id,
        entry_export: (entry_module && symbol.exported).then(|| symbol.name.clone()),
        span: symbol.declaration_span,
        parameters,
        result,
        blocks: lowerer.blocks,
    })
}

fn empty_block(index: usize) -> raw::Block {
    raw::Block {
        id: raw::BlockId(u32::try_from(index).unwrap_or(u32::MAX)),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: Vec::new(),
    }
}

struct FunctionLowerer<'body, 'symbols, 'state, 'source> {
    body: &'body FunctionBodySyntax,
    callables: &'symbols BTreeMap<String, raw::FunctionId>,
    modules: &'symbols [ModuleSymbols],
    offsets: &'symbols [usize],
    caller: usize,
    errors: &'state mut Errors<'source>,
    call_edges: &'state mut Vec<CallEdge>,
    bindings: Vec<Binding>,
    expression_values: Vec<Option<TypedValue>>,
    blocks: Vec<raw::Block>,
    current_block: usize,
    cfg_edges: usize,
    next_value: u32,
}

impl FunctionLowerer<'_, '_, '_, '_> {
    #[allow(clippy::too_many_lines)]
    fn lower_syntax_block(&mut self, block_id: BlockId, depth: usize, result: Type) -> bool {
        let marker = self.bindings.len();
        let block = &self.body.blocks()[block_id.index() as usize];
        let statements = block.statements().to_vec();
        let mut open = true;
        for statement_id in statements {
            if self.errors.is_exhausted() {
                return false;
            }
            let statement = self.body.statements()[statement_id.index() as usize].clone();
            if !open {
                self.errors.at(
                    "ZRYNA-M2009",
                    statement.span(),
                    "statement is unreachable after control flow returns on every path",
                    "remove the unreachable statement",
                );
                continue;
            }
            match statement.kind() {
                StatementKind::LocalDeclaration {
                    name, mutable, type_syntax, initializer, ..
                } => {
                    let value = self.lower_root(*initializer);
                    if self.errors.is_exhausted() {
                        return false;
                    }
                    let ty = lower_type(type_syntax, "local", self.errors);
                    if self.errors.is_exhausted() {
                        return false;
                    }
                    let duplicate = self.bindings.iter().any(|binding| {
                        binding.name == name.text()
                            && (binding.depth == depth || (depth == 0 && binding.parameter))
                    });
                    if duplicate {
                        self.errors.at(
                            "ZRYNA-M2003",
                            name.span(),
                            format!("binding '{}' cannot be redeclared in this block", name.text()),
                            "use a unique name in this block; nested blocks may shadow outer bindings",
                        );
                    }
                    if self.errors.is_exhausted() {
                        return false;
                    }
                    if let (Some(value), Some(ty)) = (value, ty) {
                        if value.ty != ty {
                            self.errors.at(
                                "ZRYNA-M2006",
                                name.span(),
                                "local initializer does not match its declared exact type",
                                "initialize the local with the declared i32 or bool type",
                            );
                        } else if !duplicate {
                            self.bindings.push(Binding {
                                name: name.text().to_owned(),
                                ty,
                                mutable: *mutable,
                                value: value.value,
                                span: name.span(),
                                depth,
                                parameter: false,
                            });
                        }
                    }
                }
                StatementKind::Assignment { target, value, .. } => {
                    let value = self.lower_root(*value);
                    if self.errors.is_exhausted() {
                        return false;
                    }
                    let binding =
                        self.bindings.iter().rposition(|binding| binding.name == target.text());
                    let Some(binding) = binding else {
                        self.errors.at(
                            "ZRYNA-M2005",
                            target.span(),
                            format!("assignment target '{}' is not declared", target.text()),
                            "assign only a previously declared mutable let binding",
                        );
                        continue;
                    };
                    if !self.bindings[binding].mutable {
                        self.errors.at(
                            "ZRYNA-M2005",
                            target.span(),
                            format!("binding '{}' is not mutable", target.text()),
                            "assign only a let binding; const values and parameters are immutable",
                        );
                        continue;
                    }
                    if let Some(value) = value {
                        if value.ty == self.bindings[binding].ty {
                            self.bindings[binding].value = value.value;
                        } else {
                            self.errors.at(
                                "ZRYNA-M2006",
                                target.span(),
                                "assignment value does not match the binding's exact type",
                                "assign a value with exactly the declared i32 or bool type",
                            );
                        }
                    }
                }
                StatementKind::Return { value, .. } => {
                    let value = self.lower_root(*value);
                    if self.errors.is_exhausted() {
                        return false;
                    }
                    if let Some(value) = value {
                        if value.ty == result {
                            if !self.terminate(raw::SpannedTerminator {
                                span: statement.span(),
                                kind: raw::Terminator::Return(value.value),
                            }) {
                                return false;
                            }
                            open = false;
                        } else {
                            self.errors.at(
                                "ZRYNA-M2009",
                                statement.span(),
                                "returned value does not match the declared exact result type",
                                "return a value with exactly the function's declared type",
                            );
                        }
                    }
                }
                StatementKind::Block { block } => {
                    open = self.lower_syntax_block(*block, depth.saturating_add(1), result);
                }
                StatementKind::If { condition, then_block, else_clause, keyword_span, .. } => {
                    open = self.lower_if(
                        *condition,
                        *then_block,
                        else_clause.as_ref().map(syntax::ElseSyntax::block),
                        *keyword_span,
                        depth,
                        result,
                    );
                }
                StatementKind::While { condition, body_block, keyword_span, .. } => {
                    open = self.lower_while(*condition, *body_block, *keyword_span, depth, result);
                }
            }
        }
        if open {
            self.bindings.truncate(marker);
        }
        open
    }

    fn lower_root(&mut self, root: ExpressionId) -> Option<TypedValue> {
        if self.errors.is_exhausted() {
            return None;
        }
        let root_index = root.index() as usize;
        if root_index >= self.body.expressions().len() {
            self.errors.limit_at(
                "ZRYNA-M2201",
                self.body.span(),
                "verified expression root is outside its sealed arena",
                "report this compiler invariant failure with the smallest source",
            );
            return None;
        }
        let mut order = Vec::new();
        let mut stack = vec![(root, false)];
        while let Some((id, exit)) = stack.pop() {
            let index = id.index() as usize;
            if exit {
                order.push(index);
                continue;
            }
            stack.push((id, true));
            let mut children = expression_children(self.body.expressions()[index].kind());
            children.reverse();
            stack.extend(children.into_iter().map(|child| (child, false)));
        }
        for index in order {
            if self.errors.is_exhausted() {
                return None;
            }
            let expression = self.body.expressions()[index].clone();
            let value = self.lower_expression(expression.kind(), expression.span());
            if !store_expression_result(self.errors, &mut self.expression_values, index, value) {
                return None;
            }
        }
        let result = self.expression_values[root_index];
        for value in &mut self.expression_values {
            *value = None;
        }
        result
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn lower_if(
        &mut self,
        condition: ExpressionId,
        then_block: BlockId,
        else_block: Option<BlockId>,
        span: Span,
        depth: usize,
        result: Type,
    ) -> bool {
        let condition_span = self.body.expressions()[condition.index() as usize].span();
        let Some(condition) = self.lower_root(condition) else { return false };
        if condition.ty != Type::Bool {
            self.errors.at(
                "ZRYNA-M2014",
                condition_span,
                "if condition does not have exact bool type",
                "use one bool expression without truthiness or numeric coercion",
            );
        }
        if self.errors.is_exhausted() {
            return false;
        }
        let incoming = self.bindings.clone();
        let Some(true_target) = self.new_block() else { return false };
        let Some(false_target) = self.new_block() else { return false };
        if !self.terminate(raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Branch {
                condition: condition.value,
                true_target: raw::BlockId(u32::try_from(true_target).unwrap_or(u32::MAX)),
                true_arguments: Vec::new(),
                false_target: raw::BlockId(u32::try_from(false_target).unwrap_or(u32::MAX)),
                false_arguments: Vec::new(),
            },
        }) {
            return false;
        }

        self.current_block = true_target;
        self.bindings = incoming.clone();
        let true_open = self.lower_syntax_block(then_block, depth.saturating_add(1), result);
        let true_state = true_open.then(|| (self.current_block, self.bindings.clone()));

        self.current_block = false_target;
        self.bindings = incoming.clone();
        let false_open = else_block
            .is_none_or(|block| self.lower_syntax_block(block, depth.saturating_add(1), result));
        let false_state = false_open.then(|| (self.current_block, self.bindings.clone()));

        match (true_state, false_state) {
            (None, None) => false,
            (Some((block, bindings)), None) | (None, Some((block, bindings))) => {
                self.current_block = block;
                self.bindings = bindings;
                true
            }
            (Some((true_block, true_bindings)), Some((false_block, false_bindings))) => {
                let carried = incoming
                    .iter()
                    .enumerate()
                    .filter(|(index, binding)| {
                        binding.mutable
                            && true_bindings[*index].value != false_bindings[*index].value
                    })
                    .count();
                if !draft_merge_within_limit(carried, span, self.errors) {
                    return false;
                }
                let Some(join) = self.new_block() else { return false };
                let mut merged = incoming;
                let mut true_arguments = Vec::new();
                let mut false_arguments = Vec::new();
                for index in 0..merged.len() {
                    if !merged[index].mutable
                        || true_bindings[index].value == false_bindings[index].value
                    {
                        merged[index].value = true_bindings[index].value;
                        continue;
                    }
                    let Some(parameter) = self.new_value(merged[index].ty, merged[index].span)
                    else {
                        return false;
                    };
                    self.blocks[join].parameters.push(raw::ValueDefinition {
                        id: parameter.value,
                        ty: parameter.ty,
                        span: merged[index].span,
                    });
                    true_arguments.push(true_bindings[index].value);
                    false_arguments.push(false_bindings[index].value);
                    merged[index].value = parameter.value;
                }
                self.current_block = true_block;
                if !self.terminate(raw::SpannedTerminator {
                    span,
                    kind: raw::Terminator::Jump {
                        target: raw::BlockId(u32::try_from(join).unwrap_or(u32::MAX)),
                        arguments: true_arguments,
                    },
                }) {
                    return false;
                }
                self.current_block = false_block;
                if !self.terminate(raw::SpannedTerminator {
                    span,
                    kind: raw::Terminator::Jump {
                        target: raw::BlockId(u32::try_from(join).unwrap_or(u32::MAX)),
                        arguments: false_arguments,
                    },
                }) {
                    return false;
                }
                self.current_block = join;
                self.bindings = merged;
                true
            }
        }
    }

    fn lower_while(
        &mut self,
        condition: ExpressionId,
        body_block: BlockId,
        span: Span,
        depth: usize,
        result: Type,
    ) -> bool {
        let condition_span = self.body.expressions()[condition.index() as usize].span();
        let incoming = self.bindings.clone();
        let preheader = self.current_block;
        let carried = incoming.iter().filter(|binding| binding.mutable).count();
        if !draft_merge_within_limit(carried, span, self.errors) {
            return false;
        }
        let Some(header) = self.new_block() else { return false };
        let mut header_bindings = incoming.clone();
        let mut entry_arguments = Vec::new();
        let mut carried_indices = Vec::new();
        for (index, binding) in incoming.iter().enumerate() {
            if !binding.mutable {
                continue;
            }
            let Some(parameter) = self.new_value(binding.ty, binding.span) else { return false };
            self.blocks[header].parameters.push(raw::ValueDefinition {
                id: parameter.value,
                ty: parameter.ty,
                span: binding.span,
            });
            entry_arguments.push(binding.value);
            carried_indices.push(index);
            header_bindings[index].value = parameter.value;
        }
        self.current_block = preheader;
        if !self.terminate(raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Jump {
                target: raw::BlockId(u32::try_from(header).unwrap_or(u32::MAX)),
                arguments: entry_arguments,
            },
        }) {
            return false;
        }
        self.current_block = header;
        self.bindings = header_bindings.clone();
        let Some(condition) = self.lower_root(condition) else { return false };
        if condition.ty != Type::Bool {
            self.errors.at(
                "ZRYNA-M2014",
                condition_span,
                "while condition does not have exact bool type",
                "use one bool expression without truthiness or numeric coercion",
            );
        }
        if self.errors.is_exhausted() {
            return false;
        }
        let Some(body) = self.new_block() else { return false };
        let Some(exit) = self.new_block() else { return false };
        self.current_block = header;
        if !self.terminate(raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Branch {
                condition: condition.value,
                true_target: raw::BlockId(u32::try_from(body).unwrap_or(u32::MAX)),
                true_arguments: Vec::new(),
                false_target: raw::BlockId(u32::try_from(exit).unwrap_or(u32::MAX)),
                false_arguments: Vec::new(),
            },
        }) {
            return false;
        }
        self.current_block = body;
        self.bindings = header_bindings.clone();
        if self.lower_syntax_block(body_block, depth.saturating_add(1), result) {
            let backedge =
                carried_indices.iter().map(|index| self.bindings[*index].value).collect();
            if !self.terminate(raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Jump {
                    target: raw::BlockId(u32::try_from(header).unwrap_or(u32::MAX)),
                    arguments: backedge,
                },
            }) {
                return false;
            }
        }
        self.current_block = exit;
        self.bindings = header_bindings;
        true
    }

    fn new_block(&mut self) -> Option<usize> {
        if self.errors.is_exhausted() {
            return None;
        }
        if self.blocks.len() >= ir::MAX_BLOCKS_PER_FUNCTION {
            self.errors.limit_at(
                "ZRYNA-M2201",
                self.body.span(),
                "control-flow lowering exceeds the deterministic IR block limit",
                "reduce nested branches and loops before semantic analysis",
            );
            return None;
        }
        let index = self.blocks.len();
        self.blocks.push(empty_block(index));
        Some(index)
    }

    fn new_value(&mut self, ty: Type, span: Span) -> Option<TypedValue> {
        if self.errors.is_exhausted() {
            return None;
        }
        if !draft_value_within_limit(self.next_value, span, self.errors) {
            return None;
        }
        let value = raw::ValueId(self.next_value);
        let Some(next) = self.next_value.checked_add(1) else {
            self.errors.limit_at(
                "ZRYNA-M2201",
                span,
                "function value identity space is exhausted",
                "reduce expressions and live mutable state before semantic analysis",
            );
            return None;
        };
        self.next_value = next;
        Some(TypedValue { ty, value })
    }

    fn terminate(&mut self, terminator: raw::SpannedTerminator) -> bool {
        if self.errors.is_exhausted() {
            return false;
        }
        let edges = match &terminator.kind {
            raw::Terminator::Return(_) => 0,
            raw::Terminator::Jump { .. } => 1,
            raw::Terminator::Branch { .. } => 2,
        };
        let Some(total) = self.cfg_edges.checked_add(edges) else {
            self.errors.limit_at(
                "ZRYNA-M2201",
                terminator.span,
                "control-flow edge count cannot be represented",
                "reduce nested branches and loops before semantic analysis",
            );
            return false;
        };
        if total > ir::MAX_CFG_EDGES_PER_FUNCTION {
            self.errors.limit_at(
                "ZRYNA-M2201",
                terminator.span,
                "control-flow lowering exceeds the deterministic CFG edge limit",
                "reduce nested branches and loops before semantic analysis",
            );
            return false;
        }
        if !self.blocks[self.current_block].terminators.is_empty() {
            return false;
        }
        self.cfg_edges = total;
        self.blocks[self.current_block].terminators.push(terminator);
        true
    }

    #[allow(clippy::too_many_lines)]
    fn lower_expression(&mut self, kind: &ExpressionKind, span: Span) -> Option<TypedValue> {
        match kind {
            ExpressionKind::Reference { name } => self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == name.text())
                .map(|binding| TypedValue { ty: binding.ty, value: binding.value })
                .or_else(|| {
                    self.errors.at(
                        "ZRYNA-M2004",
                        name.span(),
                        format!("value name '{}' is not in lexical scope", name.text()),
                        "reference a parameter or an already initialized local binding",
                    );
                    None
                }),
            ExpressionKind::BoolLiteral { value } => {
                self.emit(Type::Bool, span, raw::InstructionKind::BoolLiteral(*value))
            }
            ExpressionKind::I32Literal { spelling } => {
                if let Ok(value) = spelling.parse::<i32>() {
                    self.emit(Type::I32, span, raw::InstructionKind::I32Literal(value))
                } else {
                    self.errors.at(
                        "ZRYNA-M2008",
                        span,
                        format!("integer literal '{spelling}' is outside the i32 range"),
                        "use a decimal integer from -2147483648 through 2147483647",
                    );
                    None
                }
            }
            ExpressionKind::Negation { operator_span, operand } => {
                let operand = self.value(*operand)?;
                if operand.ty != Type::I32 {
                    return self
                        .operator_error(*operator_span, "negation requires one i32 operand");
                }
                self.emit(Type::I32, span, raw::InstructionKind::I32Neg { operand: operand.value })
            }
            ExpressionKind::Addition { operator_span, lhs, rhs } => {
                self.binary_i32(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32Add { lhs, rhs }
                })
            }
            ExpressionKind::Subtraction { operator_span, lhs, rhs } => {
                self.binary_i32(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32Sub { lhs, rhs }
                })
            }
            ExpressionKind::Multiplication { operator_span, lhs, rhs } => {
                self.binary_i32(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32Mul { lhs, rhs }
                })
            }
            ExpressionKind::Equal { operator_span, lhs, rhs } => {
                self.equality(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::Eq { lhs, rhs }
                })
            }
            ExpressionKind::NotEqual { operator_span, lhs, rhs } => {
                self.equality(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::Ne { lhs, rhs }
                })
            }
            ExpressionKind::LessThan { operator_span, lhs, rhs } => {
                self.comparison(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32LtS { lhs, rhs }
                })
            }
            ExpressionKind::LessEqual { operator_span, lhs, rhs } => {
                self.comparison(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32LeS { lhs, rhs }
                })
            }
            ExpressionKind::GreaterThan { operator_span, lhs, rhs } => {
                self.comparison(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32GtS { lhs, rhs }
                })
            }
            ExpressionKind::GreaterEqual { operator_span, lhs, rhs } => {
                self.comparison(*operator_span, span, *lhs, *rhs, |lhs, rhs| {
                    raw::InstructionKind::I32GeS { lhs, rhs }
                })
            }
            ExpressionKind::Call { callee, arguments, .. } => {
                let values = arguments
                    .iter()
                    .copied()
                    .map(|argument| self.value(argument))
                    .collect::<Option<Vec<_>>>()?;
                let Some(function_id) = self.callables.get(callee.text()).copied() else {
                    self.errors.at(
                        "ZRYNA-M2012",
                        callee.span(),
                        format!("callable '{}' is not declared in this module", callee.text()),
                        "call one same-module function or named imported function",
                    );
                    return None;
                };
                let signature = &self.modules[function_id.module.0 as usize].functions
                    [function_id.declaration as usize]
                    .signature;
                let expected = signature.parameters.iter().copied().collect::<Option<Vec<_>>>();
                let expected = expected?;
                let result = signature.result?;
                if values.len() != expected.len() {
                    self.errors.at(
                        "ZRYNA-M2012",
                        callee.span(),
                        format!(
                            "call to '{}' has {} arguments but requires {}",
                            callee.text(),
                            values.len(),
                            expected.len()
                        ),
                        "pass exactly the declared arguments in source order",
                    );
                    return None;
                }
                if values.iter().zip(&expected).any(|(actual, expected)| actual.ty != *expected) {
                    self.errors.at(
                        "ZRYNA-M2012",
                        span,
                        format!(
                            "call to '{}' has an argument with the wrong exact type",
                            callee.text()
                        ),
                        "match every declared i32 or bool parameter exactly",
                    );
                    return None;
                }
                let edge = CallEdge {
                    caller: self.caller,
                    callee: self.offsets[function_id.module.0 as usize]
                        + function_id.declaration as usize,
                    span: callee.span(),
                };
                if !record_call_edge(self.call_edges, edge, self.errors) {
                    return None;
                }
                self.emit(
                    result,
                    span,
                    raw::InstructionKind::DirectCall {
                        callee: function_id,
                        arguments: values.into_iter().map(|value| value.value).collect(),
                    },
                )
            }
        }
    }

    fn value(&self, id: ExpressionId) -> Option<TypedValue> {
        self.expression_values.get(id.index() as usize).copied().flatten()
    }

    fn binary_i32(
        &mut self,
        operator: Span,
        span: Span,
        lhs: ExpressionId,
        rhs: ExpressionId,
        operation: fn(raw::ValueId, raw::ValueId) -> raw::InstructionKind,
    ) -> Option<TypedValue> {
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        if lhs.ty != Type::I32 || rhs.ty != Type::I32 {
            return self.operator_error(operator, "arithmetic requires two exact i32 operands");
        }
        self.emit(Type::I32, span, operation(lhs.value, rhs.value))
    }

    fn comparison(
        &mut self,
        operator: Span,
        span: Span,
        lhs: ExpressionId,
        rhs: ExpressionId,
        operation: fn(raw::ValueId, raw::ValueId) -> raw::InstructionKind,
    ) -> Option<TypedValue> {
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        if lhs.ty != Type::I32 || rhs.ty != Type::I32 {
            return self
                .operator_error(operator, "ordered comparison requires two exact i32 operands");
        }
        self.emit(Type::Bool, span, operation(lhs.value, rhs.value))
    }

    fn equality(
        &mut self,
        operator: Span,
        span: Span,
        lhs: ExpressionId,
        rhs: ExpressionId,
        operation: fn(raw::ValueId, raw::ValueId) -> raw::InstructionKind,
    ) -> Option<TypedValue> {
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        if lhs.ty != rhs.ty || !matches!(lhs.ty, Type::I32 | Type::Bool) {
            return self.operator_error(
                operator,
                "equality requires two operands with the same exact i32 or bool type",
            );
        }
        self.emit(Type::Bool, span, operation(lhs.value, rhs.value))
    }

    fn operator_error(&mut self, span: Span, message: &'static str) -> Option<TypedValue> {
        self.errors.at(
            "ZRYNA-M2007",
            span,
            message,
            "use only the exact operand types frozen by ControlFlowV1",
        );
        None
    }

    fn emit(&mut self, ty: Type, span: Span, kind: raw::InstructionKind) -> Option<TypedValue> {
        if self.errors.is_exhausted() {
            return None;
        }
        if self.blocks[self.current_block].instructions.len() >= ir::MAX_VALUES_PER_FUNCTION {
            self.errors.limit_at(
                "ZRYNA-M2201",
                span,
                "function lowering exceeds the deterministic IR value limit",
                "reduce expressions before semantic analysis",
            );
            return None;
        }
        let value = self.new_value(ty, span)?;
        self.blocks[self.current_block].instructions.push(raw::Instruction {
            result: raw::ValueDefinition { id: value.value, ty, span },
            kind,
        });
        Some(value)
    }
}

fn draft_merge_within_limit(count: usize, span: Span, errors: &mut Errors<'_>) -> bool {
    if errors.is_exhausted() {
        return false;
    }
    if count > ir::MAX_BLOCK_PARAMETERS {
        errors.limit_at(
            "ZRYNA-M2201",
            span,
            "live mutable bindings exceed the deterministic merge/header limit",
            "reduce simultaneously live mutable bindings across control-flow edges",
        );
        return false;
    }
    true
}

fn draft_value_within_limit(next: u32, span: Span, errors: &mut Errors<'_>) -> bool {
    if errors.is_exhausted() {
        return false;
    }
    if usize::try_from(next).map_or(true, |count| count >= ir::MAX_VALUES_PER_FUNCTION) {
        errors.limit_at(
            "ZRYNA-M2201",
            span,
            "control-flow draft exceeds the deterministic IR value limit",
            "reduce expressions and live mutable state before semantic analysis",
        );
        return false;
    }
    true
}

fn store_expression_result(
    errors: &Errors<'_>,
    expression_values: &mut [Option<TypedValue>],
    index: usize,
    value: Option<TypedValue>,
) -> bool {
    if errors.is_exhausted() {
        return false;
    }
    expression_values[index] = value;
    true
}

fn expression_children(kind: &ExpressionKind) -> Vec<ExpressionId> {
    match kind {
        ExpressionKind::Reference { .. }
        | ExpressionKind::BoolLiteral { .. }
        | ExpressionKind::I32Literal { .. } => Vec::new(),
        ExpressionKind::Negation { operand, .. } => vec![*operand],
        ExpressionKind::Addition { lhs, rhs, .. }
        | ExpressionKind::Subtraction { lhs, rhs, .. }
        | ExpressionKind::Multiplication { lhs, rhs, .. }
        | ExpressionKind::Equal { lhs, rhs, .. }
        | ExpressionKind::NotEqual { lhs, rhs, .. }
        | ExpressionKind::LessThan { lhs, rhs, .. }
        | ExpressionKind::LessEqual { lhs, rhs, .. }
        | ExpressionKind::GreaterThan { lhs, rhs, .. }
        | ExpressionKind::GreaterEqual { lhs, rhs, .. } => vec![*lhs, *rhs],
        ExpressionKind::Call { arguments, .. } => arguments.clone(),
    }
}

fn instruction_operands(kind: &raw::InstructionKind) -> Vec<raw::ValueId> {
    match kind {
        raw::InstructionKind::BoolLiteral(_) | raw::InstructionKind::I32Literal(_) => Vec::new(),
        raw::InstructionKind::I32Neg { operand } => vec![*operand],
        raw::InstructionKind::I32Add { lhs, rhs }
        | raw::InstructionKind::I32Sub { lhs, rhs }
        | raw::InstructionKind::I32Mul { lhs, rhs }
        | raw::InstructionKind::Eq { lhs, rhs }
        | raw::InstructionKind::Ne { lhs, rhs }
        | raw::InstructionKind::I32LtS { lhs, rhs }
        | raw::InstructionKind::I32LeS { lhs, rhs }
        | raw::InstructionKind::I32GtS { lhs, rhs }
        | raw::InstructionKind::I32GeS { lhs, rhs } => vec![*lhs, *rhs],
        raw::InstructionKind::DirectCall { arguments, .. } => arguments.clone(),
    }
}

fn mark_live(live: &mut [bool], value: raw::ValueId) -> bool {
    let Some(slot) = live.get_mut(value.0 as usize) else { return false };
    let changed = !*slot;
    *slot = true;
    changed
}

#[allow(clippy::too_many_lines)]
fn prune_dead_block_parameters(blocks: &mut [raw::Block]) {
    let count = blocks
        .iter()
        .flat_map(|block| {
            block
                .parameters
                .iter()
                .map(|value| value.id.0)
                .chain(block.instructions.iter().map(|instruction| instruction.result.id.0))
        })
        .max()
        .map_or(0, |value| value as usize + 1);
    let mut live = vec![false; count];
    for block in blocks.iter() {
        for instruction in &block.instructions {
            for operand in instruction_operands(&instruction.kind) {
                mark_live(&mut live, operand);
            }
        }
        if let Some(terminator) = block.terminators.first() {
            match &terminator.kind {
                raw::Terminator::Return(value) => {
                    mark_live(&mut live, *value);
                }
                raw::Terminator::Jump { .. } => {}
                raw::Terminator::Branch { condition, .. } => {
                    mark_live(&mut live, *condition);
                }
            }
        }
    }
    loop {
        let mut changed = false;
        for block in blocks.iter() {
            let Some(terminator) = block.terminators.first() else { continue };
            let mut propagate = |target: raw::BlockId, arguments: &[raw::ValueId]| {
                let Some(target) = blocks.get(target.0 as usize) else { return };
                for (parameter, argument) in target.parameters.iter().zip(arguments) {
                    if live.get(parameter.id.0 as usize).copied().unwrap_or(false) {
                        changed |= mark_live(&mut live, *argument);
                    }
                }
            };
            match &terminator.kind {
                raw::Terminator::Return(_) => {}
                raw::Terminator::Jump { target, arguments } => propagate(*target, arguments),
                raw::Terminator::Branch {
                    true_target,
                    true_arguments,
                    false_target,
                    false_arguments,
                    ..
                } => {
                    propagate(*true_target, true_arguments);
                    propagate(*false_target, false_arguments);
                }
            }
        }
        if !changed {
            break;
        }
    }
    let keep = blocks
        .iter()
        .map(|block| {
            block
                .parameters
                .iter()
                .map(|parameter| live.get(parameter.id.0 as usize).copied().unwrap_or(false))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (block, mask) in blocks.iter_mut().zip(&keep) {
        let mut index = 0;
        block.parameters.retain(|_| {
            let retain = mask[index];
            index += 1;
            retain
        });
    }
    for block in blocks {
        let Some(terminator) = block.terminators.first_mut() else { continue };
        let filter = |target: raw::BlockId, arguments: &mut Vec<raw::ValueId>| {
            let mask = &keep[target.0 as usize];
            let mut index = 0;
            arguments.retain(|_| {
                let retain = mask[index];
                index += 1;
                retain
            });
        };
        match &mut terminator.kind {
            raw::Terminator::Return(_) => {}
            raw::Terminator::Jump { target, arguments } => filter(*target, arguments),
            raw::Terminator::Branch {
                true_target,
                true_arguments,
                false_target,
                false_arguments,
                ..
            } => {
                filter(*true_target, true_arguments);
                filter(*false_target, false_arguments);
            }
        }
    }
}

fn canonicalize_values(
    parameters: &mut [raw::ValueDefinition],
    blocks: &mut [raw::Block],
    errors: &mut Errors<'_>,
) -> bool {
    let count = parameters.len()
        + blocks
            .iter()
            .map(|block| block.parameters.len() + block.instructions.len())
            .sum::<usize>();
    if count > ir::MAX_VALUES_PER_FUNCTION {
        errors.limit(
            "ZRYNA-M2201",
            "control-flow lowering exceeds the deterministic IR value limit",
            "reduce expressions and live mutable state before semantic analysis",
        );
        return false;
    }
    let maximum = parameters
        .iter()
        .map(|value| value.id.0)
        .chain(blocks.iter().flat_map(|block| {
            block
                .parameters
                .iter()
                .map(|value| value.id.0)
                .chain(block.instructions.iter().map(|instruction| instruction.result.id.0))
        }))
        .max()
        .map_or(0, |value| value as usize + 1);
    let mut mapping = vec![None; maximum];
    let mut next = 0_u32;
    let mut define = |definition: &mut raw::ValueDefinition| {
        mapping[definition.id.0 as usize] = Some(raw::ValueId(next));
        definition.id = raw::ValueId(next);
        next = next.saturating_add(1);
    };
    for parameter in parameters.iter_mut() {
        define(parameter);
    }
    for block in blocks.iter_mut() {
        for parameter in &mut block.parameters {
            define(parameter);
        }
        for instruction in &mut block.instructions {
            define(&mut instruction.result);
        }
    }
    let map = |value: &mut raw::ValueId| -> bool {
        let Some(mapped) = mapping.get(value.0 as usize).copied().flatten() else { return false };
        *value = mapped;
        true
    };
    for block in blocks {
        for instruction in &mut block.instructions {
            if !rewrite_instruction_operands(&mut instruction.kind, &map) {
                errors.limit_at(
                    "ZRYNA-M2201",
                    instruction.result.span,
                    "control-flow lowering retained a value without one canonical definition",
                    "report this compiler invariant failure with the smallest source",
                );
                return false;
            }
        }
        if let Some(terminator) = block.terminators.first_mut()
            && !rewrite_terminator_operands(&mut terminator.kind, &map)
        {
            errors.limit_at(
                "ZRYNA-M2201",
                terminator.span,
                "control-flow lowering retained a terminator value without one canonical definition",
                "report this compiler invariant failure with the smallest source",
            );
            return false;
        }
    }
    true
}

fn rewrite_instruction_operands(
    kind: &mut raw::InstructionKind,
    map: &impl Fn(&mut raw::ValueId) -> bool,
) -> bool {
    match kind {
        raw::InstructionKind::BoolLiteral(_) | raw::InstructionKind::I32Literal(_) => true,
        raw::InstructionKind::I32Neg { operand } => map(operand),
        raw::InstructionKind::I32Add { lhs, rhs }
        | raw::InstructionKind::I32Sub { lhs, rhs }
        | raw::InstructionKind::I32Mul { lhs, rhs }
        | raw::InstructionKind::Eq { lhs, rhs }
        | raw::InstructionKind::Ne { lhs, rhs }
        | raw::InstructionKind::I32LtS { lhs, rhs }
        | raw::InstructionKind::I32LeS { lhs, rhs }
        | raw::InstructionKind::I32GtS { lhs, rhs }
        | raw::InstructionKind::I32GeS { lhs, rhs } => map(lhs) && map(rhs),
        raw::InstructionKind::DirectCall { arguments, .. } => arguments.iter_mut().all(map),
    }
}

fn rewrite_terminator_operands(
    kind: &mut raw::Terminator,
    map: &impl Fn(&mut raw::ValueId) -> bool,
) -> bool {
    match kind {
        raw::Terminator::Return(value) => map(value),
        raw::Terminator::Jump { arguments, .. } => arguments.iter_mut().all(map),
        raw::Terminator::Branch { condition, true_arguments, false_arguments, .. } => {
            map(condition)
                && true_arguments.iter_mut().all(map)
                && false_arguments.iter_mut().all(map)
        }
    }
}

fn record_call_edge(edges: &mut Vec<CallEdge>, edge: CallEdge, errors: &mut Errors<'_>) -> bool {
    if errors.is_exhausted() {
        return false;
    }
    if edges.len() >= MAX_CALL_EDGES {
        errors.limit(
            "ZRYNA-M2201",
            "semantic direct-call sites exceed the deterministic limit",
            "reduce direct calls before semantic analysis",
        );
        return false;
    }
    edges.push(edge);
    true
}

fn lower_type(
    syntax: &TypeSyntax,
    position: &'static str,
    errors: &mut Errors<'_>,
) -> Option<Type> {
    match syntax.kind() {
        TypeSyntaxKind::Missing => {
            errors.at(
                "ZRYNA-M2002",
                syntax.span(),
                format!("{position} type annotation is required"),
                "write an explicit i32 or bool annotation",
            );
            None
        }
        TypeSyntaxKind::Named { name } if name == "i32" => Some(Type::I32),
        TypeSyntaxKind::Named { name } if name == "bool" => Some(Type::Bool),
        TypeSyntaxKind::Named { name } => {
            errors.at(
                "ZRYNA-M2002",
                syntax.span(),
                format!("{position} type '{name}' is not supported by ControlFlowV1"),
                "use only the exact i32 or bool scalar type",
            );
            None
        }
    }
}

fn function_offsets(modules: &[ModuleSymbols]) -> Vec<usize> {
    let mut total = 0_usize;
    modules
        .iter()
        .map(|module| {
            let offset = total;
            total = total.saturating_add(module.functions.len());
            offset
        })
        .collect()
}

fn verify_call_graph(modules: &[ModuleSymbols], edges: &[CallEdge], errors: &mut Errors<'_>) {
    let count = modules.iter().map(|module| module.functions.len()).sum::<usize>();
    let mut adjacency = vec![Vec::<(usize, Span)>::new(); count];
    let mut indegree = vec![0_usize; count];
    for edge in edges {
        adjacency[edge.caller].push((edge.callee, edge.span));
        indegree[edge.callee] = indegree[edge.callee].saturating_add(1);
    }
    let mut colors = vec![0_u8; count];
    'search: for start in 0..count {
        if colors[start] != 0 {
            continue;
        }
        colors[start] = 1;
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next)) = stack.last_mut() {
            if *next == adjacency[*node].len() {
                colors[*node] = 2;
                stack.pop();
                continue;
            }
            let (target, span) = adjacency[*node][*next];
            *next += 1;
            match colors[target] {
                0 => {
                    colors[target] = 1;
                    stack.push((target, 0));
                }
                1 => {
                    errors.at(
                        "ZRYNA-M2013",
                        span,
                        "the resolved direct-call graph contains a cycle",
                        "remove direct, mutual, or cross-module recursion",
                    );
                    if errors.is_exhausted() {
                        return;
                    }
                    break 'search;
                }
                _ => {}
            }
        }
    }
    if errors.contains("ZRYNA-M2013") {
        return;
    }
    let mut queue = VecDeque::new();
    let mut depth = vec![1_usize; count];
    for (node, degree) in indegree.iter().copied().enumerate() {
        if degree == 0 {
            queue.push_back(node);
        }
    }
    while let Some(node) = queue.pop_front() {
        for &(target, span) in &adjacency[node] {
            depth[target] = depth[target].max(depth[node].saturating_add(1));
            if depth[target] > MAX_STATIC_CALL_DEPTH {
                errors.limit_at(
                    "ZRYNA-M2201",
                    span,
                    "static direct-call depth exceeds the deterministic limit of 128",
                    "reduce the acyclic direct-call chain",
                );
                return;
            }
            indegree[target] -= 1;
            if indegree[target] == 0 {
                queue.push_back(target);
            }
        }
    }
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

    fn path(
        &mut self,
        code: &'static str,
        path: &str,
        message: impl Into<String>,
        guidance: &'static str,
    ) {
        self.push(Diagnostic::error(code, Some(path.to_owned()), message, guidance));
    }

    fn global(&mut self, code: &'static str, message: impl Into<String>, guidance: &'static str) {
        self.push(Diagnostic::error(code, None, message, guidance));
    }

    fn limit(&mut self, code: &'static str, message: impl Into<String>, guidance: &'static str) {
        if !self.exhausted {
            self.diagnostics.push(Diagnostic::error(code, None, message, guidance));
            self.exhausted = true;
        }
    }

    fn limit_at(
        &mut self,
        code: &'static str,
        span: Span,
        message: impl Into<String>,
        guidance: &'static str,
    ) {
        if !self.exhausted {
            self.diagnostics.push(Diagnostic::error_at(code, span, message, guidance));
            self.exhausted = true;
        }
    }

    fn push(&mut self, diagnostic: Diagnostic) {
        if self.exhausted {
            return;
        }
        if self.diagnostics.len() < MAX_SEMANTIC_DIAGNOSTICS.saturating_sub(1) {
            self.diagnostics.push(diagnostic);
            return;
        }
        self.diagnostics.push(Diagnostic::error(
            "ZRYNA-M2201",
            None,
            format!(
                "M2 semantic analysis reached its diagnostic limit of {MAX_SEMANTIC_DIAGNOSTICS}"
            ),
            "fix the retained diagnostics before compiling again",
        ));
        self.exhausted = true;
    }

    fn contains(&self, code: &str) -> bool {
        self.diagnostics.iter().any(|diagnostic| diagnostic.code() == code)
    }

    fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        let sources = self.sources;
        self.diagnostics.sort_by(|left, right| compare_diagnostics(left, right, sources));
        self.diagnostics
    }
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic, sources: &SourceMap) -> Ordering {
    fn location_key<'a>(
        diagnostic: &'a Diagnostic,
        sources: &'a SourceMap,
    ) -> Option<(&'a str, u32, u32)> {
        if let Some(span) = diagnostic.primary_span() {
            return Some((sources.source(span.file())?.path().as_str(), span.start(), span.end()));
        }
        diagnostic.path().map(|path| (path, 0, 0))
    }
    match (location_key(left, sources), location_key(right, sources)) {
        (Some((left_path, left_start, left_end)), Some((right_path, right_start, right_end))) => (
            left_path.as_bytes(),
            left_start,
            left.code(),
            left_end,
            left.message(),
            left.guidance(),
        )
            .cmp(&(
                right_path.as_bytes(),
                right_start,
                right.code(),
                right_end,
                right.message(),
                right.guidance(),
            )),
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
mod tests {
    use std::collections::BTreeMap;

    use super::{
        CallEdge, Errors, FunctionLowerer, FunctionSymbol, MAX_CALL_EDGES,
        MAX_SEMANTIC_DIAGNOSTICS, MAX_STATIC_CALL_DEPTH, ModuleSymbols, SemanticInput, Signature,
        TypedValue, draft_merge_within_limit, draft_value_within_limit, empty_block, lower,
        lower_function, record_call_edge, run_phase, store_expression_result, verify_call_graph,
    };
    use serde_json::Value;
    use zryna_diagnostics::Diagnostic;
    use zryna_diagnostics::render_structured;
    use zryna_ir::{
        Type,
        control_flow_v1::{self as ir, VerifiedInstructionKind, VerifiedTerminatorKind, raw},
    };
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
    use zryna_syntax::v3::{decode_snapshot, verify_snapshot};

    fn sources_from_request(bytes: &[u8]) -> SourceMap {
        let request: Value = serde_json::from_slice(bytes).expect("checked-in request must decode");
        let files = request["params"]["files"]
            .as_array()
            .expect("checked-in request must contain files")
            .iter()
            .map(|file| SourceFileInput {
                path: file["path"].as_str().expect("fixture path must be text").to_owned(),
                text: file["text"].as_str().expect("fixture source must be text").to_owned(),
            })
            .collect();
        SourceMap::build(files).expect("checked-in source map must build")
    }

    fn straight_line_sources() -> SourceMap {
        sources_from_request(include_bytes!(
            "../../../tests/fixtures/m2-straight-line-request.json"
        ))
    }

    fn straight_line_snapshot(sources: &SourceMap) -> zryna_syntax::v3::ProjectSyntaxSnapshot {
        let raw =
            decode_snapshot(include_bytes!("../../../tests/fixtures/m2-straight-line-result.json"))
                .expect("checked-in v3 result must decode");
        verify_snapshot(raw, sources).expect("checked-in v3 result must verify")
    }

    fn dummy_function(
        id: raw::FunctionId,
        span: zryna_source::Span,
        parameters: &[Type],
        result: Type,
    ) -> raw::Function {
        let definitions = parameters
            .iter()
            .copied()
            .enumerate()
            .map(|(index, ty)| raw::ValueDefinition {
                id: raw::ValueId(u32::try_from(index).expect("bounded test parameter")),
                ty,
                span,
            })
            .collect::<Vec<_>>();
        let value = raw::ValueId(u32::try_from(definitions.len()).expect("bounded test value"));
        let kind = match result {
            Type::Bool => raw::InstructionKind::BoolLiteral(true),
            Type::I32 => {
                if definitions.len() == 2 {
                    raw::InstructionKind::I32Add { lhs: raw::ValueId(0), rhs: raw::ValueId(1) }
                } else {
                    raw::InstructionKind::I32Literal(0)
                }
            }
            Type::Unit => unreachable!("ControlFlowV1 test helper does not admit unit"),
        };
        raw::Function {
            id,
            entry_export: None,
            span,
            parameters: definitions,
            result,
            blocks: vec![raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions: vec![raw::Instruction {
                    result: raw::ValueDefinition { id: value, ty: result, span },
                    kind,
                }],
                terminators: vec![raw::SpannedTerminator {
                    span,
                    kind: raw::Terminator::Return(value),
                }],
            }],
        }
    }

    fn control_flow_fixture(
        request: &[u8],
        result: &[u8],
    ) -> (SourceMap, zryna_syntax::v3::ProjectSyntaxSnapshot) {
        let sources = sources_from_request(request);
        let raw = decode_snapshot(result).expect("checked-in control-flow result must decode");
        let snapshot =
            verify_snapshot(raw, &sources).expect("checked-in control-flow result must verify");
        (sources, snapshot)
    }

    fn source_text(sources: &SourceMap, span: zryna_source::Span) -> &str {
        let source = sources.source(span.file()).expect("diagnostic file must exist");
        &source.text()[span.start() as usize..span.end() as usize]
    }

    #[test]
    fn real_v3_control_flow_covers_omitted_else_returns_loops_and_nesting() {
        let (sources, snapshot) = control_flow_fixture(
            include_bytes!("../../../tests/fixtures/m2-control-flow-positive-request.json"),
            include_bytes!("../../../tests/fixtures/m2-control-flow-positive-result.json"),
        );
        let entry_path =
            NormalizedSourcePath::new("src/main.zry").expect("entry path must normalize");
        let entry = sources.file_id(&entry_path).expect("entry source must exist");
        let input = SemanticInput::try_new(&snapshot, &sources, entry)
            .expect("verified control-flow snapshot must enter M2 semantics");
        let program = lower(input).expect("positive control-flow source must lower and seal");
        let functions = program
            .modules()
            .next()
            .expect("entry module must exist")
            .functions()
            .collect::<Vec<_>>();
        assert_eq!(functions.len(), 6);

        for function in &functions[..2] {
            let blocks = function.blocks().collect::<Vec<_>>();
            assert_eq!(blocks.len(), 4, "omitted and empty else use one explicit false path");
            assert!(matches!(blocks[0].terminator().kind(), VerifiedTerminatorKind::Branch { .. }));
            assert!(matches!(blocks[1].terminator().kind(), VerifiedTerminatorKind::Jump { .. }));
            assert!(matches!(blocks[2].terminator().kind(), VerifiedTerminatorKind::Jump { .. }));
            assert_eq!(blocks[3].parameters().len(), 1, "mutable value merges once");
            assert!(matches!(blocks[3].terminator().kind(), VerifiedTerminatorKind::Return(_)));
        }

        let one_arm = functions[2].blocks().collect::<Vec<_>>();
        assert_eq!(one_arm.len(), 3);
        assert!(matches!(one_arm[0].terminator().kind(), VerifiedTerminatorKind::Branch { .. }));
        assert!(matches!(one_arm[1].terminator().kind(), VerifiedTerminatorKind::Return(_)));
        assert!(matches!(one_arm[2].terminator().kind(), VerifiedTerminatorKind::Return(_)));

        let both_arms = functions[3].blocks().collect::<Vec<_>>();
        assert_eq!(both_arms.len(), 3, "two terminal arms need no synthetic join");
        assert!(matches!(both_arms[1].terminator().kind(), VerifiedTerminatorKind::Return(_)));
        assert!(matches!(both_arms[2].terminator().kind(), VerifiedTerminatorKind::Return(_)));

        let loop_blocks = functions[4].blocks().collect::<Vec<_>>();
        assert_eq!(loop_blocks.len(), 4);
        assert!(matches!(
            loop_blocks[0].terminator().kind(),
            VerifiedTerminatorKind::Jump { target, arguments }
                if target.index() == 1 && arguments.len() == 1
        ));
        assert_eq!(loop_blocks[1].parameters().len(), 1);
        assert!(matches!(
            loop_blocks[1].terminator().kind(),
            VerifiedTerminatorKind::Branch { .. }
        ));
        assert!(matches!(
            loop_blocks[2].terminator().kind(),
            VerifiedTerminatorKind::Jump { target, arguments }
                if target.index() == 1 && arguments.len() == 1
        ));
        assert!(matches!(loop_blocks[3].terminator().kind(), VerifiedTerminatorKind::Return(_)));

        let nested = functions[5].blocks().collect::<Vec<_>>();
        assert_eq!(nested.len(), 7);
        assert_eq!(
            nested
                .iter()
                .filter(|block| matches!(
                    block.terminator().kind(),
                    VerifiedTerminatorKind::Branch { .. }
                ))
                .count(),
            2
        );
        assert!(nested.iter().any(|block| matches!(
            block.terminator().kind(),
            VerifiedTerminatorKind::Jump { target, arguments }
                if target.index() == 1 && arguments.len() == 1
        )));
        assert!(
            nested.iter().any(|block| matches!(
                block.terminator().kind(),
                VerifiedTerminatorKind::Return(_)
            ))
        );
    }

    #[test]
    fn real_v3_control_flow_rejections_are_precise_and_deterministic() {
        let (sources, snapshot) = control_flow_fixture(
            include_bytes!("../../../tests/fixtures/m2-control-flow-negative-request.json"),
            include_bytes!("../../../tests/fixtures/m2-control-flow-negative-result.json"),
        );
        let entry_path =
            NormalizedSourcePath::new("src/main.zry").expect("entry path must normalize");
        let entry = sources.file_id(&entry_path).expect("entry source must exist");
        let input = SemanticInput::try_new(&snapshot, &sources, entry)
            .expect("provider-valid negative snapshot must enter M2 semantics");
        let first = lower(input).expect_err("invalid control flow must not produce IR");
        let second = lower(input).expect_err("repeated invalid control flow must still fail");

        let conditions = first
            .iter()
            .filter(|diagnostic| diagnostic.code() == "ZRYNA-M2014")
            .collect::<Vec<_>>();
        assert_eq!(conditions.len(), 2);
        assert!(conditions.iter().all(|diagnostic| {
            source_text(&sources, diagnostic.primary_span().expect("condition span")) == "1"
        }));
        assert!(first.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-M2009"
                && diagnostic.message().contains("fall through")
                && source_text(&sources, diagnostic.primary_span().expect("function body span"))
                    .contains("while (true)")
        }));
        assert!(first.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-M2009"
                && diagnostic.message().contains("unreachable")
                && source_text(&sources, diagnostic.primary_span().expect("statement span"))
                    == "return 3;"
        }));
        assert_eq!(
            render_structured(&first, &sources).expect("first diagnostics must render"),
            render_structured(&second, &sources).expect("second diagnostics must render")
        );
    }

    #[test]
    fn live_mutable_draft_boundary_accepts_256_and_rejects_257() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("draft-boundary source map must build");
        let path = NormalizedSourcePath::new("src/main.zry").expect("path must normalize");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must resolve");

        let mut exact = Errors::new(&sources);
        assert!(draft_merge_within_limit(ir::MAX_BLOCK_PARAMETERS, span, &mut exact));
        assert!(exact.is_empty());

        let mut extra = Errors::new(&sources);
        assert!(!draft_merge_within_limit(ir::MAX_BLOCK_PARAMETERS + 1, span, &mut extra));
        let diagnostics = extra.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M2201");
    }

    #[test]
    fn provisional_value_boundary_accepts_last_identity_and_rejects_first_extra() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("draft-value source map must build");
        let path = NormalizedSourcePath::new("src/main.zry").expect("path must normalize");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must resolve");

        let mut exact = Errors::new(&sources);
        assert!(draft_value_within_limit(
            u32::try_from(ir::MAX_VALUES_PER_FUNCTION - 1).expect("IR limit fits u32"),
            span,
            &mut exact,
        ));
        assert!(exact.is_empty());

        let mut extra = Errors::new(&sources);
        assert!(!draft_value_within_limit(
            u32::try_from(ir::MAX_VALUES_PER_FUNCTION).expect("IR limit fits u32"),
            span,
            &mut extra,
        ));
        let diagnostics = extra.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M2201");
    }

    #[test]
    fn terminal_exhaustion_prevents_every_cfg_draft_mutation() {
        let (sources, snapshot) = control_flow_fixture(
            include_bytes!("../../../tests/fixtures/m2-control-flow-positive-request.json"),
            include_bytes!("../../../tests/fixtures/m2-control-flow-positive-result.json"),
        );
        let body = snapshot.files()[0].functions()[0].body();
        let mut errors = Errors::new(&sources);
        errors.limit(
            "ZRYNA-M2201",
            "fixture has already exhausted its deterministic resource",
            "reduce the fixture resource use",
        );
        let callables = BTreeMap::new();
        let modules = Vec::new();
        let offsets = Vec::new();
        let mut call_edges = Vec::new();
        let span = body.span();
        let mut lowerer = FunctionLowerer {
            body,
            callables: &callables,
            modules: &modules,
            offsets: &offsets,
            caller: 0,
            errors: &mut errors,
            call_edges: &mut call_edges,
            bindings: Vec::new(),
            expression_values: vec![None; body.expressions().len()],
            blocks: vec![empty_block(0)],
            current_block: 0,
            cfg_edges: 0,
            next_value: 0,
        };

        assert!(lowerer.new_block().is_none());
        assert!(lowerer.new_value(Type::I32, span).is_none());
        assert!(lowerer.emit(Type::I32, span, raw::InstructionKind::I32Literal(1)).is_none());
        assert!(!lowerer.terminate(raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Return(raw::ValueId(0)),
        }));
        assert_eq!(lowerer.blocks.len(), 1);
        assert!(lowerer.blocks[0].instructions.is_empty());
        assert!(lowerer.blocks[0].terminators.is_empty());
        assert_eq!(lowerer.cfg_edges, 0);
        assert_eq!(lowerer.next_value, 0);
    }

    #[test]
    fn real_v3_if_else_and_while_lower_to_one_verified_canonical_cfg() {
        let request = include_bytes!("../../../tests/fixtures/typescript-adapter-v3-request.json");
        let sources = sources_from_request(request);
        let raw_snapshot = decode_snapshot(include_bytes!(
            "../../../tests/fixtures/typescript-adapter-v3-result.json"
        ))
        .expect("checked-in control-flow snapshot must decode");
        let snapshot = verify_snapshot(raw_snapshot, &sources)
            .expect("checked-in control-flow snapshot must verify");
        let syntax = &snapshot.files()[0].functions()[1];
        let main_span = syntax.span();
        let dependency_span = snapshot.files()[1].functions()[0].span();
        let main_id = raw::FunctionId { module: raw::ModuleId(0), declaration: 0 };
        let add_id = raw::FunctionId { module: raw::ModuleId(1), declaration: 0 };
        let truth_id = raw::FunctionId { module: raw::ModuleId(1), declaration: 1 };
        let main_symbol = FunctionSymbol {
            id: main_id,
            name: "main".to_owned(),
            name_span: syntax.name().span(),
            declaration_span: main_span,
            exported: true,
            signature: Signature { parameters: vec![Some(Type::I32)], result: Some(Type::I32) },
        };
        let add_symbol = FunctionSymbol {
            id: add_id,
            name: "add".to_owned(),
            name_span: dependency_span,
            declaration_span: dependency_span,
            exported: false,
            signature: Signature {
                parameters: vec![Some(Type::I32), Some(Type::I32)],
                result: Some(Type::I32),
            },
        };
        let truth_symbol = FunctionSymbol {
            id: truth_id,
            name: "truth".to_owned(),
            name_span: dependency_span,
            declaration_span: dependency_span,
            exported: false,
            signature: Signature { parameters: Vec::new(), result: Some(Type::Bool) },
        };
        let mut callables = BTreeMap::new();
        callables.insert("add".to_owned(), add_id);
        callables.insert("truth".to_owned(), truth_id);
        let modules = vec![
            ModuleSymbols { functions: vec![main_symbol.clone()], ..ModuleSymbols::default() },
            ModuleSymbols { functions: vec![add_symbol, truth_symbol], ..ModuleSymbols::default() },
        ];
        let mut errors = Errors::new(&sources);
        let mut call_edges = Vec::new();
        let main = lower_function(
            syntax,
            &main_symbol,
            &callables,
            &modules,
            &[0, 1],
            &mut call_edges,
            &mut errors,
            true,
        )
        .expect("real v3 control flow must lower");
        assert!(errors.is_empty());
        assert_eq!(main.blocks.len(), 7);
        assert!(matches!(main.blocks[0].terminators[0].kind, raw::Terminator::Branch { .. }));
        assert!(matches!(main.blocks[1].terminators[0].kind, raw::Terminator::Jump { .. }));
        assert!(matches!(main.blocks[2].terminators[0].kind, raw::Terminator::Jump { .. }));
        assert_eq!(main.blocks[3].parameters.len(), 1, "if merge carries mutable y");
        assert!(matches!(main.blocks[3].terminators[0].kind, raw::Terminator::Jump { .. }));
        assert_eq!(main.blocks[4].parameters.len(), 1, "loop header carries mutable y");
        assert!(matches!(main.blocks[4].terminators[0].kind, raw::Terminator::Branch { .. }));
        assert!(matches!(
            main.blocks[5].terminators[0].kind,
            raw::Terminator::Jump { target: raw::BlockId(4), .. }
        ));
        assert!(matches!(main.blocks[6].terminators[0].kind, raw::Terminator::Return(_)));

        let add = dummy_function(add_id, dependency_span, &[Type::I32, Type::I32], Type::I32);
        let truth = dummy_function(truth_id, dependency_span, &[], Type::Bool);
        let entry = sources.file_id(snapshot.files()[0].path()).expect("entry file");
        let program = ir::verify(
            raw::Program {
                entry_module: raw::ModuleId(0),
                modules: vec![
                    raw::Module { id: raw::ModuleId(0), source_file: entry, functions: vec![main] },
                    raw::Module {
                        id: raw::ModuleId(1),
                        source_file: snapshot.files()[1].id(),
                        functions: vec![add, truth],
                    },
                ],
            },
            &sources,
            entry,
        )
        .expect("lowered real-v3 CFG must pass the sealed verifier");
        assert_eq!(program.modules().next().expect("entry module").functions().count(), 1);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn real_v3_source_lowers_every_straight_line_operation_and_call_in_order() {
        let sources = straight_line_sources();
        let syntax = straight_line_snapshot(&sources);
        let entry_path =
            NormalizedSourcePath::new("src/main.zry").expect("entry path must normalize");
        let entry = sources.file_id(&entry_path).expect("entry source must exist");
        let input = SemanticInput::try_new(&syntax, &sources, entry)
            .expect("verified final snapshot must enter M2 semantics");
        let program = lower(input).expect("straight-line M2 fixture must lower and verify");

        assert_eq!(program.modules().len(), 2);
        assert_eq!(program.scalar_abi().exports().len(), 1);
        let entry_module = program.modules().next().expect("entry module must exist");
        let functions = entry_module.functions().collect::<Vec<_>>();
        let forward = functions[4];
        let evaluate = functions[6];
        assert!(forward.blocks().next().expect("forward block must exist").instructions().any(
            |instruction| matches!(
                instruction.kind(),
                VerifiedInstructionKind::DirectCall { callee, .. }
                    if callee.module().index() == 0 && callee.declaration() == 5
            )
        ));
        assert_eq!(
            evaluate.parameters().map(|(_, ty, _)| ty).collect::<Vec<_>>(),
            [Type::I32, Type::I32]
        );
        assert_eq!(evaluate.result(), Type::Bool);
        let block = evaluate.blocks().next().expect("straight-line entry block must exist");
        assert!(matches!(block.terminator().kind(), VerifiedTerminatorKind::Return(_)));

        let instructions = block.instructions().collect::<Vec<_>>();
        assert!(instructions.iter().any(|instruction| matches!(
            instruction.kind(),
            VerifiedInstructionKind::I32Literal(i32::MIN)
        )));
        assert!(instructions.iter().any(|instruction| matches!(
            instruction.kind(),
            VerifiedInstructionKind::I32Literal(i32::MAX)
        )));
        let twice_result = instructions
            .iter()
            .copied()
            .find(|instruction| {
                matches!(
                    instruction.kind(),
                    VerifiedInstructionKind::DirectCall { callee, .. }
                        if callee.module().index() == 1 && callee.declaration() == 0
                )
            })
            .expect("cross-module twice call must exist")
            .result();
        let mut calls = Vec::new();
        let mut join_first_argument = None;
        let mut kinds = Vec::new();
        for instruction in instructions {
            let kind = instruction.kind();
            kinds.push(std::mem::discriminant(&kind));
            if let VerifiedInstructionKind::DirectCall { callee, arguments } = kind {
                calls.push((callee.module().index(), callee.declaration()));
                if callee.module().index() == 0 && callee.declaration() == 0 {
                    join_first_argument = arguments.iter().next();
                }
            }
        }
        assert_eq!(
            calls,
            [(1, 0), (0, 0), (1, 1), (0, 1), (0, 2), (0, 3), (1, 0), (0, 4)],
            "calls and nested arguments must lower left-to-right exactly once"
        );
        assert_eq!(
            join_first_argument,
            Some(twice_result),
            "nested shadowing must restore the outer mutable binding before assignment"
        );
        assert!(
            kinds.iter().any(|kind| *kind
                == std::mem::discriminant(&VerifiedInstructionKind::I32Neg(twice_result)))
        );
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32Add(
                twice_result,
                twice_result
            ))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32Sub(
                twice_result,
                twice_result
            ))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32Mul(
                twice_result,
                twice_result
            ))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::Eq(twice_result, twice_result))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::Ne(twice_result, twice_result))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32LtS(
                twice_result,
                twice_result
            ))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32LeS(
                twice_result,
                twice_result
            ))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32GtS(
                twice_result,
                twice_result
            ))));
        assert!(kinds.iter().any(|kind| *kind
            == std::mem::discriminant(&VerifiedInstructionKind::I32GeS(
                twice_result,
                twice_result
            ))));
    }

    #[test]
    fn m2_input_rejects_an_independently_built_source_map() {
        let sources = straight_line_sources();
        let syntax = straight_line_snapshot(&sources);
        let other = straight_line_sources();
        let entry_path =
            NormalizedSourcePath::new("src/main.zry").expect("entry path must normalize");
        let other_entry = other.file_id(&entry_path).expect("other entry must exist");

        assert!(SemanticInput::try_new(&syntax, &other, other_entry).is_none());
    }

    #[test]
    fn semantic_rejections_are_source_owned_bounded_and_deterministic() {
        let sources = sources_from_request(include_bytes!(
            "../../../tests/fixtures/m2-semantic-negative-request.json"
        ));
        let raw = decode_snapshot(include_bytes!(
            "../../../tests/fixtures/m2-semantic-negative-result.json"
        ))
        .expect("negative v3 result must decode");
        let syntax = verify_snapshot(raw, &sources).expect("negative v3 result must verify");
        let entry_path =
            NormalizedSourcePath::new("src/main.zry").expect("entry path must normalize");
        let entry = sources.file_id(&entry_path).expect("entry source must exist");
        let input = SemanticInput::try_new(&syntax, &sources, entry)
            .expect("provider-valid syntax must enter M2 semantics");
        let first = lower(input).expect_err("invalid source must not produce verified IR");
        let second = lower(input).expect_err("repeated invalid source must still fail");
        let codes = first.iter().map(Diagnostic::code).collect::<Vec<_>>();

        for expected in [
            "ZRYNA-M2001",
            "ZRYNA-M2002",
            "ZRYNA-M2003",
            "ZRYNA-M2004",
            "ZRYNA-M2005",
            "ZRYNA-M2006",
            "ZRYNA-M2007",
            "ZRYNA-M2008",
            "ZRYNA-M2009",
            "ZRYNA-M2010",
            "ZRYNA-M2011",
            "ZRYNA-M2012",
            "ZRYNA-M2013",
            "ZRYNA-M2022",
        ] {
            assert!(codes.contains(&expected), "missing semantic rejection {expected}: {codes:?}");
        }
        let first_rendered = render_structured(&first, &sources)
            .expect("source-owned semantic diagnostics must render");
        let second_rendered = render_structured(&second, &sources)
            .expect("repeated source-owned diagnostics must render");
        assert_eq!(first_rendered, second_rendered);
        assert!(first.len() <= MAX_SEMANTIC_DIAGNOSTICS);
        assert!(first.iter().all(|diagnostic| !diagnostic.code().starts_with("ZRYNA-I2")));
        assert!(first.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-M2010"
                && diagnostic.message().contains("import graph contains a cycle")
                && diagnostic.primary_span().is_some()
        }));
    }

    #[test]
    fn diagnostic_budget_reserves_one_exact_terminal_m2_error() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("diagnostic source map must build");
        let path = NormalizedSourcePath::new("src/main.zry").expect("path must normalize");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must resolve");
        let mut errors = Errors::new(&sources);
        for index in 0..(MAX_SEMANTIC_DIAGNOSTICS + 32) {
            errors.push(Diagnostic::error_at(
                "ZRYNA-M2099",
                span,
                format!("fixture semantic error {index}"),
                "fix the fixture",
            ));
        }
        let diagnostics = errors.finish();

        assert_eq!(diagnostics.len(), MAX_SEMANTIC_DIAGNOSTICS);
        assert_eq!(
            diagnostics.iter().filter(|diagnostic| diagnostic.code() == "ZRYNA-M2099").count(),
            MAX_SEMANTIC_DIAGNOSTICS - 1
        );
        assert_eq!(
            diagnostics.iter().filter(|diagnostic| diagnostic.code() == "ZRYNA-M2201").count(),
            1
        );
    }

    #[test]
    fn terminal_exhaustion_stops_all_later_semantic_phases() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("phase-gate source map must build");
        let mut errors = Errors::new(&sources);
        let first = run_phase(&mut errors, |errors| {
            errors.limit(
                "ZRYNA-M2201",
                "fixture phase exhausted its bounded semantic resource",
                "reduce the fixture resource use",
            );
        });
        let mut later_phase_ran = false;
        let later = run_phase(&mut errors, |_| {
            later_phase_ran = true;
        });

        assert!(first.is_none(), "the exhausting phase must close immediately");
        assert!(later.is_none(), "a later semantic phase must not be entered");
        assert!(!later_phase_ran, "terminal exhaustion must prevent later phase actions");
    }

    #[test]
    fn terminal_exhaustion_cannot_commit_a_per_root_expression_result() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("expression-commit source map must build");
        let mut errors = Errors::new(&sources);
        errors.limit(
            "ZRYNA-M2201",
            "fixture expression exhausted its bounded semantic resource",
            "reduce the fixture resource use",
        );
        let mut values = [None];

        assert!(!store_expression_result(
            &errors,
            &mut values,
            0,
            Some(TypedValue { ty: Type::I32, value: raw::ValueId(0) }),
        ));
        assert!(values[0].is_none(), "no per-root expression value may survive exhaustion");
    }

    #[test]
    fn one_semantic_failure_is_retained_once() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("diagnostic source map must build");
        let path = NormalizedSourcePath::new("src/main.zry").expect("path must normalize");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must resolve");
        let mut errors = Errors::new(&sources);

        errors.at("ZRYNA-M2099", span, "one failure", "fix the fixture");

        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M2099");
    }

    #[test]
    fn call_site_budget_accepts_exact_and_first_extra_is_one_terminal() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("call-budget source map must build");
        let path = NormalizedSourcePath::new("src/main.zry").expect("path must normalize");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must resolve");
        let edge = CallEdge { caller: 0, callee: 1, span };
        let mut blocked_edges = Vec::new();
        let mut blocked_errors = Errors::new(&sources);
        blocked_errors.limit(
            "ZRYNA-M2201",
            "fixture exhausted before call-edge recording",
            "reduce the fixture resource use",
        );
        assert!(!record_call_edge(&mut blocked_edges, edge, &mut blocked_errors));
        assert!(blocked_edges.is_empty(), "terminal exhaustion must prevent call-edge commits");

        let mut edges = Vec::with_capacity(MAX_CALL_EDGES);
        let mut errors = Errors::new(&sources);

        for _ in 0..MAX_CALL_EDGES {
            assert!(record_call_edge(&mut edges, edge, &mut errors));
        }
        assert!(errors.is_empty());
        assert!(!record_call_edge(&mut edges, edge, &mut errors));
        assert!(!record_call_edge(&mut edges, edge, &mut errors));

        let diagnostics = errors.finish();
        assert_eq!(edges.len(), MAX_CALL_EDGES);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M2201");
    }

    fn call_graph_module(count: usize, span: zryna_source::Span) -> ModuleSymbols {
        ModuleSymbols {
            functions: (0..count)
                .map(|declaration| FunctionSymbol {
                    id: raw::FunctionId {
                        module: raw::ModuleId(0),
                        declaration: u32::try_from(declaration)
                            .expect("test declaration must fit u32"),
                    },
                    name: format!("function_{declaration}"),
                    name_span: span,
                    declaration_span: span,
                    exported: false,
                    signature: Signature { parameters: Vec::new(), result: Some(Type::I32) },
                })
                .collect(),
            ..ModuleSymbols::default()
        }
    }

    #[test]
    fn static_call_depth_accepts_128_and_rejects_129() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("call-depth source map must build");
        let path = NormalizedSourcePath::new("src/main.zry").expect("path must normalize");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must resolve");

        for (count, expected_errors) in
            [(MAX_STATIC_CALL_DEPTH, 0_usize), (MAX_STATIC_CALL_DEPTH + 1, 1_usize)]
        {
            let modules = vec![call_graph_module(count, span)];
            let edges = (0..count.saturating_sub(1))
                .map(|caller| CallEdge { caller, callee: caller + 1, span })
                .collect::<Vec<_>>();
            let mut errors = Errors::new(&sources);
            verify_call_graph(&modules, &edges, &mut errors);
            let diagnostics = errors.finish();
            assert_eq!(diagnostics.len(), expected_errors);
            assert!(diagnostics.iter().all(|diagnostic| diagnostic.code() == "ZRYNA-M2201"));
        }
    }

    #[test]
    fn diagnostics_share_one_portable_path_start_code_order() {
        let sources = SourceMap::build(vec![
            SourceFileInput { path: "z.zry".to_owned(), text: "z".to_owned() },
            SourceFileInput { path: "a.zry".to_owned(), text: "a".to_owned() },
        ])
        .expect("ordering source map must build");
        let z_path = NormalizedSourcePath::new("z.zry").expect("path must normalize");
        let z_file = sources.file_id(&z_path).expect("z source must exist");
        let z_span = sources.span(z_file, 0, 1).expect("z span must resolve");
        let mut errors = Errors::new(&sources);
        errors.global("ZRYNA-M2099", "global", "fix global");
        errors.at("ZRYNA-M2002", z_span, "source", "fix source");
        errors.path("ZRYNA-M2010", "a.zry", "path", "fix path");

        let diagnostics = errors.finish();
        assert_eq!(diagnostics[0].path(), Some("a.zry"));
        assert_eq!(diagnostics[1].primary_span(), Some(z_span));
        assert_eq!(diagnostics[2].path(), None);
        assert_eq!(diagnostics[2].primary_span(), None);
    }
}
