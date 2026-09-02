use super::*;

#[test]
fn projected_aggregate_clone_local_then_assignment_rejects_the_second_global_site() {
    let (source, raw) = two_projected_aggregate_clone_sites_snapshot(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful combined projected clones");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("second projected aggregate clone");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "dst.inner = clone(src.inner);", 0),)),
    );
}

#[test]
fn projected_aggregate_clone_assignment_then_local_rejects_the_second_global_site() {
    let (source, raw) = two_projected_aggregate_clone_sites_snapshot(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed projected clones");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("second projected aggregate clone");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "clone(src.inner)", 1),)),
    );
}

#[test]
fn projected_aggregate_clone_assignment_rejects_a_same_root_source() {
    let mut source = PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.trim_end().to_owned();
    let source_start = source.rfind("src.inner").expect("clone source projection");
    source.replace_range(source_start..source_start + "src".len(), "dst");
    let mut raw = response_snapshot(PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Assignment { value: clone, .. } = body.statements[2].kind else {
        panic!("assignment")
    };
    let zryna_syntax::v4::RawExpressionKind::Clone { value: operand, .. } =
        body.expressions[clone as usize].kind
    else {
        panic!("clone")
    };
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { base, .. } =
        body.expressions[operand as usize].kind
    else {
        panic!("source projection")
    };
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut body.expressions[base as usize].kind
    else {
        panic!("source root")
    };
    name.text = "dst".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful same-root clone");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("same-root clone source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "dst.inner", 1))),
    );
}

#[test]
fn projected_aggregate_clone_assignment_rejects_a_moved_source_subtree() {
    let source = PROJECTED_SUBOBJECT_CLONE_AFTER_MOVE_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_SUBOBJECT_CLONE_AFTER_MOVE_RESPONSE), &sources)
            .expect("source-faithful moved projected clone source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved clone source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(source, "src.inner", 1))),
    );
}

#[test]
fn projected_aggregate_clone_assignment_rejects_function_parameters() {
    let source = PROJECTED_SUBOBJECT_CLONE_WITH_PARAMETER_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax = verify_snapshot(
        response_snapshot(PROJECTED_SUBOBJECT_CLONE_WITH_PARAMETER_RESPONSE),
        &sources,
    )
    .expect("source-faithful parameterized projected clone");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("parameterized clone");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(source, "clone(src.inner)", 0))),
    );
}

#[test]
fn projected_aggregate_assignment_rejects_a_same_root_projected_source() {
    let mut source = PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE.to_owned();
    let start = source.rfind("replacement").expect("assignment source");
    let end = start + "replacement".len();
    source.replace_range(start..end, "o.inner");
    let start = u32::try_from(start).expect("source start");
    let end = u32::try_from(end).expect("source end");
    let mut raw = shift_snapshot_signed(
        response_snapshot(PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE),
        end,
        i32::try_from("o.inner".len()).expect("replacement length")
            - i32::try_from("replacement".len()).expect("source length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    body.expressions[8] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
            },
        },
    };
    let projected = u32::try_from(body.expressions.len()).expect("projected source");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 7 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: 8,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: start + 1, end: start + 2 },
            field: RawIdentifierSyntax {
                text: "inner".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: start + 2, end: start + 7 },
            },
        },
    });
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[2].kind else {
        panic!("assignment")
    };
    *value = projected;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("same-root projected source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "o.inner", 1))),
    );
}
