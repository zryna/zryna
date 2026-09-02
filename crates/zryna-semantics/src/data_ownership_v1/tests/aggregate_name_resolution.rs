use super::*;

#[test]
fn wrong_case_value_references_are_not_aliases() {
    let mut parameter_source = ARRAY_OOB_SOURCE.to_owned();
    parameter_source.replace_range(51..53, "XS");
    let parameter_sources = sources_for(&parameter_source);
    let mut parameter = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut parameter.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("parameter reference")
    };
    name.text = "XS".to_owned();
    let syntax = verify_snapshot(parameter, &parameter_sources).expect("wrong-case parameter v4");
    let diagnostics =
        lower(pair_input(&syntax, &parameter_sources)).expect_err("wrong-case parameter");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("parameter use").start(), 51);

    let mut local_source = PAIR_SCORE_SOURCE.to_owned();
    local_source.replace_range(160..164, "PAIR");
    let local_sources = sources_for(&local_source);
    let mut local = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut local.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("local reference")
    };
    name.text = "PAIR".to_owned();
    let syntax = verify_snapshot(local, &local_sources).expect("wrong-case local v4");
    let diagnostics = lower(pair_input(&syntax, &local_sources)).expect_err("wrong-case local");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("local use").start(), 160);

    let mut scrutinee_source = ENUM_SOURCE.to_owned();
    scrutinee_source.replace_range(109..110, "X");
    let scrutinee_sources = sources_for(&scrutinee_source);
    let mut scrutinee = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut scrutinee.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("scrutinee")
    };
    name.text = "X".to_owned();
    let syntax = verify_snapshot(scrutinee, &scrutinee_sources).expect("wrong-case scrutinee v4");
    let diagnostics =
        lower(pair_input(&syntax, &scrutinee_sources)).expect_err("wrong-case scrutinee");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("scrutinee use").start(), 109);

    let mut payload_source = ENUM_SOURCE.to_owned();
    payload_source.replace_range(162..167, "VALUE");
    let payload_sources = sources_for(&payload_source);
    let mut payload = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut payload.files[0].functions[0].body.expressions[2].kind
    else {
        panic!("payload")
    };
    name.text = "VALUE".to_owned();
    let syntax = verify_snapshot(payload, &payload_sources).expect("wrong-case payload v4");
    let diagnostics = lower(pair_input(&syntax, &payload_sources)).expect_err("wrong-case payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("payload use").start(), 162);
}

#[test]
fn unresolved_aggregate_element_type_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(28..31, "Foo");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[0].kind else {
        panic!("named element")
    };
    name.text = "Foo".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved type");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved type");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
}

#[test]
fn unresolved_value_name_is_rejected_at_its_use() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(160..164, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut raw.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("pair reference")
    };
    name.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unresolved value");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unresolved value");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("name use").start(), 160);
}
