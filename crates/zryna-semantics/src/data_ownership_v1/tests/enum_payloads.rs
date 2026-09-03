use super::*;

const OWNED_ENUM_PAYLOAD_STRUCT_SOURCE: &str = "interface Payload extends ZrynaStruct { text: String; flag: bool; }\ninterface Only extends ZrynaEnum { value: Payload; }\nfunction take(source: Only): Payload { const result: Payload = match(source, { \"Only.value\": (payload) => payload }); return result; }";
const OWNED_ENUM_PAYLOAD_STRUCT_RESPONSE: &str = r#"{"id":901,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":46,"end":52},"kind":{"kind":"string","keyword_span":{"file":0,"start":46,"end":52}}},{"span":{"file":0,"start":60,"end":64},"kind":{"kind":"named","name":{"text":"bool","span":{"file":0,"start":60,"end":64}}}},{"span":{"file":0,"start":110,"end":117},"kind":{"kind":"named","name":{"text":"Payload","span":{"file":0,"start":110,"end":117}}}},{"span":{"file":0,"start":143,"end":147},"kind":{"kind":"named","name":{"text":"Only","span":{"file":0,"start":143,"end":147}}}},{"span":{"file":0,"start":150,"end":157},"kind":{"kind":"named","name":{"text":"Payload","span":{"file":0,"start":150,"end":157}}}},{"span":{"file":0,"start":174,"end":181},"kind":{"kind":"named","name":{"text":"Payload","span":{"file":0,"start":174,"end":181}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":67},"export_span":null,"kind":{"kind":"struct","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Payload","span":{"file":0,"start":10,"end":17}},"extends_span":{"file":0,"start":18,"end":25},"marker_span":{"file":0,"start":26,"end":37},"open_brace_span":{"file":0,"start":38,"end":39},"close_brace_span":{"file":0,"start":66,"end":67},"fields":[{"span":{"file":0,"start":40,"end":53},"name":{"text":"text","span":{"file":0,"start":40,"end":44}},"colon_span":{"file":0,"start":44,"end":45},"semicolon_span":{"file":0,"start":52,"end":53},"type_syntax":0},{"span":{"file":0,"start":54,"end":65},"name":{"text":"flag","span":{"file":0,"start":54,"end":58}},"colon_span":{"file":0,"start":58,"end":59},"semicolon_span":{"file":0,"start":64,"end":65},"type_syntax":1}]}},{"span":{"file":0,"start":68,"end":120},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":68,"end":77},"name":{"text":"Only","span":{"file":0,"start":78,"end":82}},"extends_span":{"file":0,"start":83,"end":90},"marker_span":{"file":0,"start":91,"end":100},"open_brace_span":{"file":0,"start":101,"end":102},"close_brace_span":{"file":0,"start":119,"end":120},"variants":[{"span":{"file":0,"start":103,"end":118},"name":{"text":"value","span":{"file":0,"start":103,"end":108}},"colon_span":{"file":0,"start":108,"end":109},"semicolon_span":{"file":0,"start":117,"end":118},"payload_type":2,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":121,"end":255},"export_span":null,"function_span":{"file":0,"start":121,"end":129},"name":{"text":"take","span":{"file":0,"start":130,"end":134}},"parameters":[{"span":{"file":0,"start":135,"end":147},"name":{"text":"source","span":{"file":0,"start":135,"end":141}},"type_syntax":3}],"result_type":4,"body":{"span":{"file":0,"start":158,"end":255},"root_block":0,"blocks":[{"span":{"file":0,"start":158,"end":255},"open_brace_span":{"file":0,"start":158,"end":159},"statements":[0,1],"close_brace_span":{"file":0,"start":254,"end":255}}],"statements":[{"span":{"file":0,"start":160,"end":238},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":160,"end":165},"mutable":false,"name":{"text":"result","span":{"file":0,"start":166,"end":172}},"type_syntax":5,"equals_span":{"file":0,"start":182,"end":183},"initializer":2,"semicolon_span":{"file":0,"start":237,"end":238}}},{"span":{"file":0,"start":239,"end":253},"kind":{"kind":"return","keyword_span":{"file":0,"start":239,"end":245},"value":3,"semicolon_span":{"file":0,"start":252,"end":253}}}],"expressions":[{"span":{"file":0,"start":190,"end":196},"kind":{"kind":"reference","name":{"text":"source","span":{"file":0,"start":190,"end":196}}}},{"span":{"file":0,"start":227,"end":234},"kind":{"kind":"reference","name":{"text":"payload","span":{"file":0,"start":227,"end":234}}}},{"span":{"file":0,"start":184,"end":237},"kind":{"kind":"match","keyword_span":{"file":0,"start":184,"end":189},"open_paren_span":{"file":0,"start":189,"end":190},"scrutinee":0,"close_paren_span":{"file":0,"start":236,"end":237},"open_brace_span":{"file":0,"start":198,"end":199},"arms":[{"span":{"file":0,"start":200,"end":234},"type_name":{"text":"Only","span":{"file":0,"start":201,"end":205}},"dot_span":{"file":0,"start":205,"end":206},"variant":{"text":"value","span":{"file":0,"start":206,"end":211}},"binding":{"text":"payload","span":{"file":0,"start":215,"end":222}},"arrow_span":{"file":0,"start":224,"end":226},"value":1}],"close_brace_span":{"file":0,"start":235,"end":236}}},{"span":{"file":0,"start":246,"end":252},"kind":{"kind":"reference","name":{"text":"result","span":{"file":0,"start":246,"end":252}}}}]}}]}],"diagnostics":[]}}"#;
const OWNED_ENUM_PAYLOAD_ARRAY_SOURCE: &str = "interface Only extends ZrynaEnum { value: FixedArray<String, 2>; }\nfunction take(source: Only): FixedArray<String, 2> { const result: FixedArray<String, 2> = match(source, { \"Only.value\": (payload) => payload }); return result; }";
const OWNED_ENUM_PAYLOAD_ARRAY_RESPONSE: &str = r#"{"id":902,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":53,"end":59},"kind":{"kind":"string","keyword_span":{"file":0,"start":53,"end":59}}},{"span":{"file":0,"start":42,"end":63},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":42,"end":52},"less_than_span":{"file":0,"start":52,"end":53},"element":0,"comma_span":{"file":0,"start":59,"end":60},"length_span":{"file":0,"start":61,"end":62},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":62,"end":63}}},{"span":{"file":0,"start":89,"end":93},"kind":{"kind":"named","name":{"text":"Only","span":{"file":0,"start":89,"end":93}}}},{"span":{"file":0,"start":107,"end":113},"kind":{"kind":"string","keyword_span":{"file":0,"start":107,"end":113}}},{"span":{"file":0,"start":96,"end":117},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":96,"end":106},"less_than_span":{"file":0,"start":106,"end":107},"element":3,"comma_span":{"file":0,"start":113,"end":114},"length_span":{"file":0,"start":115,"end":116},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":116,"end":117}}},{"span":{"file":0,"start":145,"end":151},"kind":{"kind":"string","keyword_span":{"file":0,"start":145,"end":151}}},{"span":{"file":0,"start":134,"end":155},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":134,"end":144},"less_than_span":{"file":0,"start":144,"end":145},"element":5,"comma_span":{"file":0,"start":151,"end":152},"length_span":{"file":0,"start":153,"end":154},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":154,"end":155}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":66},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Only","span":{"file":0,"start":10,"end":14}},"extends_span":{"file":0,"start":15,"end":22},"marker_span":{"file":0,"start":23,"end":32},"open_brace_span":{"file":0,"start":33,"end":34},"close_brace_span":{"file":0,"start":65,"end":66},"variants":[{"span":{"file":0,"start":35,"end":64},"name":{"text":"value","span":{"file":0,"start":35,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":63,"end":64},"payload_type":1,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":67,"end":229},"export_span":null,"function_span":{"file":0,"start":67,"end":75},"name":{"text":"take","span":{"file":0,"start":76,"end":80}},"parameters":[{"span":{"file":0,"start":81,"end":93},"name":{"text":"source","span":{"file":0,"start":81,"end":87}},"type_syntax":2}],"result_type":4,"body":{"span":{"file":0,"start":118,"end":229},"root_block":0,"blocks":[{"span":{"file":0,"start":118,"end":229},"open_brace_span":{"file":0,"start":118,"end":119},"statements":[0,1],"close_brace_span":{"file":0,"start":228,"end":229}}],"statements":[{"span":{"file":0,"start":120,"end":212},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":120,"end":125},"mutable":false,"name":{"text":"result","span":{"file":0,"start":126,"end":132}},"type_syntax":6,"equals_span":{"file":0,"start":156,"end":157},"initializer":2,"semicolon_span":{"file":0,"start":211,"end":212}}},{"span":{"file":0,"start":213,"end":227},"kind":{"kind":"return","keyword_span":{"file":0,"start":213,"end":219},"value":3,"semicolon_span":{"file":0,"start":226,"end":227}}}],"expressions":[{"span":{"file":0,"start":164,"end":170},"kind":{"kind":"reference","name":{"text":"source","span":{"file":0,"start":164,"end":170}}}},{"span":{"file":0,"start":201,"end":208},"kind":{"kind":"reference","name":{"text":"payload","span":{"file":0,"start":201,"end":208}}}},{"span":{"file":0,"start":158,"end":211},"kind":{"kind":"match","keyword_span":{"file":0,"start":158,"end":163},"open_paren_span":{"file":0,"start":163,"end":164},"scrutinee":0,"close_paren_span":{"file":0,"start":210,"end":211},"open_brace_span":{"file":0,"start":172,"end":173},"arms":[{"span":{"file":0,"start":174,"end":208},"type_name":{"text":"Only","span":{"file":0,"start":175,"end":179}},"dot_span":{"file":0,"start":179,"end":180},"variant":{"text":"value","span":{"file":0,"start":180,"end":185}},"binding":{"text":"payload","span":{"file":0,"start":189,"end":196}},"arrow_span":{"file":0,"start":198,"end":200},"value":1}],"close_brace_span":{"file":0,"start":209,"end":210}}},{"span":{"file":0,"start":220,"end":226},"kind":{"kind":"reference","name":{"text":"result","span":{"file":0,"start":220,"end":226}}}}]}}]}],"diagnostics":[]}}"#;

#[test]
fn exhaustive_enum_match_lowers_to_refined_cfg() {
    let sources = sources_for(ENUM_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ENUM_RESPONSE), &sources)
        .expect("source-faithful enum v4");
    let program = lower(pair_input(&syntax, &sources)).expect("exhaustive match must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.blocks().len(), 3);
    assert!(
        function
            .places()
            .any(|place| matches!(place.kind(), VerifiedPlaceKind::EnumPayload { variant: 1, .. }))
    );
}

#[test]
fn owned_enum_struct_payload_moves_through_a_direct_local_continuation() {
    let sources = sources_for(OWNED_ENUM_PAYLOAD_STRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_PAYLOAD_STRUCT_RESPONSE), &sources)
        .expect("source-faithful owned Struct payload match");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct payload extraction");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.blocks().len(), 3);
    assert_eq!(function.places().len(), 7, "two payload descendants plus D+5 roots");
    let source = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::Parameter(0) => Some(place.id()),
            _ => None,
        })
        .expect("source parameter place");
    let payload = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::EnumPayload { variant: 0, .. } => Some(place.id()),
            _ => None,
        })
        .expect("active payload place");
    assert!(
        function
            .places()
            .any(|place| matches!(place.kind(), VerifiedPlaceKind::StructField { ordinal: 0, .. }))
    );
    assert_eq!(
        function
            .blocks()
            .nth(1)
            .expect("refined payload arm")
            .instructions()
            .map(FaultVerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::MoveFromPlace,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let arm = function.blocks().nth(1).expect("refined payload arm");
    assert_eq!(
        arm.instructions().nth(2).expect("source drop").place_operands().collect::<Vec<_>>(),
        vec![source]
    );
    let drop_actions =
        arm.instructions().nth(2).expect("source drop").derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(drop_actions.len(), 1);
    assert_eq!(drop_actions[0].root(), source);
    assert_eq!(drop_actions[0].active_variant(), Some(0));
    let moved = drop_actions[0].moved_projections().collect::<Vec<_>>();
    assert_eq!(moved.len(), 3, "payload root plus both declared descendants");
    assert_eq!(moved[0], payload);
    assert!(function.blocks().all(|block| block.parameters().len() == 0));
    assert!(
        function
            .blocks()
            .flat_map(|block| block.terminator().edges())
            .all(|edge| edge.arguments().len() == 0)
    );
    assert_eq!(function.cleanup_plans().len(), 1);
    assert_eq!(function.cleanup_plans().next().expect("return cleanup").actions().len(), 0);
    assert_eq!(
        function
            .places()
            .filter(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
            .count(),
        1
    );
    assert_eq!(
        function
            .places()
            .filter(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(_)))
            .count(),
        2
    );
    assert_eq!(
        function.blocks().nth(2).expect("payload continuation").terminator().kind(),
        VerifiedTerminatorKind::Return
    );
}

#[test]
fn owned_enum_fixed_array_payload_materializes_every_static_element() {
    let sources = sources_for(OWNED_ENUM_PAYLOAD_ARRAY_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_PAYLOAD_ARRAY_RESPONSE), &sources)
        .expect("source-faithful owned fixed-array payload match");
    let program =
        lower(pair_input(&syntax, &sources)).expect("owned fixed-array payload extraction");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.places().len(), 7, "two array elements plus D+5 roots");
    let payload = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::EnumPayload { variant: 0, .. } => Some(place.id()),
            _ => None,
        })
        .expect("payload place");
    assert_eq!(
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::FixedArrayConstant { base, index } if base == payload => {
                    Some(index)
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn owned_enum_payload_move_rejects_noncanonical_arm_and_mutable_local() {
    let mut wrong_arm_source = OWNED_ENUM_PAYLOAD_STRUCT_SOURCE.to_owned();
    wrong_arm_source.replace_range(227..234, "source ");
    let wrong_arm_sources = sources_for(&wrong_arm_source);
    let mut wrong_arm = response_snapshot(OWNED_ENUM_PAYLOAD_STRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut wrong_arm.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("payload reference")
    };
    name.text = "source".to_owned();
    name.span.end = 233;
    wrong_arm.files[0].functions[0].body.expressions[1].span.end = 233;
    let syntax = verify_snapshot(wrong_arm, &wrong_arm_sources)
        .expect("source-faithful noncanonical payload arm");
    assert_eq!(
        lower(pair_input(&syntax, &wrong_arm_sources))
            .expect_err("arm must yield its payload binding")[0]
            .code(),
        "ZRYNA-M3009"
    );

    let mut mutable_source = OWNED_ENUM_PAYLOAD_STRUCT_SOURCE.to_owned();
    mutable_source.replace_range(160..165, "let  ");
    let mutable_sources = sources_for(&mutable_source);
    let mut mutable = response_snapshot(OWNED_ENUM_PAYLOAD_STRUCT_RESPONSE);
    let RawStatementKind::LocalDeclaration { keyword_span, mutable: is_mutable, .. } =
        &mut mutable.files[0].functions[0].body.statements[0].kind
    else {
        panic!("payload local")
    };
    keyword_span.end = 163;
    *is_mutable = true;
    let syntax =
        verify_snapshot(mutable, &mutable_sources).expect("source-faithful mutable payload local");
    assert_eq!(
        lower(pair_input(&syntax, &mutable_sources)).expect_err("payload local must be immutable")
            [0]
        .code(),
        "ZRYNA-M3016"
    );

    let mut wrong_type_source = OWNED_ENUM_PAYLOAD_STRUCT_SOURCE.to_owned();
    wrong_type_source.replace_range(174..181, "Only   ");
    let wrong_type_sources = sources_for(&wrong_type_source);
    let mut wrong_type = response_snapshot(OWNED_ENUM_PAYLOAD_STRUCT_RESPONSE);
    let RawTypeSyntaxKind::Named { name } = &mut wrong_type.files[0].type_syntax[5].kind else {
        panic!("payload local named type")
    };
    name.text = "Only".to_owned();
    name.span.end = 178;
    wrong_type.files[0].type_syntax[5].span.end = 178;
    let syntax = verify_snapshot(wrong_type, &wrong_type_sources)
        .expect("source-faithful wrong payload local type");
    assert_eq!(
        lower(pair_input(&syntax, &wrong_type_sources))
            .expect_err("payload local type must match the result")[0]
            .code(),
        "ZRYNA-M3016"
    );
}

#[test]
fn owned_enum_payload_move_resource_preflight_accepts_exact_place_limit_only() {
    assert_eq!(
        enum_payload_move_resource_estimate(2).expect("small payload estimate"),
        EnumPayloadMoveResourceEstimate {
            blocks: 3,
            edges: 2,
            values: 3,
            places: 7,
            transitions: 4,
            cleanup_plans: 1,
            cleanup_actions: 0,
        }
    );
    let exact = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - 5;
    assert!(!enum_payload_move_resource_violation(exact));
    assert!(enum_payload_move_resource_violation(exact + 1));
    assert!(enum_payload_move_resource_violation(usize::MAX));
}
