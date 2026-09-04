use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

use super::mixed_string_calls::mixed_string_call_fixture;

fn outer_call(raw: &RawProjectSyntaxSnapshot) -> (UntrustedSpan, UntrustedSpan) {
    let expression = &raw.files[0].functions[0].body.expressions[2];
    let RawExpressionKind::Call { callee, arguments, .. } = &expression.kind else {
        panic!("source-spelled outer String identity call")
    };
    assert_eq!(arguments, &[1]);
    (expression.span, callee.span)
}

fn rename_outer_call(source: &mut String, raw: &mut RawProjectSyntaxSnapshot, name: &str) {
    let (_, at) = outer_call(raw);
    let start = usize::try_from(at.start).expect("callee start");
    let end = usize::try_from(at.end).expect("callee end");
    assert_eq!(&source[start..end], "identity");
    assert_eq!(name.len(), end - start);
    source.replace_range(start..end, name);
    let RawExpressionKind::Call { callee, .. } =
        &mut raw.files[0].functions[0].body.expressions[2].kind
    else {
        panic!("outer String call")
    };
    callee.text = name.to_owned();
}

fn positive_control() {
    let (source, raw) = mixed_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed String call control");
    lower(pair_input(&syntax, &sources))
        .expect("control reaches independent ownership IR verifier");
}

fn reject_replays(sources: &SourceMap, raw: &RawProjectSyntaxSnapshot, expected: &[Diagnostic]) {
    for _ in 0..2 {
        let syntax = verify_snapshot(raw.clone(), sources)
            .expect("rejection fixture passes source authentication");
        let actual = lower(pair_input(&syntax, sources)).expect_err("sealed String call rejection");
        assert_eq!(actual, expected);
    }
}

#[test]
fn mixed_string_call_rejects_wrong_case_at_exact_callee_span() {
    positive_control();
    let (mut source, mut raw) = mixed_string_call_fixture();
    rename_outer_call(&mut source, &mut raw, "Identity");
    let sources = sources_for(&source);
    let (_, at) = outer_call(&raw);
    let expected = [Diagnostic::error_at(
        "ZRYNA-M3002",
        span(&sources, at),
        "call name 'Identity' has the wrong portable ASCII case",
        "use the callee's exact declared spelling",
    )];
    reject_replays(&sources, &raw, &expected);
}

#[test]
fn mixed_string_call_rejects_missing_name_at_exact_callee_span() {
    positive_control();
    let (mut source, mut raw) = mixed_string_call_fixture();
    rename_outer_call(&mut source, &mut raw, "missingx");
    let sources = sources_for(&source);
    let (_, at) = outer_call(&raw);
    let expected = [Diagnostic::error_at(
        "ZRYNA-M3002",
        span(&sources, at),
        "function 'missingx' is not declared in this module",
        "call one exact private same-module function",
    )];
    reject_replays(&sources, &raw, &expected);
}

#[test]
fn mixed_string_call_rejects_arity_at_complete_call_span() {
    positive_control();
    let (mut source, mut raw) = mixed_string_call_fixture();
    // Both actual producer declarations and the nested argument remain intact.
    rename_outer_call(&mut source, &mut raw, "producer");
    let sources = sources_for(&source);
    let (at, _) = outer_call(&raw);
    let expected = [Diagnostic::error_at(
        "ZRYNA-M3012",
        span(&sources, at),
        "call to 'producer' has 1 arguments but its signature requires 0",
        "pass the exact declared String argument",
    )];
    reject_replays(&sources, &raw, &expected);
}

#[test]
fn mixed_string_call_rejects_exported_callee_and_public_owned_signature() {
    positive_control();
    let (mut source, raw) = mixed_string_call_fixture();
    let start = raw.files[0].functions[1].span.start;
    let offset = usize::try_from(start).expect("identity function start");
    assert!(source[offset..].starts_with("function identity"));
    source.insert_str(offset, "export ");
    let mut raw = shift_snapshot(raw, start, 7);
    let identity = &mut raw.files[0].functions[1];
    identity.span.start = start;
    identity.export_span = Some(UntrustedSpan { file: 0, start, end: start + 6 });
    let function_at = identity.span;
    let (_, call_at) = outer_call(&raw);
    assert!(call_at.end < function_at.start);
    let sources = sources_for(&source);
    // The ordinary module walk also diagnoses the exported declaration. Retain
    // its complete diagnostic instead of filtering the result to a call error.
    let expected = [
        Diagnostic::error_at(
            "ZRYNA-M3016",
            span(&sources, call_at),
            "owned calls require one private same-module callee",
            "keep String producers and identity functions internal",
        ),
        Diagnostic::error_at(
            "ZRYNA-M3010",
            span(&sources, function_at),
            "public owned signatures are outside scalar ABI v1",
            "keep owned functions internal and export only bool/i32 signatures",
        ),
    ];
    reject_replays(&sources, &raw, &expected);
}
