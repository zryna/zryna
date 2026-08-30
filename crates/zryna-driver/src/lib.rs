//! Zryna compiler-phase orchestration.
//!
//! Legacy protocol-v1 syntax cannot enter the protocol-v2 semantic boundary:
//!
//! ```compile_fail
//! fn bypass(
//!     legacy: &zryna_frontend::ProjectSyntaxSnapshot,
//!     sources: &zryna_source::SourceMap,
//! ) {
//!     let _ = zryna_semantics::SemanticInput::try_new(legacy, sources);
//! }
//! ```

#![forbid(unsafe_code)]

use std::{error::Error, fmt, path::Path};

use zryna_architecture::ValidationReport;
use zryna_backend_javascript::JavaScriptArtifact;
use zryna_backend_native::LlvmIrArtifact;
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::VerifiedProgram;
use zryna_source::SourceMap;

/// Artifacts emitted by the first verified dual-target slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DualTargetArtifacts {
    /// Direct ECMAScript output.
    pub javascript: JavaScriptArtifact,
    /// Textual LLVM IR validating the native backend boundary.
    pub llvm_ir: LlvmIrArtifact,
}

/// Successful source analysis with its verified IR and non-fatal provider diagnostics.
#[derive(Clone, Debug)]
pub struct SourceToIrSuccess {
    program: VerifiedProgram,
    diagnostics: Vec<Diagnostic>,
}

impl SourceToIrSuccess {
    /// Returns the backend-safe verified program.
    #[must_use]
    pub const fn program(&self) -> &VerifiedProgram {
        &self.program
    }

    /// Consumes the result and returns the backend-safe verified program.
    #[must_use]
    pub fn into_program(self) -> VerifiedProgram {
        self.program
    }

    /// Returns deterministic non-fatal provider diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Failure before source can become backend-safe verified IR.
#[derive(Debug)]
pub enum SourceToIrError {
    /// The authenticated frontend worker failed before returning verified syntax.
    Frontend(zryna_frontend::WorkerError),
    /// Provider, semantic, or IR diagnostics rejected the source.
    Rejected(Vec<Diagnostic>),
}

impl SourceToIrError {
    /// Returns source diagnostics when a compiler phase rejected the program.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Frontend(error) => error.diagnostics(),
            Self::Rejected(diagnostics) => diagnostics,
        }
    }
}

impl fmt::Display for SourceToIrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frontend(error) => error.fmt(formatter),
            Self::Rejected(diagnostics) => write!(
                formatter,
                "source was rejected by {} deterministic diagnostic(s)",
                diagnostics.len()
            ),
        }
    }
}

impl Error for SourceToIrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frontend(error) => Some(error),
            Self::Rejected(_) => None,
        }
    }
}

/// Runs the mandatory workspace gate.
#[must_use]
pub fn check_workspace(root: &Path) -> ValidationReport {
    zryna_architecture::validate_workspace(root)
}

/// Runs one authenticated frontend worker and returns only source-map-verified syntax.
///
/// # Errors
///
/// Returns a deterministic worker failure before untrusted syntax can enter later phases.
pub fn analyze_sources<Provider: zryna_frontend::VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
) -> Result<zryna_frontend::syntax_v2::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
    frontend.analyze_verified(sources)
}

/// Lowers one source-map-bound protocol-v2 snapshot and runs the mandatory IR verifier.
///
/// Provider errors stop before semantic analysis. Provider warnings remain observable on success.
///
/// # Errors
///
/// Returns deterministic diagnostics and never exposes raw IR when semantic or IR verification
/// fails.
pub fn lower_verified_syntax(
    syntax: &zryna_frontend::syntax_v2::ProjectSyntaxSnapshot,
    sources: &SourceMap,
) -> Result<SourceToIrSuccess, Vec<Diagnostic>> {
    if !syntax.is_bound_to(sources) {
        return Err(vec![Diagnostic::error(
            "ZRYNA-D1001",
            None,
            "verified syntax is not bound to the driver's authoritative source map",
            "analyze and lower with the same immutable source map instance",
        )]);
    }
    if syntax.diagnostics().iter().any(|diagnostic| diagnostic.severity() == Severity::Error) {
        return Err(syntax.diagnostics().to_vec());
    }
    let Some(input) = zryna_semantics::SemanticInput::try_new(syntax, sources) else {
        return Err(vec![Diagnostic::error(
            "ZRYNA-D1002",
            None,
            "verified syntax could not enter semantic analysis",
            "report this compiler invariant failure with the smallest reproducible source",
        )]);
    };
    let program = zryna_semantics::lower(input)?;
    let program = zryna_ir::verify(program, sources)?;
    let diagnostics = syntax
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Warning)
        .cloned()
        .collect();
    Ok(SourceToIrSuccess { program, diagnostics })
}

/// Authenticates a frontend, analyzes real source, lowers strict semantics, and verifies IR.
///
/// # Errors
///
/// Returns the exact frontend failure or bounded rejection diagnostics. No backend can observe an
/// unverified program through this path.
pub fn compile_to_verified_ir<Provider: zryna_frontend::VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
) -> Result<SourceToIrSuccess, SourceToIrError> {
    let syntax = analyze_sources(frontend, sources).map_err(SourceToIrError::Frontend)?;
    lower_verified_syntax(&syntax, sources).map_err(SourceToIrError::Rejected)
}

/// Emits both current backend artifacts from one verified program.
///
/// Raw Universal IR cannot enter driver emission:
///
/// ```compile_fail
/// let raw = zryna_ir::Program::default();
/// let _ = zryna_driver::emit_verified(&raw);
/// ```
///
/// # Errors
///
/// Returns bounded diagnostics when native MIR verification or either target emission fails.
pub fn emit_verified(program: &VerifiedProgram) -> Result<DualTargetArtifacts, Vec<Diagnostic>> {
    let javascript = zryna_backend_javascript::emit(program).map_err(|error| vec![error])?;
    let mir = zryna_native_mir::lower(program)?;
    let llvm_ir = zryna_backend_native::emit_llvm_ir(&mir).map_err(|error| vec![error])?;
    Ok(DualTargetArtifacts { javascript, llvm_ir })
}

#[cfg(test)]
mod tests {
    use std::{env, ffi::OsString, path::PathBuf};

    use super::{SourceToIrError, compile_to_verified_ir, lower_verified_syntax};
    use zryna_diagnostics::{Diagnostic, Severity, render_structured};
    use zryna_frontend::{
        FrontendCapabilities, ProviderExpectation, WorkerFrontend, WorkerLimits, WorkerSpec,
        syntax_v2::{self, RawDiagnosticLocation, RawProviderDiagnostic},
    };
    use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type, verify};
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};

    fn node_executable() -> PathBuf {
        for variable in ["ZRYNA_TEST_NODE", "NODE"] {
            if let Some(configured) = env::var_os(variable) {
                let configured = PathBuf::from(configured);
                assert!(
                    configured.is_absolute() && configured.is_file(),
                    "configured Node.js executable must be an existing absolute file"
                );
                return configured;
            }
        }
        let executable = if cfg!(windows) { "node.exe" } else { "node" };
        env::split_paths(&env::var_os("PATH").expect("test PATH must exist"))
            .map(|directory| directory.join(executable))
            .find(|candidate| candidate.is_file())
            .expect("Node.js must be installed for the source-to-IR integration suite")
    }

    fn typescript_frontend() -> WorkerFrontend {
        let adapter_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/typescript-6");
        assert!(
            adapter_root.is_absolute() && adapter_root.is_dir(),
            "TypeScript adapter root must be an existing absolute directory"
        );
        let expected = ProviderExpectation::new(
            "typescript-6",
            "6.0.3",
            syntax_v2::PROTOCOL_VERSION,
            FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
        )
        .expect("provider expectation must be valid");
        let spec = WorkerSpec::new(
            node_executable(),
            vec![OsString::from("src/worker.mjs")],
            adapter_root,
            expected,
            WorkerLimits::default(),
        )
        .expect("worker specification must be valid");
        WorkerFrontend::new(spec)
    }

    fn source_map(path: &str, text: &str) -> SourceMap {
        SourceMap::build(vec![SourceFileInput { path: path.to_owned(), text: text.to_owned() }])
            .expect("fixture source map must build")
    }

    fn byte_offset(text: &str, needle: &str) -> u32 {
        u32::try_from(text.find(needle).expect("fixture token must exist"))
            .expect("fixture offset must fit u32")
    }

    fn last_byte_offset(text: &str, needle: &str) -> u32 {
        u32::try_from(text.rfind(needle).expect("fixture token must exist"))
            .expect("fixture offset must fit u32")
    }

    fn rejection_codes(text: &str) -> (SourceMap, Vec<String>) {
        let sources = source_map("src/main.zry", text);
        let error = compile_to_verified_ir(&typescript_frontend(), &sources)
            .expect_err("fixture source must be rejected");
        let SourceToIrError::Rejected(diagnostics) = error else {
            panic!("fixture must reach a compiler rejection, not a worker failure");
        };
        let codes = diagnostics.iter().map(|diagnostic| diagnostic.code().to_owned()).collect();
        (sources, codes)
    }

    #[test]
    fn real_universal_add_source_reaches_verified_ir_through_protocol_v2() {
        let text = include_str!("../../../examples/universal/add.zry");
        let sources = source_map("examples/universal/add.zry", text);

        let result = compile_to_verified_ir(&typescript_frontend(), &sources)
            .expect("universal add must reach verified IR");

        assert!(result.diagnostics().is_empty());
        let functions = result.program().functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let function = functions[0];
        assert_eq!(function.export_name().as_str(), "add");
        assert_eq!(function.parameters(), &[Type::I32, Type::I32]);
        assert_eq!(function.return_type(), Type::I32);
        assert_eq!(function.body(), ExprId(2));
        assert_eq!(function.expressions().len(), 3);
        assert_eq!(function.expressions()[0].kind, ExprKind::Parameter(0));
        assert_eq!(function.expressions()[1].kind, ExprKind::Parameter(1));
        assert_eq!(
            function.expressions()[2].kind,
            ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) }
        );
        for expression in function.expressions() {
            assert!(sources.resolve(expression.span).is_ok());
        }
    }

    #[test]
    fn i32_boundaries_and_repeated_nested_additions_lower_exactly() {
        let text = concat!(
            "export function min(): i32 { return -2147483648; } ",
            "export function max(): i32 { return 2147483647; } ",
            "export function zero(): i32 { return 0; } ",
            "export function repeated(a: i32): i32 { return a + a; } ",
            "export function nested(a: i32, b: i32): i32 { return a + b + a; }",
        );
        let sources = source_map("src/boundaries.zry", text);

        let result = compile_to_verified_ir(&typescript_frontend(), &sources)
            .expect("supported i32 boundary fixture must reach verified IR");
        let functions = result.program().functions().collect::<Vec<_>>();

        assert_eq!(functions.len(), 5);
        assert_eq!(functions[0].expressions()[0].kind, ExprKind::I32Literal(i32::MIN));
        assert_eq!(functions[1].expressions()[0].kind, ExprKind::I32Literal(i32::MAX));
        assert_eq!(functions[2].expressions()[0].kind, ExprKind::I32Literal(0));
        assert_eq!(functions[3].expressions().len(), 3);
        assert_eq!(
            functions[3].expressions()[2].kind,
            ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) }
        );
        assert_eq!(functions[4].expressions().len(), 5);
        assert_eq!(
            functions[4].expressions()[2].kind,
            ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) }
        );
        assert_eq!(
            functions[4].expressions()[4].kind,
            ExprKind::I32Add { lhs: ExprId(2), rhs: ExprId(3) }
        );
    }

    #[test]
    fn provider_warnings_remain_observable_on_verified_success() {
        let text = include_str!("../../../examples/universal/add.zry");
        let sources = source_map("examples/universal/add.zry", text);
        let mut raw = syntax_v2::decode_snapshot(include_bytes!(
            "../../../tests/fixtures/typescript-adapter-v2-result.json"
        ))
        .expect("checked adapter fixture must decode");
        raw.diagnostics.push(RawProviderDiagnostic {
            code: "TS9000".to_owned(),
            severity: Severity::Warning,
            location: RawDiagnosticLocation::Global,
            message: "provider note".to_owned(),
            guidance: "review the declaration".to_owned(),
        });
        let syntax = syntax_v2::verify_snapshot(raw, &sources)
            .expect("source-bound provider warning must verify");

        let result = lower_verified_syntax(&syntax, &sources)
            .expect("provider warning must not block verified IR");

        assert_eq!(result.program().functions().count(), 1);
        assert_eq!(result.diagnostics().len(), 1);
        assert_eq!(result.diagnostics()[0].code(), "TS9000");
        assert_eq!(result.diagnostics()[0].severity(), Severity::Warning);
    }

    #[test]
    fn source_map_identity_precedes_provider_diagnostic_forwarding() {
        let text = "export function yes(): bool { return true; }";
        let first = source_map("src/main.zry", text);
        let second = source_map("src/main.zry", text);
        let raw = syntax_v2::decode_snapshot(include_bytes!(
            "../../../tests/fixtures/typescript-adapter-v2-error-result.json"
        ))
        .expect("provider error fixture must decode");
        let syntax = syntax_v2::verify_snapshot(raw, &first)
            .expect("provider error fixture must verify against its issuing map");

        let diagnostics = lower_verified_syntax(&syntax, &second)
            .expect_err("cross-map syntax must fail before forwarding provider diagnostics");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-D1001");
        assert!(diagnostics[0].primary_span().is_none());
    }

    #[test]
    fn exact_parameter_limit_reaches_verified_ir() {
        let parameters = (0..syntax_v2::MAX_PARAMETERS_PER_FUNCTION)
            .map(|index| format!("p{index}: i32"))
            .collect::<Vec<_>>()
            .join(", ");
        let text = format!("export function exact({parameters}): i32 {{ return p0; }}");
        let sources = source_map("src/exact-parameters.zry", &text);

        let result = compile_to_verified_ir(&typescript_frontend(), &sources)
            .expect("the exact syntax, ABI, and IR parameter limit must succeed");
        let function = result.program().functions().next().expect("function must exist");

        assert_eq!(function.parameters().len(), syntax_v2::MAX_PARAMETERS_PER_FUNCTION);
        assert_eq!(function.expressions().len(), 1);
        assert_eq!(function.expressions()[0].kind, ExprKind::Parameter(0));
    }

    #[test]
    fn semantic_rejections_are_stable_and_source_located() {
        let cases = [
            ("export function f(a): i32 { return a; }", "ZRYNA-M1003"),
            ("export function f(a: i32) { return a; }", "ZRYNA-M1003"),
            ("export function f(a: any): i32 { return 1; }", "ZRYNA-M1004"),
            ("export function f(): any { return 1; }", "ZRYNA-M1004"),
            ("export function f(a: u32): i32 { return 1; }", "ZRYNA-M1005"),
            ("export function f(a: i32, a: i32): i32 { return a; }", "ZRYNA-M1002"),
            ("export function f(a: i32): i32 { return b; }", "ZRYNA-M1006"),
            ("export function f(): i32 { return 2147483648; }", "ZRYNA-M1007"),
            ("export function f(): i32 { return -2147483649; }", "ZRYNA-M1007"),
            ("export function f(): i32 { return true + 1; }", "ZRYNA-M1008"),
            ("export function f(): i32 { return true; }", "ZRYNA-M1010"),
            ("export function f(): bool { return 1; }", "ZRYNA-M1010"),
            ("export function f(): i32 {}", "ZRYNA-M1009"),
            ("export function f(): i32 { return 1; return 2; }", "ZRYNA-M1009"),
            ("export function then(): i32 { return 1; }", "ZRYNA-M1011"),
            ("export function $value(): i32 { return 1; }", "ZRYNA-M1011"),
            (
                "export function Value(): i32 { return 1; } export function value(): i32 { return 2; }",
                "ZRYNA-M1012",
            ),
            (
                "export function value(): i32 { return 1; } export function value(): i32 { return 2; }",
                "ZRYNA-M1001",
            ),
        ];

        for (text, expected) in cases {
            let sources = source_map("src/main.zry", text);
            let error = compile_to_verified_ir(&typescript_frontend(), &sources)
                .expect_err("invalid source must not produce verified IR");
            let SourceToIrError::Rejected(diagnostics) = error else {
                panic!("invalid source must reach deterministic compiler diagnostics");
            };
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic.code() == expected),
                "expected {expected} for {text:?}, got {:?}",
                diagnostics.iter().map(Diagnostic::code).collect::<Vec<_>>()
            );
            let report = render_structured(&diagnostics, &sources)
                .expect("all source diagnostics must resolve through the authoritative map");
            let repeated_error = compile_to_verified_ir(&typescript_frontend(), &sources)
                .expect_err("invalid source must remain rejected on repeated analysis");
            let SourceToIrError::Rejected(repeated_diagnostics) = repeated_error else {
                panic!("repeated invalid source must reach compiler diagnostics");
            };
            let repeated_report = render_structured(&repeated_diagnostics, &sources)
                .expect("repeated source diagnostics must remain renderable");
            assert_eq!(report, repeated_report, "diagnostics changed for {text:?}");
            let diagnostic = report
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == expected)
                .expect("expected diagnostic must render");
            assert_eq!(diagnostic.path.as_deref(), Some("src/main.zry"));
            assert!(diagnostic.byte_start.is_some());
            assert!(diagnostic.byte_end.is_some());
        }
    }

    #[test]
    fn multi_error_diagnostic_order_and_spans_are_exact() {
        let text = "export function f(a: any): i32 { return b + true; return 2147483648; }";
        let sources = source_map("src/exact-errors.zry", text);
        let render = || {
            let error = compile_to_verified_ir(&typescript_frontend(), &sources)
                .expect_err("multi-error fixture must be rejected");
            let SourceToIrError::Rejected(diagnostics) = error else {
                panic!("multi-error fixture must reach compiler diagnostics");
            };
            render_structured(&diagnostics, &sources).expect("diagnostics must render")
        };

        let first = render();
        let second = render();
        assert_eq!(first, second);

        let any_start = byte_offset(text, "any");
        let reference_start = byte_offset(text, "b +");
        let operator_start = byte_offset(text, "+ true");
        let second_return_start = last_byte_offset(text, "return");
        let literal_start = byte_offset(text, "2147483648");
        let second_statement_end =
            last_byte_offset(text, ";").checked_add(1).expect("fixture statement end must fit");
        let actual = first
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code.as_str(),
                    diagnostic.byte_start.expect("source start must exist"),
                    diagnostic.byte_end.expect("source end must exist"),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                ("ZRYNA-M1004", any_start, any_start + 3),
                ("ZRYNA-M1006", reference_start, reference_start + 1),
                ("ZRYNA-M1008", operator_start, operator_start + 1),
                ("ZRYNA-M1009", second_return_start, second_statement_end),
                ("ZRYNA-M1007", literal_start, literal_start + 10),
            ]
        );
    }

    #[test]
    fn entrypoint_and_provider_failures_stop_before_verified_ir() {
        let no_sources = SourceMap::build(Vec::new()).expect("empty source map must build");
        let no_source_error = compile_to_verified_ir(&typescript_frontend(), &no_sources)
            .expect_err("missing entrypoint must fail");
        let SourceToIrError::Rejected(no_source_diagnostics) = no_source_error else {
            panic!("missing entrypoint must reach semantic diagnostics");
        };
        assert_eq!(
            no_source_diagnostics.iter().map(Diagnostic::code).collect::<Vec<_>>(),
            vec!["ZRYNA-M1013"]
        );
        assert!(no_source_diagnostics[0].primary_span().is_none());

        let empty_sources = source_map("src/empty.zry", "");
        let empty_error = compile_to_verified_ir(&typescript_frontend(), &empty_sources)
            .expect_err("empty entrypoint must fail");
        assert!(empty_error.diagnostics().iter().any(|item| item.code() == "ZRYNA-M1014"));

        let first = "export function a(): i32 { return 1; }";
        let second = "export function b(): i32 { return 2; }";
        let multiple_sources = SourceMap::build(vec![
            SourceFileInput { path: "src/a.zry".to_owned(), text: first.to_owned() },
            SourceFileInput { path: "src/b.zry".to_owned(), text: second.to_owned() },
        ])
        .expect("multi-file fixture must build");
        let multiple_error = compile_to_verified_ir(&typescript_frontend(), &multiple_sources)
            .expect_err("modules must remain disabled");
        assert!(multiple_error.diagnostics().iter().any(|item| item.code() == "ZRYNA-M1013"));

        let unsupported = source_map("src/main.zry", "export class Unsupported {}");
        let provider_error = compile_to_verified_ir(&typescript_frontend(), &unsupported)
            .expect_err("unsupported provider syntax must stop compilation");
        assert!(provider_error.diagnostics().iter().any(|item| item.code() == "ZRYNA-F2002"));

        let parenthesized = source_map(
            "src/main.zry",
            "export function nested(a: i32, b: i32): i32 { return a + (b + a); }",
        );
        let parenthesized_error = compile_to_verified_ir(&typescript_frontend(), &parenthesized)
            .expect_err("parenthesized expressions are outside protocol v2");
        assert!(parenthesized_error.diagnostics().iter().any(|item| item.code() == "ZRYNA-F2002"));
    }

    #[test]
    fn bool_source_has_a_stable_verified_profile_rejection() {
        for text in [
            "export function yes(): bool { return true; }",
            "export function no(): bool { return false; }",
            "export function identity(value: bool): bool { return value; }",
        ] {
            let (_, codes) = rejection_codes(text);
            assert!(codes.iter().any(|code| code == "ZRYNA-I1006"), "codes: {codes:?}");
        }
    }

    #[test]
    fn one_verified_program_drives_both_backend_boundaries() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/add.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/add.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
        let program = Program {
            functions: vec![
                Function {
                    name: "add".to_owned(),
                    parameters: vec![Type::I32, Type::I32],
                    return_type: Type::I32,
                    expressions: vec![
                        Expr { ty: Type::I32, span, kind: ExprKind::Parameter(0) },
                        Expr { ty: Type::I32, span, kind: ExprKind::Parameter(1) },
                        Expr {
                            ty: Type::I32,
                            span,
                            kind: ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) },
                        },
                    ],
                    body: ExprId(2),
                },
                Function {
                    name: "answer".to_owned(),
                    parameters: Vec::new(),
                    return_type: Type::I32,
                    expressions: vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(42) }],
                    body: ExprId(0),
                },
            ],
        };
        let verified = verify(program, &sources).expect("fixture IR must verify");

        let artifacts = super::emit_verified(&verified).expect("both backends must emit");

        assert_eq!(
            artifacts.javascript.source,
            "export function add(p0, p1) {\n  const v0 = p0;\n  const v1 = p1;\n  const v2 = (v0 + v1) | 0;\n  return v2;\n}\nexport function answer() {\n  const v0 = 42;\n  return v0;\n}\n"
        );
        assert_eq!(
            artifacts.llvm_ir.source,
            "define i32 @add(i32 %p0, i32 %p1) {\nentry:\n  %v2 = add i32 %p0, %p1\n  ret i32 %v2\n}\ndefine i32 @answer() {\nentry:\n  %v0 = add i32 0, 42\n  ret i32 %v0\n}\n"
        );
    }
}
