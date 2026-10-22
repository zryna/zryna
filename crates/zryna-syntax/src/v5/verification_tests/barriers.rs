//! Whole-project rejection barriers, hostile inventory, and bounded diagnostics.

use super::*;

#[test]
fn whole_project_raw_barrier_precedes_real_earlier_declaration_error() {
    let source = MAIN.replacen("function score", "function Weak ", 1);
    let map = sources(&source, VALUES);
    let mut authentic = value(REFERENCE);
    let name = &mut authentic["files"][0]["functions"][0]["name"];
    name["text"] = json!("Weak");
    name["span"]["end"] = json!(name["span"]["end"].as_u64().expect("name end") - 1);
    let start = u32::try_from(source.find("Weak").expect("reserved name")).expect("fixture offset");
    let expected = map
        .verify_span(UntrustedSpan { file: 0, start, end: start + 4 })
        .expect("authentic reserved identifier");
    let declaration = validate_declarations(&map, &raw(authentic.clone()))
        .expect_err("real source-spelled reserved declaration");
    assert_eq!(declaration.code, "ZRYNA-D7001");
    assert_eq!(declaration.span, Some(expected));
    let full = verify_snapshot(raw(authentic.clone()), &map)
        .expect_err("complete source/arena baseline reaches declaration admission");
    assert_eq!(full, vec![declaration]);
    for orphan in [false, true] {
        let mut snapshot = authentic.clone();
        let body = &mut snapshot["files"][1]["functions"][0]["body"];
        if orphan {
            let expression = body["expressions"][0].clone();
            body["expressions"].as_array_mut().expect("expressions").push(expression);
        } else {
            body["expressions"][0]["span"]["file"] = json!(0);
        }
        assert_eq!(
            validate_declarations(&map, &raw(snapshot.clone()))
                .expect_err("real reserved declaration")
                .code,
            "ZRYNA-D7001"
        );
        rejects(snapshot, &map);
    }
}

#[test]
fn faithful_invalid_bound_is_deferred_until_complete_project_barrier() {
    let mut snapshot = value(REFERENCE);
    let bound = &mut snapshot["files"][1]["data_declarations"][0]["type_parameters"]["parameters"]
        [0]["bound"];
    let start =
        bound["span"]["start"].as_u64().and_then(|n| usize::try_from(n).ok()).expect("start");
    let end = bound["span"]["end"].as_u64().and_then(|n| usize::try_from(n).ok()).expect("end");
    bound["text"] = json!("OpaqueCopy");
    let mut source = VALUES.to_owned();
    source.replace_range(start..end, "OpaqueCopy");
    let errors = verify_snapshot(raw(snapshot), &sources(MAIN, &source))
        .expect_err("invalid source-spelled bound");
    assert_eq!(errors[0].code, "ZRYNA-D7001");
    assert!(errors[0].span.is_some());
}

#[test]
fn omitted_body_source_rejects_even_with_clean_owned_arena() {
    let mut snapshot = value(REFERENCE);
    let body = &mut snapshot["files"][0]["functions"][0]["body"];
    body["blocks"][0]["statements"].as_array_mut().expect("block statements").pop();
    body["statements"].as_array_mut().expect("statements").pop();
    body["expressions"].as_array_mut().expect("expressions").truncate(14);
    let map = sources(MAIN, VALUES);
    validate_declarations(&map, &raw(snapshot.clone())).expect("header-only boundary");
    rejects(snapshot, &map);
}

#[test]
fn typed_caller_cannot_bypass_per_expression_budget() {
    let mut snapshot = raw(value(CONTROL));
    let first = snapshot.files[0].functions[0].body.expressions[0].clone();
    snapshot.files[0].functions[0].body.expressions =
        vec![first; crate::v4::MAX_EXPRESSIONS_PER_FUNCTION + 1];
    let errors =
        verify_snapshot(snapshot, &single("control", CONTROL_SOURCE)).expect_err("first extra");
    assert_eq!(errors[0].code, "ZRYNA-Y5201");
}

#[test]
fn provider_advisory_cannot_hide_incomplete_source() {
    let mut snapshot = value(REFERENCE);
    snapshot["diagnostics"] = json!([{ "code": "ZRYNA-M7001", "severity": "error", "location": { "kind": "global" }, "message": "missing arguments", "guidance": "provide arguments" }]);
    snapshot["files"][0]["functions"][0]["body"]["blocks"][0]["statements"]
        .as_array_mut()
        .expect("root")
        .pop();
    rejects(snapshot, &sources(MAIN, VALUES));
}

#[test]
fn reserved_terminal_slot_and_order_are_deterministic() {
    let map = single("control", &"x".repeat(300));
    let run = || {
        let mut errors = super::super::diagnostic_order::Errors::default();
        for start in 0..300 {
            let span = map
                .verify_span(UntrustedSpan { file: 0, start, end: start + 1 })
                .expect("authenticated location");
            errors.push(super::super::DeclarationError::malformed(Some(span)));
        }
        assert!(errors.terminal());
        errors.finish()
    };
    let errors = run();
    assert_eq!(errors, run());
    assert_eq!(errors.len(), 256);
    assert_eq!(errors.last().expect("terminal").code, "ZRYNA-Y5201");
}

#[test]
fn raw_span_extremes_never_panic_full_verifier() {
    for field in ["start", "end"] {
        let mut snapshot = value(REFERENCE);
        snapshot["files"][0]["functions"][0]["body"]["expressions"][0]["span"][field] =
            json!(u32::MAX);
        let map = sources(MAIN, VALUES);
        let result = std::panic::catch_unwind(|| verify_snapshot(raw(snapshot), &map))
            .expect("no untrusted arithmetic panic");
        assert_eq!(result.expect_err("no seal")[0].code, "ZRYNA-Y5001");
    }
}
