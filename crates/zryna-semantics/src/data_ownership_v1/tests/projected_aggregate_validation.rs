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

#[test]
fn projected_string_clone_rejects_a_moved_overlapping_leaf() {
    let (source, raw) =
        owned_array_projected_clone_return_snapshot(OwnedArrayProjectionCase::Repeat, 1);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful repeated projected clone");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("clone of moved projection must fail");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "clone(a[0])", 0))),
    );
}
#[test]
fn projected_string_clone_rejects_copy_and_nonconstant_array_leaves() {
    let (copy_source, copy_raw) = owned_pair_copy_projection_clone_snapshot();
    let copy_sources = sources_for(&copy_source);
    let copy_syntax =
        verify_snapshot(copy_raw, &copy_sources).expect("source-faithful Copy projection clone");
    let copy = lower(pair_input(&copy_syntax, &copy_sources))
        .expect_err("Copy projection is not a String clone source");
    assert_eq!(copy.len(), 1);
    assert_eq!(copy[0].code(), "ZRYNA-M3012");
    assert_eq!(
        copy[0].primary_span(),
        Some(span(&copy_sources, nth_untrusted_span(&copy_source, "clone(p.flag)", 0),)),
    );

    for (case, needle, label) in [
        (OwnedArrayProjectionCase::Dynamic, "a[a]", "dynamic"),
        (OwnedArrayProjectionCase::Negative, "a[-1]", "negative"),
        (OwnedArrayProjectionCase::OutOfBounds, "a[2]", "out of bounds"),
    ] {
        let (source, raw) = owned_array_projected_clone_return_snapshot(case, 0);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw, &sources).expect("source-faithful invalid projected clone");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics.len(), 1, "{label}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3006", "{label}");
        let projection = nth_untrusted_span(&source, needle, 0);
        let child = zryna_source::UntrustedSpan {
            file: projection.file,
            start: projection.start + 2,
            end: projection.end - 1,
        };
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, child)), "{label}");
    }
}
#[test]
fn owned_projection_repeat_is_m3014() {
    let (repeat_source, repeat_raw) =
        owned_array_projected_return_snapshot(OwnedArrayProjectionCase::Repeat);
    let repeat_sources = sources_for(&repeat_source);
    let repeat_syntax = verify_snapshot(repeat_raw, &repeat_sources)
        .expect("source-faithful repeated array projection");
    let repeat =
        lower(pair_input(&repeat_syntax, &repeat_sources)).expect_err("repeated projection move");
    assert_eq!(repeat[0].code(), "ZRYNA-M3014");
    assert_eq!(
        repeat[0].primary_span(),
        Some(span(&repeat_sources, nth_untrusted_span(&repeat_source, "a[0]", 1))),
    );
}
#[test]
fn owned_projection_invalid_field_and_index_diagnostics_use_the_projection_child() {
    let (field_source, field_raw) = owned_pair_projected_return_snapshot("nope");
    let field_sources = sources_for(&field_source);
    let field_syntax =
        verify_snapshot(field_raw, &field_sources).expect("source-faithful invalid owned field");
    let field = lower(pair_input(&field_syntax, &field_sources)).expect_err("invalid owned field");
    assert_eq!(field[0].code(), "ZRYNA-M3006");
    assert_eq!(
        field[0].primary_span(),
        Some(span(&field_sources, nth_untrusted_span(&field_source, "nope", 0))),
    );

    for (case, needle, label) in [
        (OwnedArrayProjectionCase::Dynamic, "a[a]", "dynamic"),
        (OwnedArrayProjectionCase::Negative, "a[-1]", "negative"),
        (OwnedArrayProjectionCase::OutOfBounds, "a[2]", "out of bounds"),
    ] {
        let (source, raw) = owned_array_projected_return_snapshot(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful invalid owned index");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3006", "{label}");
        let projection = nth_untrusted_span(&source, needle, 0);
        let expected = zryna_source::UntrustedSpan {
            file: projection.file,
            start: projection.start + 2,
            end: projection.end - 1,
        };
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, expected)), "{label}");
    }
}
#[test]
fn aggregate_projection_wrong_base_kinds_are_symmetric_m3006() {
    for (source, raw, needle, label) in [
        {
            let (source, raw) = struct_index_wrong_base_snapshot();
            (source, raw, "p[0]", "Struct indexed as FixedArray")
        },
        {
            let (source, raw) = fixed_array_field_wrong_base_snapshot();
            (source, raw, "a.foo", "FixedArray accessed as Struct")
        },
    ] {
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong-base projection");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics.len(), 1, "{label}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3006", "{label}");
        let projection = nth_untrusted_span(&source, needle, 0);
        let child = zryna_source::UntrustedSpan {
            file: projection.file,
            start: projection.start + 2,
            end: if needle == "p[0]" { projection.start + 3 } else { projection.end },
        };
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, child)), "{label}");
    }
}
