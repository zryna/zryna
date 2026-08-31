//! Cross-platform proof tests for the retained deterministic M2 module closure.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::path::Path;

use crate::{
    MAX_MODULE_DISCOVERY_WALL_TIME, MAX_MODULE_EDGE_MANIFEST_BYTES, MAX_MODULE_FILES,
    MAX_MODULE_IMPORT_DECLARATIONS, MAX_MODULE_PROVIDER_SOURCE_BYTES, MAX_MODULE_SOURCE_BYTES,
    ModuleClosureError, WorkspaceSourceRoot, discover_module_closure,
    module_closure::{account_edge_manifest_budget, discover_module_closure_with_clock},
};
use zryna_diagnostics::Severity;
use zryna_frontend::{VerifiedFrontendProviderV3, WorkerError, syntax_v3};
use zryna_source::{NormalizedSourcePath, SourceMap, UntrustedSpan};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProviderCall {
    paths: Vec<String>,
    source_bytes: usize,
}

type CallHook = Arc<dyn Fn(usize) + Send + Sync>;

struct FixtureProvider {
    calls: Mutex<Vec<ProviderCall>>,
    omit_imports_on_call: Option<usize>,
    error_on_call: Option<usize>,
    hook: Option<CallHook>,
}

impl FixtureProvider {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: None,
            hook: None,
        }
    }

    fn omitting_imports_on(call: usize) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: Some(call),
            error_on_call: None,
            hook: None,
        }
    }

    fn reporting_error_on(call: usize) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: Some(call),
            hook: None,
        }
    }

    #[cfg(unix)]
    fn with_hook(call: usize, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: None,
            hook: Some(Arc::new(move |current| {
                if current == call {
                    hook();
                }
            })),
        }
    }

    #[cfg(unix)]
    fn reporting_error_with_hook(call: usize, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: Some(call),
            hook: Some(Arc::new(move |current| {
                if current == call {
                    hook();
                }
            })),
        }
    }

    fn calls(&self) -> Vec<ProviderCall> {
        self.calls.lock().expect("provider call log must remain available").clone()
    }
}

impl VerifiedFrontendProviderV3 for FixtureProvider {
    fn analyze_verified_v3(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v3::ProjectSyntaxSnapshot, WorkerError> {
        let mut paths = Vec::with_capacity(sources.len());
        let mut source_bytes = 0_usize;
        for index in 0..sources.len() {
            let raw = u32::try_from(index).expect("bounded fixture file id");
            let id = sources.verify_file_id(raw).expect("fixture file id must be valid");
            let source = sources.source(id).expect("fixture source must exist");
            paths.push(source.path().as_str().to_owned());
            source_bytes = source_bytes
                .checked_add(source.text().len())
                .expect("fixture source accounting must fit");
        }
        let mut calls = self.calls.lock().expect("provider call log must remain available");
        calls.push(ProviderCall { paths, source_bytes });
        let call = calls.len();
        drop(calls);
        if let Some(hook) = &self.hook {
            hook(call);
        }
        let omit_imports = self.omit_imports_on_call == Some(call);
        let mut raw = raw_snapshot(sources, omit_imports);
        if self.error_on_call == Some(call) {
            raw.diagnostics.push(syntax_v3::RawProviderDiagnostic {
                code: "TS1000".to_owned(),
                severity: Severity::Error,
                location: syntax_v3::RawDiagnosticLocation::Global,
                message: "fixture provider error".to_owned(),
                guidance: "fix the fixture source".to_owned(),
            });
        }
        let verified = syntax_v3::verify_snapshot(raw, sources)
            .expect("fixture provider must construct valid exact-v3 syntax");
        Ok(verified)
    }
}

struct TemporaryWorkspace {
    path: PathBuf,
}

impl TemporaryWorkspace {
    fn new(label: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("zryna-module-closure-{}-{label}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("unique module workspace must be created");
        Self { path }
    }

    #[cfg(unix)]
    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, path: &str, source: &str) {
        let destination = self.path.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("module parent must be created");
        }
        fs::write(destination, source).expect("module source must be written");
    }

    fn root(&self) -> WorkspaceSourceRoot {
        WorkspaceSourceRoot::capture(&self.path).expect("fixture workspace must be captured")
    }
}

impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn raw_snapshot(sources: &SourceMap, omit_imports: bool) -> syntax_v3::RawProjectSyntaxSnapshot {
    let mut files = Vec::with_capacity(sources.len());
    for index in 0..sources.len() {
        let raw_id = u32::try_from(index).expect("bounded fixture file id");
        let id = sources.verify_file_id(raw_id).expect("fixture file id must be valid");
        let source = sources.source(id).expect("fixture source must exist");
        let imports = if omit_imports { Vec::new() } else { parse_imports(raw_id, source.text()) };
        files.push(syntax_v3::RawSourceUnit {
            id: raw_id,
            path: source.path().as_str().to_owned(),
            imports,
            functions: Vec::new(),
        });
    }
    syntax_v3::RawProjectSyntaxSnapshot {
        schema_version: syntax_v3::PROTOCOL_VERSION,
        files,
        diagnostics: Vec::new(),
    }
}

fn parse_imports(file: u32, source: &str) -> Vec<syntax_v3::RawImportSyntax> {
    let mut imports = Vec::new();
    let mut offset = 0_usize;
    for line in source.split_inclusive('\n') {
        if line.starts_with("import { ") {
            imports.push(parse_import(file, source, offset, line));
        }
        offset = offset.checked_add(line.len()).expect("fixture offset must fit");
    }
    imports
}

fn parse_import(
    file: u32,
    source: &str,
    line_start: usize,
    line: &str,
) -> syntax_v3::RawImportSyntax {
    let close_relative = line.find(" } from ").expect("fixture import must contain from");
    let from_relative = close_relative.checked_add(3).expect("fixture offset must fit");
    let quote_relative = line[from_relative..]
        .find('"')
        .map(|value| value + from_relative)
        .expect("fixture import must contain quote");
    let quote_end_relative = line[quote_relative + 1..]
        .find('"')
        .map(|value| value + quote_relative + 1)
        .expect("fixture import must close quote");
    let semicolon_relative = line[quote_end_relative + 1..]
        .find(';')
        .map(|value| value + quote_end_relative + 1)
        .expect("fixture import must contain semicolon");
    let binding_start = "import { ".len();
    let mut binding_relative = binding_start;
    let mut bindings = Vec::new();
    for binding in line[binding_start..close_relative].split(", ") {
        let as_in_binding = binding.find(" as ").expect("fixture import must contain as");
        let imported_start = line_start + binding_relative;
        let imported_end = imported_start + as_in_binding;
        let as_start = imported_end + 1;
        let as_end = as_start + 2;
        let local_start = imported_start + as_in_binding + " as ".len();
        let local_end = imported_start + binding.len();
        bindings.push(syntax_v3::RawImportBindingSyntax {
            span: span(file, imported_start, local_end),
            imported: syntax_v3::RawIdentifierSyntax {
                text: source[imported_start..imported_end].to_owned(),
                span: span(file, imported_start, imported_end),
            },
            local: syntax_v3::RawIdentifierSyntax {
                text: source[local_start..local_end].to_owned(),
                span: span(file, local_start, local_end),
            },
            as_span: Some(span(file, as_start, as_end)),
        });
        binding_relative += binding.len() + ", ".len();
    }
    let from_start = line_start + from_relative;
    let quote_start = line_start + quote_relative;
    let quote_end = line_start + quote_end_relative + 1;
    let value_start = quote_start + 1;
    let value_end = quote_end - 1;
    let semicolon = line_start + semicolon_relative;
    let specifier = source[value_start..value_end].to_owned();
    syntax_v3::RawImportSyntax {
        span: span(file, line_start, semicolon + 1),
        import_span: span(file, line_start, line_start + "import".len()),
        bindings,
        from_span: span(file, from_start, from_start + "from".len()),
        specifier: syntax_v3::RawModuleSpecifierSyntax {
            text: specifier,
            token_span: span(file, quote_start, quote_end),
            value_span: span(file, value_start, value_end),
        },
        semicolon_span: span(file, semicolon, semicolon + 1),
    }
}

fn span(file: u32, start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file,
        start: u32::try_from(start).expect("fixture span start must fit"),
        end: u32::try_from(end).expect("fixture span end must fit"),
    }
}

fn entry() -> NormalizedSourcePath {
    NormalizedSourcePath::new("main.zry").expect("fixture entry path")
}

fn rejection_code(error: ModuleClosureError) -> String {
    match error {
        ModuleClosureError::Rejected(diagnostics) => {
            diagnostics.first().expect("rejection must contain one diagnostic").code().to_owned()
        }
        ModuleClosureError::Frontend(error) => error.code().to_owned(),
    }
}

fn diamond_sources() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            "main.zry",
            concat!(
                "import { left as leftValue } from \"./lib/left.zry\";\n",
                "import { right as rightValue } from \"./lib/right.zry\";\n",
            ),
        ),
        ("lib/left.zry", "import { leaf as leftLeaf } from \"./leaf.zry\";\n"),
        ("lib/right.zry", "import { leaf as rightLeaf } from \"./leaf.zry\";\n"),
        ("lib/leaf.zry", ""),
    ])
}

fn padded_source(prefix: &str, bytes: usize) -> String {
    assert!(prefix.len() <= bytes);
    let mut source = String::with_capacity(bytes);
    source.push_str(prefix);
    source.extend(std::iter::repeat_n('x', bytes - prefix.len()));
    source
}

fn module_path(index: usize) -> String {
    if index == 0 { "main.zry".to_owned() } else { format!("m{index:04}.zry") }
}

fn write_declaration_budget_graph(workspace: &TemporaryWorkspace, plus_one: bool) {
    const POPULATED_MODULES: usize = 16;
    const DECLARATIONS_PER_MODULE: usize = 4_096;

    for module in 0..POPULATED_MODULES {
        let mut source = String::new();
        for declaration in 0..DECLARATIONS_PER_MODULE {
            let target = if declaration == 0
                && (module + 1 < POPULATED_MODULES || (plus_one && module + 1 == POPULATED_MODULES))
            {
                format!("./m{:04}.zry", module + 1)
            } else {
                "./leaf.zry".to_owned()
            };
            writeln!(
                &mut source,
                "import {{ value as local{module}_{declaration} }} from \"{target}\";"
            )
            .expect("fixture source write must succeed");
        }
        workspace.write(&module_path(module), &source);
    }
    if plus_one {
        workspace.write(
            &module_path(POPULATED_MODULES),
            "import { value as extra } from \"./leaf.zry\";\n",
        );
    }
    workspace.write("leaf.zry", "");
}

fn scripted_clock(times: Vec<Instant>) -> impl FnMut() -> Instant {
    let fallback = *times.last().expect("scripted clock needs one instant");
    let mut times = times.into_iter();
    move || times.next().unwrap_or(fallback)
}

#[test]
fn aggregate_discovery_deadline_is_checked_before_and_after_frontend_calls_without_sleeping() {
    let workspace = TemporaryWorkspace::new("aggregate-deadline");
    workspace.write("main.zry", "");
    let started = Instant::now();
    let expired = started + MAX_MODULE_DISCOVERY_WALL_TIME + Duration::from_nanos(1);

    let before_provider = FixtureProvider::new();
    let before = discover_module_closure_with_clock(
        &workspace.root(),
        entry(),
        &before_provider,
        scripted_clock(vec![started, expired]),
    )
    .expect_err("expired discovery must stop before starting the provider");
    assert_eq!(rejection_code(before), "ZRYNA-D3201");
    assert!(before_provider.calls().is_empty());

    let after_provider = FixtureProvider::new();
    let after = discover_module_closure_with_clock(
        &workspace.root(),
        entry(),
        &after_provider,
        scripted_clock(vec![started, started, expired]),
    )
    .expect_err("provider completion beyond the aggregate deadline must fail");
    assert_eq!(rejection_code(after), "ZRYNA-D3201");
    assert_eq!(after_provider.calls().len(), 1);

    let exact_provider = FixtureProvider::new();
    let exact = started + MAX_MODULE_DISCOVERY_WALL_TIME;
    discover_module_closure_with_clock(
        &workspace.root(),
        entry(),
        &exact_provider,
        scripted_clock(vec![started, exact]),
    )
    .expect("the exact aggregate deadline remains accepted");
    assert_eq!(exact_provider.calls().len(), 2);
}

#[test]
fn discovers_one_canonical_linear_batched_diamond() {
    let first = TemporaryWorkspace::new("diamond-first");
    for (path, source) in diamond_sources() {
        first.write(path, source);
    }
    let first_provider = FixtureProvider::new();
    let first_closure = discover_module_closure(&first.root(), entry(), &first_provider)
        .expect("diamond closure must succeed");

    assert!(first_closure.syntax().is_bound_to(first_closure.sources()));
    assert_eq!(
        first_closure.modules().iter().map(|module| module.path().as_str()).collect::<Vec<_>>(),
        vec!["lib/leaf.zry", "lib/left.zry", "lib/right.zry", "main.zry"]
    );
    assert_eq!(first_closure.edges().len(), 4);
    assert_eq!(
        first_closure.graph_sha256().iter().fold(String::new(), |mut hex, byte| {
            write!(&mut hex, "{byte:02x}").expect("digest formatting must succeed");
            hex
        }),
        "42e0cac9bdcf12832c6ad03ee20cd01fa4f4892536325b154cf5df4e902a0e26"
    );
    let calls = first_provider.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[0].paths, vec!["main.zry"]);
    assert_eq!(calls[1].paths, vec!["lib/left.zry", "lib/right.zry"]);
    assert_eq!(calls[2].paths, vec!["lib/leaf.zry"]);
    assert_eq!(calls[3].paths, vec!["lib/leaf.zry", "lib/left.zry", "lib/right.zry", "main.zry"]);
    let total_source_bytes = diamond_sources().values().map(|source| source.len()).sum::<usize>();
    assert_eq!(calls.iter().map(|call| call.source_bytes).sum::<usize>(), 2 * total_source_bytes);
    assert!(2 * total_source_bytes <= MAX_MODULE_PROVIDER_SOURCE_BYTES);

    let second = TemporaryWorkspace::new("diamond-second");
    for (path, source) in diamond_sources().into_iter().rev() {
        second.write(path, source);
    }
    let second_closure = discover_module_closure(&second.root(), entry(), &FixtureProvider::new())
        .expect("reordered diamond closure must succeed");
    assert_eq!(first_closure.graph_sha256(), second_closure.graph_sha256());
    assert_eq!(first_closure.modules(), second_closure.modules());
    assert_eq!(first_closure.edges().len(), second_closure.edges().len());
}

#[test]
fn final_closure_enters_verifier_sealed_straight_line_semantics() {
    let workspace = TemporaryWorkspace::new("semantic-boundary");
    workspace.write("main.zry", "");
    let closure = discover_module_closure(&workspace.root(), entry(), &FixtureProvider::new())
        .expect("complete fixture closure must verify");

    let program = closure
        .lower_control_flow_v1()
        .expect("empty-function closure is a valid internal M2 program");

    assert_eq!(program.modules().len(), 1);
    assert_eq!(program.scalar_abi().exports().len(), 0);
}

#[test]
fn rejects_missing_wrong_case_duplicate_cycle_escape_and_final_drift() {
    let missing = TemporaryWorkspace::new("missing");
    missing.write("main.zry", "import { value as local } from \"./missing.zry\";\n");
    assert_eq!(
        rejection_code(
            discover_module_closure(&missing.root(), entry(), &FixtureProvider::new())
                .expect_err("missing dependency must fail")
        ),
        "ZRYNA-D3003"
    );

    let wrong_case = TemporaryWorkspace::new("wrong-case");
    wrong_case.write("main.zry", "import { value as local } from \"./dep.zry\";\n");
    wrong_case.write("Dep.zry", "");
    assert_eq!(
        rejection_code(
            discover_module_closure(&wrong_case.root(), entry(), &FixtureProvider::new())
                .expect_err("wrong-case dependency must fail")
        ),
        "ZRYNA-D3005"
    );

    let duplicate = TemporaryWorkspace::new("duplicate");
    duplicate.write(
        "main.zry",
        concat!(
            "import { value as local } from \"./dep.zry\";\n",
            "import { value as local } from \"./dep.zry\";\n",
        ),
    );
    duplicate.write("dep.zry", "");
    assert_eq!(
        rejection_code(
            discover_module_closure(&duplicate.root(), entry(), &FixtureProvider::new())
                .expect_err("duplicate binding edge must fail")
        ),
        "ZRYNA-D3006"
    );

    let cycle = TemporaryWorkspace::new("cycle");
    cycle.write("main.zry", "import { a as localA } from \"./a.zry\";\n");
    cycle.write("a.zry", "import { main as localMain } from \"./main.zry\";\n");
    assert_eq!(
        rejection_code(
            discover_module_closure(&cycle.root(), entry(), &FixtureProvider::new())
                .expect_err("module cycle must fail")
        ),
        "ZRYNA-D3007"
    );

    let escape = TemporaryWorkspace::new("escape");
    escape.write("main.zry", "import { value as local } from \"../outside.zry\";\n");
    assert_eq!(
        rejection_code(
            discover_module_closure(&escape.root(), entry(), &FixtureProvider::new())
                .expect_err("root escape must fail")
        ),
        "ZRYNA-D3001"
    );

    let drift = TemporaryWorkspace::new("drift");
    drift.write("main.zry", "import { value as local } from \"./dep.zry\";\n");
    drift.write("dep.zry", "");
    assert_eq!(
        rejection_code(
            discover_module_closure(
                &drift.root(),
                entry(),
                &FixtureProvider::omitting_imports_on(3),
            )
            .expect_err("final provider drift must fail")
        ),
        "ZRYNA-D3102"
    );
}

#[test]
fn provider_error_stops_before_resolving_or_reading_another_source() {
    let workspace = TemporaryWorkspace::new("provider-error");
    workspace.write("main.zry", "import { value as local } from \"./missing.zry\";\n");
    let provider = FixtureProvider::reporting_error_on(1);
    assert_eq!(
        rejection_code(
            discover_module_closure(&workspace.root(), entry(), &provider)
                .expect_err("provider error must stop discovery immediately")
        ),
        "ZRYNA-D3101"
    );
    assert_eq!(provider.calls().len(), 1);
}

#[cfg(unix)]
#[test]
fn final_provider_error_precedes_post_call_filesystem_revalidation() {
    let workspace = TemporaryWorkspace::new("final-provider-error");
    workspace.write("main.zry", "original\n");
    let source = workspace.path().join("main.zry");
    let provider = FixtureProvider::reporting_error_with_hook(2, move || {
        fs::write(&source, "modified\n").expect("final provider hook must mutate the source");
    });
    assert_eq!(
        rejection_code(
            discover_module_closure(&workspace.root(), entry(), &provider)
                .expect_err("final provider error must win before filesystem revalidation")
        ),
        "ZRYNA-D3101"
    );
    assert_eq!(provider.calls().len(), 2);
}

#[test]
fn import_declaration_budget_accepts_exact_and_rejects_first_extra_declaration() {
    let exact = TemporaryWorkspace::new("exact-import-declarations");
    write_declaration_budget_graph(&exact, false);
    let closure = discover_module_closure(&exact.root(), entry(), &FixtureProvider::new())
        .expect("exact aggregate import-declaration limit must succeed");
    assert_eq!(closure.edges().len(), MAX_MODULE_IMPORT_DECLARATIONS);

    let plus_one = TemporaryWorkspace::new("plus-one-import-declaration");
    write_declaration_budget_graph(&plus_one, true);
    assert_eq!(
        rejection_code(
            discover_module_closure(&plus_one.root(), entry(), &FixtureProvider::new())
                .expect_err("first extra import declaration must fail")
        ),
        "ZRYNA-D3201"
    );
}

#[test]
fn aggregate_and_file_budgets_accept_exact_and_reject_one_more() {
    let exact_bytes = TemporaryWorkspace::new("exact-bytes");
    let imports = concat!(
        "import { a as localA } from \"./a.zry\";\n",
        "import { b as localB } from \"./b.zry\";\n",
        "import { c as localC } from \"./c.zry\";\n",
    );
    exact_bytes.write("main.zry", &padded_source(imports, MAX_MODULE_SOURCE_BYTES / 4));
    for dependency in ["a.zry", "b.zry", "c.zry"] {
        exact_bytes.write(dependency, &padded_source("", MAX_MODULE_SOURCE_BYTES / 4));
    }
    let provider = FixtureProvider::new();
    let closure = discover_module_closure(&exact_bytes.root(), entry(), &provider)
        .expect("exact aggregate source budget must succeed");
    assert_eq!(closure.modules().len(), 4);
    assert_eq!(
        provider.calls().iter().map(|call| call.source_bytes).sum::<usize>(),
        MAX_MODULE_PROVIDER_SOURCE_BYTES
    );

    let plus_one_byte = TemporaryWorkspace::new("plus-one-byte");
    let imports = concat!(
        "import { a as localA } from \"./a.zry\";\n",
        "import { b as localB } from \"./b.zry\";\n",
        "import { c as localC } from \"./c.zry\";\n",
        "import { d as localD } from \"./d.zry\";\n",
    );
    plus_one_byte.write("main.zry", &padded_source(imports, MAX_MODULE_SOURCE_BYTES / 4));
    for dependency in ["a.zry", "b.zry", "c.zry"] {
        plus_one_byte.write(dependency, &padded_source("", MAX_MODULE_SOURCE_BYTES / 4));
    }
    plus_one_byte.write("d.zry", "x");
    assert_eq!(
        rejection_code(
            discover_module_closure(&plus_one_byte.root(), entry(), &FixtureProvider::new())
                .expect_err("aggregate source budget plus one must fail")
        ),
        "ZRYNA-D3201"
    );

    let exact_files = TemporaryWorkspace::new("exact-files");
    let mut exact_imports = String::new();
    for index in 1..MAX_MODULE_FILES {
        let path = format!("m{index:04}.zry");
        writeln!(&mut exact_imports, "import {{ value as local{index} }} from \"./{path}\";")
            .expect("fixture source write must succeed");
        exact_files.write(&path, "");
    }
    exact_files.write("main.zry", &exact_imports);
    let closure = discover_module_closure(&exact_files.root(), entry(), &FixtureProvider::new())
        .expect("exact module file budget must succeed");
    assert_eq!(closure.modules().len(), MAX_MODULE_FILES);

    let plus_one_file = TemporaryWorkspace::new("plus-one-file");
    let mut too_many_imports = String::new();
    for index in 1..=MAX_MODULE_FILES {
        writeln!(
            &mut too_many_imports,
            "import {{ value as local{index} }} from \"./n{index:04}.zry\";"
        )
        .expect("fixture source write must succeed");
    }
    plus_one_file.write("main.zry", &too_many_imports);
    assert_eq!(
        rejection_code(
            discover_module_closure(&plus_one_file.root(), entry(), &FixtureProvider::new())
                .expect_err("module file budget plus one must fail before dependency reads")
        ),
        "ZRYNA-D3201"
    );
}

#[test]
fn one_import_chain_is_linear_at_exact_round_and_call_limits() {
    fn write_chain(workspace: &TemporaryWorkspace, extra_edge: bool) {
        workspace.write("main.zry", "import { value as local0 } from \"./c0001.zry\";\n");
        for index in 1..MAX_MODULE_FILES {
            let path = format!("c{index:04}.zry");
            let source = if index + 1 < MAX_MODULE_FILES || extra_edge {
                format!("import {{ value as local{index} }} from \"./c{:04}.zry\";\n", index + 1)
            } else {
                String::new()
            };
            workspace.write(&path, &source);
        }
    }

    let exact = TemporaryWorkspace::new("exact-chain");
    write_chain(&exact, false);
    let provider = FixtureProvider::new();
    let closure = discover_module_closure(&exact.root(), entry(), &provider)
        .expect("exact one-import chain must close");
    assert_eq!(closure.modules().len(), MAX_MODULE_FILES);
    assert_eq!(provider.calls().len(), MAX_MODULE_FILES + 1);
    let discovery_bytes =
        provider.calls().iter().take(MAX_MODULE_FILES).map(|call| call.source_bytes).sum::<usize>();
    assert_eq!(
        provider.calls().last().expect("final call must exist").source_bytes,
        discovery_bytes
    );

    let plus_one = TemporaryWorkspace::new("plus-one-chain");
    write_chain(&plus_one, true);
    assert_eq!(
        rejection_code(
            discover_module_closure(&plus_one.root(), entry(), &FixtureProvider::new())
                .expect_err("round limit plus one must fail before another source read")
        ),
        "ZRYNA-D3201"
    );
}

#[cfg(unix)]
#[test]
fn rejects_case_collisions_links_and_source_mutation_during_final_analysis() {
    use std::os::unix::fs::symlink;

    let collision = TemporaryWorkspace::new("case-collision");
    collision.write("main.zry", "import { value as local } from \"./dep.zry\";\n");
    collision.write("dep.zry", "");
    collision.write("Dep.zry", "");
    assert_eq!(
        rejection_code(
            discover_module_closure(&collision.root(), entry(), &FixtureProvider::new())
                .expect_err("portable case collision must fail")
        ),
        "ZRYNA-D3005"
    );

    let linked = TemporaryWorkspace::new("linked");
    linked.write("main.zry", "import { value as local } from \"./linked/dep.zry\";\n");
    fs::create_dir(linked.path().join("real")).expect("real dependency directory");
    linked.write("real/dep.zry", "");
    symlink("real", linked.path().join("linked")).expect("directory link must be created");
    assert_eq!(
        rejection_code(
            discover_module_closure(&linked.root(), entry(), &FixtureProvider::new())
                .expect_err("linked source component must fail")
        ),
        "ZRYNA-D3002"
    );

    let changed = TemporaryWorkspace::new("changed-final");
    changed.write("main.zry", "import { value as local } from \"./dep.zry\";\n");
    changed.write("dep.zry", "original\n");
    let dependency = changed.path().join("dep.zry");
    let provider = FixtureProvider::with_hook(3, move || {
        fs::write(&dependency, "modified\n").expect("source mutation hook must write");
    });
    assert_eq!(
        rejection_code(
            discover_module_closure(&changed.root(), entry(), &provider)
                .expect_err("source mutation during final analysis must fail")
        ),
        "ZRYNA-D3004"
    );
}

#[test]
fn canonical_edge_manifest_budget_accepts_exact_limit_and_rejects_plus_one() {
    let mut exact = 0;
    account_edge_manifest_budget(&mut exact, MAX_MODULE_EDGE_MANIFEST_BYTES)
        .expect("the exact edge-manifest budget must be accepted");
    assert_eq!(exact, MAX_MODULE_EDGE_MANIFEST_BYTES);

    let mut plus_one = 0;
    let error = account_edge_manifest_budget(&mut plus_one, MAX_MODULE_EDGE_MANIFEST_BYTES + 1)
        .expect_err("one byte beyond the edge-manifest budget must be rejected");
    assert_eq!(rejection_code(error), "ZRYNA-D3201");
}
