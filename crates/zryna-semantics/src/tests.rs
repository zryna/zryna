use super::{MAX_SEMANTIC_DIAGNOSTICS, SemanticErrors, SemanticInput, lower};
use zryna_diagnostics::{Diagnostic, render_structured};
use zryna_ir::{ExprKind, Type, verify};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v2::{
    PROTOCOL_VERSION, RawProjectSyntaxSnapshot, decode_snapshot, verify_snapshot,
};

fn sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: "export function yes(): bool { return true; }".to_owned(),
    }])
    .expect("fixture source map must build")
}

#[test]
fn semantic_input_rejects_a_different_source_map_instance() {
    let sources = sources();
    let raw = decode_snapshot(include_bytes!("../../../tests/fixtures/syntax-v2-valid.json"))
        .expect("checked-in protocol fixture must decode");
    let syntax = verify_snapshot(raw, &sources).expect("checked-in fixture must verify");

    assert!(SemanticInput::try_new(&syntax, &sources).is_some());
    assert!(SemanticInput::try_new(&syntax, &self::sources()).is_none());
}

#[test]
fn semantic_input_rejects_a_different_empty_source_map_instance() {
    let first = SourceMap::build(Vec::new()).expect("empty source map must build");
    let second = SourceMap::build(Vec::new()).expect("second empty source map must build");
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: Vec::new(),
        diagnostics: Vec::new(),
    };
    let syntax = verify_snapshot(raw, &first).expect("empty snapshot must verify structurally");

    assert!(SemanticInput::try_new(&syntax, &first).is_some());
    assert!(SemanticInput::try_new(&syntax, &second).is_none());
}

#[test]
fn semantic_input_rejects_provider_errors() {
    let sources = sources();
    let raw = decode_snapshot(include_bytes!(
        "../../../tests/fixtures/typescript-adapter-v2-error-result.json"
    ))
    .expect("error fixture must decode");
    let syntax = verify_snapshot(raw, &sources).expect("provider error must remain verifiable");

    assert!(SemanticInput::try_new(&syntax, &sources).is_none());
}

#[test]
fn semantic_input_accepts_provider_warnings() {
    let sources = sources();
    let raw = decode_snapshot(include_bytes!(
        "../../../tests/fixtures/typescript-adapter-v2-warning-result.json"
    ))
    .expect("warning fixture must decode");
    let syntax = verify_snapshot(raw, &sources).expect("provider warning fixture must verify");

    assert!(SemanticInput::try_new(&syntax, &sources).is_some());
}

#[test]
fn bool_source_lowers_deterministically_but_remains_profile_gated() {
    let sources = sources();
    let raw = decode_snapshot(include_bytes!("../../../tests/fixtures/syntax-v2-valid.json"))
        .expect("checked-in protocol fixture must decode");
    let syntax = verify_snapshot(raw, &sources).expect("checked-in fixture must verify");
    let input = SemanticInput::try_new(&syntax, &sources).expect("fixture must enter semantics");

    let program = lower(input).expect("bool is valid source semantics");

    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].return_type, Type::Bool);
    assert_eq!(program.functions[0].expressions.len(), 1);
    assert_eq!(program.functions[0].expressions[0].ty, Type::Bool);
    assert_eq!(program.functions[0].expressions[0].kind, ExprKind::BoolLiteral(true));
    let diagnostics = verify(program, &sources).expect_err("I32V1 must keep bool profile-gated");
    assert_eq!(diagnostics[0].code(), "ZRYNA-I1006");
}

#[test]
fn semantic_diagnostic_budget_is_exact_terminal_and_deterministic() {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: "x".to_owned(),
    }])
    .expect("diagnostic fixture source map must build");
    let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must normalize");
    let file = sources.file_id(&path).expect("fixture file must exist");
    let span = sources.span(file, 0, 1).expect("fixture span must resolve");
    let make_diagnostics = || {
        let mut errors = SemanticErrors::default();
        for index in 0..(MAX_SEMANTIC_DIAGNOSTICS + 32) {
            errors.push(Diagnostic::error_at(
                "ZRYNA-M1999",
                span,
                format!("semantic fixture error {index:03}"),
                "fix the fixture",
            ));
        }
        errors.finish()
    };

    let first_diagnostics = make_diagnostics();
    let second_diagnostics = make_diagnostics();
    assert_eq!(first_diagnostics.len(), MAX_SEMANTIC_DIAGNOSTICS);
    assert_eq!(
        first_diagnostics.last().expect("terminal diagnostic must exist").code(),
        "ZRYNA-M1201"
    );
    let first = render_structured(&first_diagnostics, &sources)
        .expect("bounded semantic diagnostics must render");
    let second = render_structured(&second_diagnostics, &sources)
        .expect("repeated semantic diagnostics must render");

    assert_eq!(first, second);
    assert_eq!(first.diagnostics.len(), MAX_SEMANTIC_DIAGNOSTICS);
    assert_eq!(
        first.diagnostics.iter().filter(|diagnostic| diagnostic.code == "ZRYNA-M1999").count(),
        MAX_SEMANTIC_DIAGNOSTICS - 1
    );
    assert!(
        first
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "ZRYNA-M1999")
            .all(|diagnostic| diagnostic.path.as_deref() == Some("src/main.zry"))
    );
    let terminal = first
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "ZRYNA-M1201")
        .expect("terminal rendered diagnostic must exist");
    assert!(terminal.path.is_none());
}
