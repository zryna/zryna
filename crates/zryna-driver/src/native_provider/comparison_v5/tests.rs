use super::*;
use zryna_frontend::{ProviderExpectationV5, WorkerLimitsV5, WorkerSpecV5};
use zryna_source::SourceFileInput;
fn sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function entry(): i32 { return 7; }".into(),
    }])
    .expect("genuine source map")
}
fn syntax(sources: &SourceMap, warning: bool) -> syntax_v5::VerifiedProjectSyntaxV5 {
    let lexed = zryna_frontend::native_lexer::lex(sources).expect("actual lexer");
    let mut raw = zryna_frontend::native_parser::v5::parse_v5_candidate(sources, &lexed)
        .expect("actual current parser");
    if warning {
        raw.diagnostics.push(zryna_syntax::v4::RawProviderDiagnostic {
            code: "fixture-advisory".into(),
            severity: zryna_diagnostics::Severity::Warning,
            location: zryna_syntax::v4::RawDiagnosticLocation::Global,
            message: "different advisory".into(),
            guidance: "retain exact observation".into(),
        });
    }
    syntax_v5::verify_snapshot(raw, sources).expect("complete actual source correspondence")
}
#[test]
fn complete_comparison_rejects_foreign_same_bytes_and_advisory_only_difference() {
    let source = sources();
    let first = syntax(&source, false);
    let second = syntax(&source, false);
    assert!(equal(&source, first, &second).expect("exact complete match").is_bound_to(&source));
    let foreign = sources();
    assert!(equal(&source, syntax(&source, false), &syntax(&foreign, false)).is_err());
    assert!(equal(&source, syntax(&source, false), &syntax(&source, true)).is_err());
}
#[test]
fn configured_identity_and_physical_work_fail_before_process_creation() {
    let root = std::env::temp_dir();
    let worker = WorkerFrontendV5::new(
        WorkerSpecV5::new(
            root.join("never-executed-v5-comparison"),
            vec![],
            root,
            ProviderExpectationV5::new("typescript-6", "6.0.3").expect("bootstrap identity"),
            WorkerLimitsV5::default(),
        )
        .expect("typed worker"),
    );
    let comparison = ComparisonFrontendV5::new(worker.clone(), worker);
    assert_eq!(comparison.analysis_work_units(), 2);
    assert_eq!(comparison.minimum_analysis_timeout(), MIN_WORKER_TIMEOUT * 2);
    assert!(comparison.analyze_verified_v5(&sources()).is_err());
}
