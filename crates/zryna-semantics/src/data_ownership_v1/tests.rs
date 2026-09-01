use super::{
    Errors, SemanticInput, ValueBudgetLimit, derived_value_count, lower, semantic_preflight,
    value_budget_violation,
};
use zryna_ir::data_ownership_v1::{
    VerifiedFunction, VerifiedInstructionKind, VerifiedPlaceKind, VerifiedTerminatorKind,
};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{
    PROTOCOL_VERSION, RawBlockSyntax, RawDataDeclaration, RawDataDeclarationKind, RawDataField,
    RawExpressionSyntax, RawFunctionBodySyntax, RawFunctionSyntax, RawIdentifierSyntax,
    RawParameterSyntax, RawProjectSyntaxSnapshot, RawSourceUnit, RawStatementKind,
    RawStatementSyntax, RawTypeSyntax, RawTypeSyntaxKind, decode_snapshot, verify_snapshot,
};

const PAIR_SOURCE: &str = include_str!("../../../../tests/m3-fixtures/syntax-v4-shorthand.zry");
const PAIR_JSON: &[u8] = include_bytes!("../../../../tests/m3-fixtures/syntax-v4-shorthand.json");
const PAIR_SCORE_SOURCE: &str = include_str!("../../../../tests/m3-fixtures/pair-score-v4.zry");
const PAIR_SCORE_JSON: &[u8] = include_bytes!("../../../../tests/m3-fixtures/pair-score-v4.json");
const PAIR_ORACLE: &str = include_str!("../../../../tests/m3-fixtures/pair-oracle-v1.json");
const STRING_SOURCE: &str = "function bad(): String { return \"x\"; }";
const STRING_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":22},"kind":{"kind":"string","keyword_span":{"file":0,"start":16,"end":22}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":38},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"bad","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":23,"end":38},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":38},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0],"close_brace_span":{"file":0,"start":37,"end":38}}],"statements":[{"span":{"file":0,"start":25,"end":36},"kind":{"kind":"return","keyword_span":{"file":0,"start":25,"end":31},"value":0,"semicolon_span":{"file":0,"start":35,"end":36}}}],"expressions":[{"span":{"file":0,"start":32,"end":35},"kind":{"kind":"string-literal","spelling":"\"x\""}}]}}]}],"diagnostics":[]}}"#;
const ENUM_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction get(x: Maybe): i32 { return match(x, { \"Maybe.none\": () => 0, \"Maybe.some\": (value) => value }); }";
const ENUM_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":82,"end":87},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":82,"end":87}}}},{"span":{"file":0,"start":90,"end":93},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":90,"end":93}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":173},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"get","span":{"file":0,"start":75,"end":78}},"parameters":[{"span":{"file":0,"start":79,"end":87},"name":{"text":"x","span":{"file":0,"start":79,"end":80}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":94,"end":173},"root_block":0,"blocks":[{"span":{"file":0,"start":94,"end":173},"open_brace_span":{"file":0,"start":94,"end":95},"statements":[0],"close_brace_span":{"file":0,"start":172,"end":173}}],"statements":[{"span":{"file":0,"start":96,"end":171},"kind":{"kind":"return","keyword_span":{"file":0,"start":96,"end":102},"value":3,"semicolon_span":{"file":0,"start":170,"end":171}}}],"expressions":[{"span":{"file":0,"start":109,"end":110},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":109,"end":110}}}},{"span":{"file":0,"start":134,"end":135},"kind":{"kind":"i32-literal","spelling":"0"}},{"span":{"file":0,"start":162,"end":167},"kind":{"kind":"reference","name":{"text":"value","span":{"file":0,"start":162,"end":167}}}},{"span":{"file":0,"start":103,"end":170},"kind":{"kind":"match","keyword_span":{"file":0,"start":103,"end":108},"open_paren_span":{"file":0,"start":108,"end":109},"scrutinee":0,"close_paren_span":{"file":0,"start":169,"end":170},"open_brace_span":{"file":0,"start":112,"end":113},"arms":[{"span":{"file":0,"start":114,"end":135},"type_name":{"text":"Maybe","span":{"file":0,"start":115,"end":120}},"dot_span":{"file":0,"start":120,"end":121},"variant":{"text":"none","span":{"file":0,"start":121,"end":125}},"binding":null,"arrow_span":{"file":0,"start":131,"end":133},"value":1},{"span":{"file":0,"start":137,"end":167},"type_name":{"text":"Maybe","span":{"file":0,"start":138,"end":143}},"dot_span":{"file":0,"start":143,"end":144},"variant":{"text":"some","span":{"file":0,"start":144,"end":148}},"binding":{"text":"value","span":{"file":0,"start":152,"end":157}},"arrow_span":{"file":0,"start":159,"end":161},"value":2}],"close_brace_span":{"file":0,"start":168,"end":169}}}]}}]}],"diagnostics":[]}}"#;
const ARRAY_OOB_SOURCE: &str = "function get(xs: FixedArray<i32, 2>): i32 { return xs[2]; }";
const ARRAY_VALID_SOURCE: &str = "function get(xs: FixedArray<i32, 2>): i32 { return xs[1]; }";
const ARRAY_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":38,"end":41},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":38,"end":41}}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":59},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"get","span":{"file":0,"start":9,"end":12}},"parameters":[{"span":{"file":0,"start":13,"end":35},"name":{"text":"xs","span":{"file":0,"start":13,"end":15}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":42,"end":59},"root_block":0,"blocks":[{"span":{"file":0,"start":42,"end":59},"open_brace_span":{"file":0,"start":42,"end":43},"statements":[0],"close_brace_span":{"file":0,"start":58,"end":59}}],"statements":[{"span":{"file":0,"start":44,"end":57},"kind":{"kind":"return","keyword_span":{"file":0,"start":44,"end":50},"value":2,"semicolon_span":{"file":0,"start":56,"end":57}}}],"expressions":[{"span":{"file":0,"start":51,"end":53},"kind":{"kind":"reference","name":{"text":"xs","span":{"file":0,"start":51,"end":53}}}},{"span":{"file":0,"start":54,"end":55},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":51,"end":56},"kind":{"kind":"index","base":0,"open_bracket_span":{"file":0,"start":53,"end":54},"index":1,"close_bracket_span":{"file":0,"start":55,"end":56}}}]}}]}],"diagnostics":[]}}"#;
const ARRAY_CONSTRUCT_SOURCE: &str =
    "function make(): FixedArray<i32, 2> { return FixedArray<i32, 2>([1, 2]); }";
const ARRAY_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":28,"end":31},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":28,"end":31}}}},{"span":{"file":0,"start":17,"end":35},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":17,"end":27},"less_than_span":{"file":0,"start":27,"end":28},"element":0,"comma_span":{"file":0,"start":31,"end":32},"length_span":{"file":0,"start":33,"end":34},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":34,"end":35}}},{"span":{"file":0,"start":56,"end":59},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":56,"end":59}}}},{"span":{"file":0,"start":45,"end":63},"kind":{"kind":"fixed-array","keyword_span":{"file":0,"start":45,"end":55},"less_than_span":{"file":0,"start":55,"end":56},"element":2,"comma_span":{"file":0,"start":59,"end":60},"length_span":{"file":0,"start":61,"end":62},"length":2,"length_spelling":"2","greater_than_span":{"file":0,"start":62,"end":63}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":74},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":36,"end":74},"root_block":0,"blocks":[{"span":{"file":0,"start":36,"end":74},"open_brace_span":{"file":0,"start":36,"end":37},"statements":[0],"close_brace_span":{"file":0,"start":73,"end":74}}],"statements":[{"span":{"file":0,"start":38,"end":72},"kind":{"kind":"return","keyword_span":{"file":0,"start":38,"end":44},"value":2,"semicolon_span":{"file":0,"start":71,"end":72}}}],"expressions":[{"span":{"file":0,"start":65,"end":66},"kind":{"kind":"i32-literal","spelling":"1"}},{"span":{"file":0,"start":68,"end":69},"kind":{"kind":"i32-literal","spelling":"2"}},{"span":{"file":0,"start":45,"end":71},"kind":{"kind":"fixed-array-construction","type_syntax":3,"open_paren_span":{"file":0,"start":63,"end":64},"open_bracket_span":{"file":0,"start":64,"end":65},"elements":[0,1],"close_bracket_span":{"file":0,"start":69,"end":70},"close_paren_span":{"file":0,"start":70,"end":71}}}]}}]}],"diagnostics":[]}}"#;
const ENUM_CONSTRUCT_SOURCE: &str = "interface Maybe extends ZrynaEnum { none: ZrynaNone; some: i32; }\nfunction make(x: i32): Maybe { return Maybe.some(x); }";
const ENUM_CONSTRUCT_RESPONSE: &str = r#"{"id":1,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":59,"end":62},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":59,"end":62}}}},{"span":{"file":0,"start":83,"end":86},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":83,"end":86}}}},{"span":{"file":0,"start":89,"end":94},"kind":{"kind":"named","name":{"text":"Maybe","span":{"file":0,"start":89,"end":94}}}}],"data_declarations":[{"span":{"file":0,"start":0,"end":65},"export_span":null,"kind":{"kind":"enum","interface_span":{"file":0,"start":0,"end":9},"name":{"text":"Maybe","span":{"file":0,"start":10,"end":15}},"extends_span":{"file":0,"start":16,"end":23},"marker_span":{"file":0,"start":24,"end":33},"open_brace_span":{"file":0,"start":34,"end":35},"close_brace_span":{"file":0,"start":64,"end":65},"variants":[{"span":{"file":0,"start":36,"end":52},"name":{"text":"none","span":{"file":0,"start":36,"end":40}},"colon_span":{"file":0,"start":40,"end":41},"semicolon_span":{"file":0,"start":51,"end":52},"payload_type":null,"none_span":{"file":0,"start":42,"end":51}},{"span":{"file":0,"start":53,"end":63},"name":{"text":"some","span":{"file":0,"start":53,"end":57}},"colon_span":{"file":0,"start":57,"end":58},"semicolon_span":{"file":0,"start":62,"end":63},"payload_type":0,"none_span":null}]}}],"functions":[{"span":{"file":0,"start":66,"end":120},"export_span":null,"function_span":{"file":0,"start":66,"end":74},"name":{"text":"make","span":{"file":0,"start":75,"end":79}},"parameters":[{"span":{"file":0,"start":80,"end":86},"name":{"text":"x","span":{"file":0,"start":80,"end":81}},"type_syntax":1}],"result_type":2,"body":{"span":{"file":0,"start":95,"end":120},"root_block":0,"blocks":[{"span":{"file":0,"start":95,"end":120},"open_brace_span":{"file":0,"start":95,"end":96},"statements":[0],"close_brace_span":{"file":0,"start":119,"end":120}}],"statements":[{"span":{"file":0,"start":97,"end":118},"kind":{"kind":"return","keyword_span":{"file":0,"start":97,"end":103},"value":1,"semicolon_span":{"file":0,"start":117,"end":118}}}],"expressions":[{"span":{"file":0,"start":115,"end":116},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":115,"end":116}}}},{"span":{"file":0,"start":104,"end":117},"kind":{"kind":"enum-construction","type_name":{"text":"Maybe","span":{"file":0,"start":104,"end":109}},"dot_span":{"file":0,"start":109,"end":110},"variant":{"text":"some","span":{"file":0,"start":110,"end":114}},"open_paren_span":{"file":0,"start":114,"end":115},"payload":0,"close_paren_span":{"file":0,"start":116,"end":117}}}]}}]}],"diagnostics":[]}}"#;

fn response_snapshot(response: &str) -> RawProjectSyntaxSnapshot {
    let value: serde_json::Value = serde_json::from_str(response).expect("adapter response JSON");
    let result = value.get("result").expect("adapter result");
    decode_snapshot(&serde_json::to_vec(result).expect("snapshot JSON")).expect("v4 snapshot")
}

fn sources_for(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("source map")
}

fn shift_snapshot(
    raw: RawProjectSyntaxSnapshot,
    cutoff: u32,
    amount: u32,
) -> RawProjectSyntaxSnapshot {
    fn visit(value: &mut serde_json::Value, cutoff: u32, amount: u32) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("file")
                    && object.contains_key("start")
                    && object.contains_key("end")
                {
                    for key in ["start", "end"] {
                        if let Some(number) = object.get_mut(key) {
                            let current = u32::try_from(number.as_u64().expect("span number"))
                                .expect("u32 span");
                            if current >= cutoff {
                                *number = serde_json::Value::from(current + amount);
                            }
                        }
                    }
                } else {
                    for child in object.values_mut() {
                        visit(child, cutoff, amount);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, cutoff, amount);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    visit(&mut value, cutoff, amount);
    serde_json::from_value(value).expect("shifted snapshot")
}

fn pair_sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: PAIR_SOURCE.to_owned(),
    }])
    .expect("Pair source map")
}

fn pair_input<'a>(
    syntax: &'a zryna_syntax::v4::ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
) -> SemanticInput<'a> {
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = sources.file_id(&path).expect("entry");
    SemanticInput::try_new(syntax, sources, entry).expect("authenticated Pair input")
}

#[test]
fn pair_oracle_lowers_to_sealed_copy_aggregate_ir() {
    let sources = pair_sources();
    let raw = decode_snapshot(PAIR_JSON).expect("Pair v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("Pair v4 authority");

    let program = lower(pair_input(&syntax, &sources)).expect("Pair must lower and verify");

    assert_eq!(program.modules().len(), 1);
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let places = function.places().collect::<Vec<_>>();
    assert!(places.iter().any(|place| matches!(place.kind(), VerifiedPlaceKind::Parameter(0))));
    let kinds = function
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(kinds.iter().any(|kind| matches!(kind, VerifiedInstructionKind::StructConstruct)));
}

#[derive(Clone, Debug)]
enum OracleValue {
    I32(i32),
    Aggregate(Vec<OracleValue>),
}

#[allow(clippy::too_many_lines)]
fn evaluate_pair(function: VerifiedFunction<'_>, arguments: [i32; 2]) -> i32 {
    let places = function.places().collect::<Vec<_>>();
    let mut values = vec![None; 64];
    for (index, argument) in arguments.into_iter().enumerate() {
        values[index] = Some(OracleValue::I32(argument));
    }
    let mut roots = vec![None; places.len()];
    for place in &places {
        if let VerifiedPlaceKind::Parameter(index) = place.kind() {
            roots[usize::try_from(place.id().index()).expect("place index")] =
                values[usize::try_from(index).expect("parameter index")].clone();
        }
    }
    let resolve_place = |place_index: u32,
                         roots: &[Option<OracleValue>],
                         values: &[Option<OracleValue>]| {
        let mut path = Vec::new();
        let mut current = place_index;
        loop {
            let place = places[usize::try_from(current).expect("place index")];
            match place.kind() {
                VerifiedPlaceKind::Parameter(_) | VerifiedPlaceKind::Local(_) => {
                    let mut value = roots[usize::try_from(current).expect("root index")]
                        .clone()
                        .expect("initialized root");
                    for ordinal in path.into_iter().rev() {
                        let OracleValue::Aggregate(fields) = value else {
                            panic!("aggregate projection")
                        };
                        value = fields[usize::try_from(ordinal).expect("field ordinal")].clone();
                    }
                    break value;
                }
                VerifiedPlaceKind::Temporary(value) => {
                    let mut result = values[usize::try_from(value.index()).expect("value index")]
                        .clone()
                        .expect("temporary value");
                    for ordinal in path.into_iter().rev() {
                        let OracleValue::Aggregate(fields) = result else {
                            panic!("aggregate projection")
                        };
                        result = fields[usize::try_from(ordinal).expect("field ordinal")].clone();
                    }
                    break result;
                }
                VerifiedPlaceKind::StructField { base, ordinal }
                | VerifiedPlaceKind::FixedArrayConstant { base, index: ordinal } => {
                    path.push(ordinal);
                    current = base.index();
                }
                VerifiedPlaceKind::EnumPayload { .. } => panic!("Pair oracle has no enum payload"),
            }
        }
    };
    let block = function.blocks().next().expect("Pair block");
    for instruction in block.instructions() {
        let operands = instruction
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>();
        let place_operands = instruction
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>();
        let result = match instruction.kind() {
            VerifiedInstructionKind::I32Literal => {
                Some(OracleValue::I32(instruction.i32_literal().expect("literal")))
            }
            VerifiedInstructionKind::StructConstruct => Some(OracleValue::Aggregate(
                operands
                    .iter()
                    .map(|id| {
                        values[usize::try_from(*id).expect("value index")].clone().expect("operand")
                    })
                    .collect(),
            )),
            VerifiedInstructionKind::CopyFromPlace => {
                Some(resolve_place(place_operands[0], &roots, &values))
            }
            VerifiedInstructionKind::InitializePlace => {
                roots[usize::try_from(place_operands[0]).expect("place index")] =
                    values[usize::try_from(operands[0]).expect("value index")].clone();
                None
            }
            VerifiedInstructionKind::I32Mul | VerifiedInstructionKind::I32Add => {
                let OracleValue::I32(lhs) =
                    values[usize::try_from(operands[0]).expect("lhs")].clone().expect("lhs value")
                else {
                    panic!("i32 lhs")
                };
                let OracleValue::I32(rhs) =
                    values[usize::try_from(operands[1]).expect("rhs")].clone().expect("rhs value")
                else {
                    panic!("i32 rhs")
                };
                Some(OracleValue::I32(if instruction.kind() == VerifiedInstructionKind::I32Mul {
                    lhs.wrapping_mul(rhs)
                } else {
                    lhs.wrapping_add(rhs)
                }))
            }
            other => panic!("unexpected Pair oracle instruction {other:?}"),
        };
        if let (Some(id), Some(value)) = (instruction.result(), result) {
            let index = usize::try_from(id.index()).expect("result index");
            if index >= values.len() {
                values.resize(index + 1, None);
            }
            values[index] = Some(value);
        }
    }
    assert_eq!(block.terminator().kind(), VerifiedTerminatorKind::Return);
    let returned = block.terminator().value_operands().next().expect("return value");
    let OracleValue::I32(value) = values[usize::try_from(returned.index()).expect("return index")]
        .clone()
        .expect("returned value")
    else {
        panic!("scalar return")
    };
    value
}

#[test]
fn normative_pair_score_matches_all_five_frozen_oracle_cases() {
    let sources = sources_for(PAIR_SCORE_SOURCE);
    let raw = decode_snapshot(PAIR_SCORE_JSON).expect("generated Pair score v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Pair score v4");
    let program = lower(pair_input(&syntax, &sources)).expect("Pair score must lower and verify");
    let function = program.modules().next().expect("module").functions().next().expect("pairScore");
    let oracle: serde_json::Value = serde_json::from_str(PAIR_ORACLE).expect("Pair oracle JSON");
    let cases = oracle["cases"].as_array().expect("oracle cases");
    assert_eq!(cases.len(), 5);
    for case in cases {
        let arguments = case["arguments"].as_array().expect("arguments");
        let left = i32::try_from(arguments[0]["value"].as_i64().expect("left")).expect("left i32");
        let right =
            i32::try_from(arguments[1]["value"].as_i64().expect("right")).expect("right i32");
        let expected = i32::try_from(case["expected"]["value"].as_i64().expect("expected"))
            .expect("expected i32");
        assert_eq!(evaluate_pair(function, [left, right]), expected, "{}", case["id"]);
    }
    let kinds = function
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(!kinds.iter().any(|kind| matches!(
        kind,
        VerifiedInstructionKind::MoveFromPlace
            | VerifiedInstructionKind::DropPlace
            | VerifiedInstructionKind::ClonePlace
    )));
}

#[test]
fn reversed_struct_fields_evaluate_and_construct_in_declaration_order() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(137..148, "right, left");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[0].kind else {
        panic!("first reference")
    };
    name.text = "right".to_owned();
    name.span.end = 142;
    expressions[0].span.end = 142;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[1].kind else {
        panic!("second reference")
    };
    name.text = "left".to_owned();
    name.span.start = 144;
    expressions[1].span.start = 144;
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("constructor")
    };
    fields[0].span.end = 142;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[0].kind
    else {
        panic!("first field")
    };
    name.text = "right".to_owned();
    name.span.end = 142;
    fields[1].span.start = 144;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[1].kind
    else {
        panic!("second field")
    };
    name.text = "left".to_owned();
    name.span.start = 144;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed fields");
    let program = lower(pair_input(&syntax, &sources)).expect("reversed fields must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let places = function.places().collect::<Vec<_>>();
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let copies = instructions
        .iter()
        .copied()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::CopyFromPlace)
        .take(2)
        .collect::<Vec<_>>();
    let first_place = copies[0].place_operands().next().expect("first source operand");
    let second_place = copies[1].place_operands().next().expect("second source operand");
    assert!(matches!(
        places[usize::try_from(first_place.index()).expect("place")].kind(),
        VerifiedPlaceKind::Parameter(0)
    ));
    assert!(matches!(
        places[usize::try_from(second_place.index()).expect("place")].kind(),
        VerifiedPlaceKind::Parameter(1)
    ));
    let construct = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StructConstruct)
        .expect("construct");
    let operands = construct
        .value_operands()
        .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(
        operands,
        vec![
            copies[0].result().expect("left result").index(),
            copies[1].result().expect("right result").index()
        ]
    );
}

#[test]
fn authenticated_input_rejects_another_source_authority() {
    let sources = pair_sources();
    let raw = decode_snapshot(PAIR_JSON).expect("Pair v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("Pair v4 authority");
    let other = pair_sources();
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = other.file_id(&path).expect("entry");
    assert!(SemanticInput::try_new(&syntax, &other, entry).is_none());
}

#[test]
fn non_copy_string_is_rejected_with_a_source_location() {
    let sources = sources_for(STRING_SOURCE);
    let syntax =
        verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("source-faithful v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("String is outside Copy M3");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3003");
    assert!(diagnostics[0].primary_span().is_some());
}

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
fn nonexhaustive_enum_match_is_rejected() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(114..137, "                       ");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &mut body.expressions[2].kind
    else {
        panic!("match")
    };
    arms.remove(0);
    arms[0].value = 1;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 2;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nonexhaustive match");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nonexhaustive match");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3009");
}

#[test]
fn v4_match_child_order_regression_authenticates_source_faithful_adapter_output() {
    let sources = sources_for(ENUM_SOURCE);
    verify_snapshot(response_snapshot(ENUM_RESPONSE), &sources)
        .expect("call-style match close parenthesis follows the complete arm object");
}

#[test]
fn fixed_array_constant_projection_accepts_last_index() {
    let sources = sources_for(ARRAY_VALID_SOURCE);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index literal")
    };
    *spelling = "1".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful fixed array v4");
    let program = lower(pair_input(&syntax, &sources)).expect("last fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 1, .. }
    )));
}

#[test]
fn fixed_array_constant_projection_accepts_zero_index() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "0".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful zero index");
    let program = lower(pair_input(&syntax, &sources)).expect("zero fixed index must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.places().any(|place| matches!(
        place.kind(),
        VerifiedPlaceKind::FixedArrayConstant { index: 0, .. }
    )));
}

#[test]
fn fixed_array_constructor_requires_exact_count_and_element_type() {
    let sources = sources_for(ARRAY_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), &sources)
        .expect("array constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact fixed-array constructor");

    let mut missing_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    missing_source.replace_range(66..69, "   ");
    let missing_sources = sources_for(&missing_source);
    let mut missing = response_snapshot(ARRAY_CONSTRUCT_RESPONSE);
    let body = &mut missing.files[0].functions[0].body;
    body.expressions.remove(1);
    let zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } =
        &mut body.expressions[1].kind
    else {
        panic!("array constructor")
    };
    elements.pop();
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 1;
    let syntax = verify_snapshot(missing, &missing_sources).expect("source-faithful short array");
    assert_eq!(
        lower(pair_input(&syntax, &missing_sources)).expect_err("short array")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ARRAY_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(65..66, "true");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ARRAY_CONSTRUCT_RESPONSE), 66, 3);
    typed.files[0].functions[0].body.expressions[0].kind =
        zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true };
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped array");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped array");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped element child");
    assert_eq!((primary.start(), primary.end()), (65, 69));
}

#[test]
fn fixed_array_index_equal_to_length_is_rejected() {
    let sources = sources_for(ARRAY_OOB_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ARRAY_RESPONSE), &sources)
        .expect("source-faithful fixed array v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("index N is out of bounds");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn struct_unknown_and_missing_field_is_rejected_at_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(137..141, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[0].kind else {
        panic!("left reference")
    };
    name.text = "nope".to_owned();
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("Pair constructor")
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[0].kind
    else {
        panic!("shorthand")
    };
    name.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unknown field");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown/missing field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
    assert_eq!(diagnostics[0].primary_span().expect("initializer span").start(), 137);
}

#[test]
fn duplicate_struct_field_is_rejected_at_later_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(143..148, "left ");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[1].kind else {
        panic!("right reference")
    };
    name.text = "left".to_owned();
    name.span.end = 147;
    expressions[1].span.end = 147;
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("Pair constructor")
    };
    fields[1].span.end = 147;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[1].kind
    else {
        panic!("shorthand")
    };
    name.text = "left".to_owned();
    name.span.end = 147;
    let diagnostics = verify_snapshot(raw, &sources)
        .expect_err("v4 rejects duplicate initializer names before semantics");
    assert_eq!(diagnostics[0].code(), "ZRYNA-Y4002");
}

#[test]
fn enum_payload_binding_mismatch_is_rejected() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(152..157, "     ");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } =
        &mut raw.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("match")
    };
    arms[1].binding = None;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful payload mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("payload binding mismatch");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3009");
}

#[test]
fn reversed_enum_match_arms_lower_in_variant_ordinal_order() {
    let mut source = ENUM_SOURCE.to_owned();
    source.replace_range(114..167, "\"Maybe.some\": (value) => value, \"Maybe.none\": () => 0");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let body = &mut raw.files[0].functions[0].body;
    body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 139, end: 144 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "value".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 139, end: 144 },
            },
        },
    };
    body.expressions[2] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 166, end: 167 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &mut body.expressions[3].kind
    else {
        panic!("match")
    };
    arms.swap(0, 1);
    let some = &mut arms[0];
    some.span = zryna_source::UntrustedSpan { file: 0, start: 114, end: 144 };
    some.type_name.span = zryna_source::UntrustedSpan { file: 0, start: 115, end: 120 };
    some.dot_span = zryna_source::UntrustedSpan { file: 0, start: 120, end: 121 };
    some.variant.span = zryna_source::UntrustedSpan { file: 0, start: 121, end: 125 };
    let binding = some.binding.as_mut().expect("some binding");
    binding.span = zryna_source::UntrustedSpan { file: 0, start: 129, end: 134 };
    some.arrow_span = zryna_source::UntrustedSpan { file: 0, start: 136, end: 138 };
    some.value = 1;
    let none = &mut arms[1];
    none.span = zryna_source::UntrustedSpan { file: 0, start: 146, end: 167 };
    none.type_name.span = zryna_source::UntrustedSpan { file: 0, start: 147, end: 152 };
    none.dot_span = zryna_source::UntrustedSpan { file: 0, start: 152, end: 153 };
    none.variant.span = zryna_source::UntrustedSpan { file: 0, start: 153, end: 157 };
    none.arrow_span = zryna_source::UntrustedSpan { file: 0, start: 163, end: 165 };
    none.value = 2;

    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed match");
    let program = lower(pair_input(&syntax, &sources)).expect("reversed match must lower");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let entry = function.blocks().next().expect("entry");
    assert_eq!(
        entry.terminator().enum_arms().map(|arm| arm.variant()).collect::<Vec<_>>(),
        vec![0, 1]
    );
}

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
fn enum_constructor_payload_shape_type_and_name_are_exact() {
    let sources = sources_for(ENUM_CONSTRUCT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(ENUM_CONSTRUCT_RESPONSE), &sources)
        .expect("enum constructor v4");
    lower(pair_input(&syntax, &sources)).expect("exact enum constructor");

    let mut absent_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    absent_source.replace_range(115..116, " ");
    let absent_sources = sources_for(&absent_source);
    let mut absent = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let body = &mut absent.files[0].functions[0].body;
    body.expressions.remove(0);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { payload, .. } =
        &mut body.expressions[0].kind
    else {
        panic!("enum constructor")
    };
    *payload = None;
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 0;
    let syntax = verify_snapshot(absent, &absent_sources).expect("source-faithful absent payload");
    assert_eq!(
        lower(pair_input(&syntax, &absent_sources)).expect_err("absent payload")[0].code(),
        "ZRYNA-M3005"
    );

    let mut extra_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    extra_source.replace_range(110..114, "none");
    let extra_sources = sources_for(&extra_source);
    let mut extra = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut extra.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    variant.text = "none".to_owned();
    let syntax = verify_snapshot(extra, &extra_sources).expect("source-faithful extra payload");
    assert_eq!(
        lower(pair_input(&syntax, &extra_sources)).expect_err("extra payload")[0].code(),
        "ZRYNA-M3005"
    );

    let mut typed_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    typed_source.replace_range(83..86, "bool");
    let typed_sources = sources_for(&typed_source);
    let mut typed = shift_snapshot(response_snapshot(ENUM_CONSTRUCT_RESPONSE), 86, 1);
    let RawTypeSyntaxKind::Named { name } = &mut typed.files[0].type_syntax[1].kind else {
        panic!("parameter type")
    };
    name.text = "bool".to_owned();
    let syntax = verify_snapshot(typed, &typed_sources).expect("source-faithful mistyped payload");
    let diagnostics = lower(pair_input(&syntax, &typed_sources)).expect_err("mistyped payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    let primary = diagnostics[0].primary_span().expect("mistyped payload child");
    assert_eq!((primary.start(), primary.end()), (116, 117));

    let mut variant_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    variant_source.replace_range(110..114, "nope");
    let variant_sources = sources_for(&variant_source);
    let mut variant_snapshot = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut variant_snapshot.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    variant.text = "nope".to_owned();
    let syntax = verify_snapshot(variant_snapshot, &variant_sources)
        .expect("source-faithful unknown variant");
    let diagnostics = lower(pair_input(&syntax, &variant_sources)).expect_err("unknown variant");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
    let primary = diagnostics[0].primary_span().expect("variant token");
    assert_eq!((primary.start(), primary.end()), (110, 114));

    let mut unknown_source = ENUM_CONSTRUCT_SOURCE.to_owned();
    unknown_source.replace_range(104..109, "Other");
    let unknown_sources = sources_for(&unknown_source);
    let mut unknown = response_snapshot(ENUM_CONSTRUCT_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { type_name, .. } =
        &mut unknown.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum constructor")
    };
    type_name.text = "Other".to_owned();
    let syntax = verify_snapshot(unknown, &unknown_sources).expect("source-faithful unknown enum");
    assert_eq!(
        lower(pair_input(&syntax, &unknown_sources)).expect_err("unknown enum")[0].code(),
        "ZRYNA-M3005"
    );
}

#[test]
fn dynamic_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "x");
    let sources = sources_for(&source);
    let mut raw = response_snapshot(ARRAY_RESPONSE);
    let span = raw.files[0].functions[0].body.expressions[1].span;
    raw.files[0].functions[0].body.expressions[1].kind =
        zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "x".to_owned(), span },
        };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful dynamic index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("dynamic index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("dynamic index child");
    assert_eq!((primary.start(), primary.end()), (54, 55));
}

#[test]
fn negative_fixed_array_index_is_rejected() {
    let mut source = ARRAY_OOB_SOURCE.to_owned();
    source.replace_range(54..55, "-1");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 55, 1);
    let body = &mut raw.files[0].functions[0].body;
    let mut index = body.expressions.pop().expect("index expression");
    let literal = &mut body.expressions[1];
    literal.span.start = 55;
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } = &mut literal.kind else {
        panic!("literal")
    };
    *spelling = "1".to_owned();
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 56 },
        kind: zryna_syntax::v4::RawExpressionKind::Negation {
            operator_span: zryna_source::UntrustedSpan { file: 0, start: 54, end: 55 },
            operand: 1,
        },
    });
    let zryna_syntax::v4::RawExpressionKind::Index { index: index_id, .. } = &mut index.kind else {
        panic!("index")
    };
    *index_id = 2;
    body.expressions.push(index);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = 3;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful negative index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("negative index");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let primary = diagnostics[0].primary_span().expect("negative index child");
    assert_eq!((primary.start(), primary.end()), (54, 56));
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

#[test]
fn unknown_field_access_is_rejected_at_the_use_not_declaration() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(165..169, "nope");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let zryna_syntax::v4::RawExpressionKind::FieldAccess { field, .. } =
        &mut raw.files[0].functions[0].body.expressions[4].kind
    else {
        panic!("field access")
    };
    field.text = "nope".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful unknown field access");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    assert_eq!(diagnostics[0].primary_span().expect("field use").start(), 165);
}

#[test]
fn by_value_recursive_struct_is_rejected_by_layout_authority() {
    let mut source = PAIR_SOURCE.to_owned();
    source.replace_range(43..46, "Pair");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(decode_snapshot(PAIR_JSON).expect("Pair JSON"), 46, 1);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[0].kind else {
        panic!("field type")
    };
    name.text = "Pair".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful recursive Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("by-value recursion");
    assert!(diagnostics[0].code().starts_with("ZRYNA-L3"));
}

#[test]
fn fixed_array_mediated_recursive_struct_is_rejected_by_layout_authority() {
    let text = "interface Loop extends ZrynaStruct { items: FixedArray<Loop, 1>; }\n";
    let sources = sources_for(text);
    let span = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        diagnostics: Vec::new(),
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![
                RawTypeSyntax {
                    span: span(55, 59),
                    kind: RawTypeSyntaxKind::Named {
                        name: RawIdentifierSyntax { text: "Loop".to_owned(), span: span(55, 59) },
                    },
                },
                RawTypeSyntax {
                    span: span(44, 63),
                    kind: RawTypeSyntaxKind::FixedArray {
                        keyword_span: span(44, 54),
                        less_than_span: span(54, 55),
                        element: 0,
                        comma_span: span(59, 60),
                        length_span: span(61, 62),
                        length_spelling: "1".to_owned(),
                        length: 1,
                        greater_than_span: span(62, 63),
                    },
                },
            ],
            data_declarations: vec![RawDataDeclaration {
                span: span(0, 66),
                export_span: None,
                kind: RawDataDeclarationKind::Struct {
                    interface_span: span(0, 9),
                    name: RawIdentifierSyntax { text: "Loop".to_owned(), span: span(10, 14) },
                    extends_span: span(15, 22),
                    marker_span: span(23, 34),
                    open_brace_span: span(35, 36),
                    fields: vec![RawDataField {
                        span: span(37, 64),
                        name: RawIdentifierSyntax { text: "items".to_owned(), span: span(37, 42) },
                        colon_span: span(42, 43),
                        type_syntax: 1,
                        semicolon_span: span(63, 64),
                    }],
                    close_brace_span: span(65, 66),
                },
            }],
            functions: Vec::new(),
        }],
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array recursion");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("array-mediated recursion");
    assert!(diagnostics[0].code().starts_with("ZRYNA-L3"));
}

#[test]
fn public_aggregate_result_is_rejected() {
    let mut source = PAIR_SOURCE.to_owned();
    source.insert_str(50, "export ");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(decode_snapshot(PAIR_JSON).expect("Pair JSON"), 50, 7);
    let function = &mut raw.files[0].functions[0];
    function.span.start = 50;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 50, end: 56 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public aggregate result");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
}

#[test]
fn public_aggregate_parameter_is_rejected() {
    let mut source = ARRAY_VALID_SOURCE.to_owned();
    source.insert_str(0, "export ");
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(ARRAY_RESPONSE), 0, 7);
    let zryna_syntax::v4::RawExpressionKind::I32Literal { spelling } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("index")
    };
    *spelling = "1".to_owned();
    let function = &mut raw.files[0].functions[0];
    function.span.start = 0;
    function.export_span = Some(zryna_source::UntrustedSpan { file: 0, start: 0, end: 6 });
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful exported array parameter");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("public aggregate parameter");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
}

#[test]
fn mistyped_struct_initializer_is_rejected_at_the_initializer() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(99..102, "bool");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 102, 1);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[3].kind else {
        panic!("right parameter type")
    };
    name.text = "bool".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful mistyped Pair");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mistyped initializer");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3007");
    assert!(diagnostics[0].primary_span().is_some());
}

#[test]
fn portable_field_name_collision_is_rejected() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(48..53, "LEFT ");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let RawDataDeclarationKind::Struct { fields, .. } = &mut raw.files[0].data_declarations[0].kind
    else {
        panic!("Pair struct")
    };
    fields[1].name.text = "LEFT".to_owned();
    fields[1].name.span.end = 52;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful case collision");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("portable collision");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].primary_span().expect("later field").start(), 48);
}

#[test]
fn semantic_diagnostics_replay_deterministically() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("first rejection");
    let second = lower(pair_input(&syntax, &sources)).expect_err("second rejection");
    let summarize = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|d| {
                (
                    d.code().to_owned(),
                    d.primary_span().map(|s| (s.start(), s.end())),
                    d.message().to_owned(),
                    d.guidance().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(summarize(&first), summarize(&second));
}

#[test]
fn duplicate_return_is_rejected_at_the_second_return() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.insert_str(189, "return 0; ");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 189, 10);
    let body = &mut raw.files[0].functions[0].body;
    let value = u32::try_from(body.expressions.len()).expect("expression id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 196, end: 197 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 198 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 195 },
            value,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 197, end: 198 },
        },
    });
    body.blocks[0].statements.push(statement);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful duplicate return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("duplicate return");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(diagnostics[0].primary_span().expect("second return").start(), 189);
}

#[test]
fn mutation_after_return_is_rejected_before_lowering_the_mutation() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.insert_str(189, "pair.left = 0; ");
    let sources = sources_for(&source);
    let mut raw =
        shift_snapshot(decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON"), 189, 15);
    let body = &mut raw.files[0].functions[0].body;
    let reference = u32::try_from(body.expressions.len()).expect("reference id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 193 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "pair".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 193 },
            },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("target id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 198 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: reference,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: 193, end: 194 },
            field: RawIdentifierSyntax {
                text: "left".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 194, end: 198 },
            },
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("value id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 201, end: 202 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 189, end: 203 },
        kind: RawStatementKind::Assignment {
            target,
            equals_span: zryna_source::UntrustedSpan { file: 0, start: 199, end: 200 },
            value,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 202, end: 203 },
        },
    });
    body.blocks[0].statements.push(statement);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful post-return mutation");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mutation after return");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(diagnostics[0].primary_span().expect("mutation span").start(), 189);
}

fn empty_declaration_file(file: u32, first: usize, count: usize) -> (String, RawSourceUnit) {
    let mut text = String::new();
    let mut declarations = Vec::with_capacity(count);
    let mut types = Vec::with_capacity(count);
    for number in first..first + count {
        let name = format!("T{number}");
        let start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str("interface ");
        let name_start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(&name);
        let name_end = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(" extends ZrynaStruct { x: i32; }\n");
        let end = u32::try_from(text.len() - 1).expect("fixture offset");
        let extends_start = name_end + 1;
        let marker_start = extends_start + 8;
        let open = marker_start + 12;
        let field_start = open + 2;
        let colon = field_start + 1;
        let type_start = colon + 2;
        let semicolon = type_start + 3;
        let type_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file, start: type_start, end: type_start + 3 },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file,
                        start: type_start,
                        end: type_start + 3,
                    },
                },
            },
        });
        declarations.push(RawDataDeclaration {
            span: zryna_source::UntrustedSpan { file, start, end },
            export_span: None,
            kind: RawDataDeclarationKind::Struct {
                interface_span: zryna_source::UntrustedSpan { file, start, end: start + 9 },
                name: RawIdentifierSyntax {
                    text: name,
                    span: zryna_source::UntrustedSpan { file, start: name_start, end: name_end },
                },
                extends_span: zryna_source::UntrustedSpan {
                    file,
                    start: extends_start,
                    end: extends_start + 7,
                },
                marker_span: zryna_source::UntrustedSpan {
                    file,
                    start: marker_start,
                    end: marker_start + 11,
                },
                open_brace_span: zryna_source::UntrustedSpan { file, start: open, end: open + 1 },
                fields: vec![RawDataField {
                    span: zryna_source::UntrustedSpan {
                        file,
                        start: field_start,
                        end: semicolon + 1,
                    },
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file,
                            start: field_start,
                            end: field_start + 1,
                        },
                    },
                    colon_span: zryna_source::UntrustedSpan { file, start: colon, end: colon + 1 },
                    type_syntax: type_id,
                    semicolon_span: zryna_source::UntrustedSpan {
                        file,
                        start: semicolon,
                        end: semicolon + 1,
                    },
                }],
                close_brace_span: zryna_source::UntrustedSpan {
                    file,
                    start: semicolon + 2,
                    end: semicolon + 3,
                },
            },
        });
    }
    let path = if file == 0 { "src/a.zry" } else { "src/b.zry" };
    (
        text,
        RawSourceUnit {
            id: file,
            path: path.to_owned(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: declarations,
            functions: Vec::new(),
        },
    )
}

#[test]
fn nominal_declaration_budget_is_exact_and_plus_one_fails_m3201() {
    let (exact_text, exact_file) =
        empty_declaration_file(0, 0, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS);
    let exact_sources =
        SourceMap::build(vec![SourceFileInput { path: "src/a.zry".to_owned(), text: exact_text }])
            .expect("exact source map");
    let exact_syntax = verify_snapshot(
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![exact_file],
            diagnostics: Vec::new(),
        },
        &exact_sources,
    )
    .expect("exact v4 budget");
    let entry_path = NormalizedSourcePath::new("src/a.zry").expect("entry path");
    let entry = exact_sources.file_id(&entry_path).expect("entry");
    lower(SemanticInput::try_new(&exact_syntax, &exact_sources, entry).expect("exact input"))
        .expect("the exact nominal declaration budget must verify");

    let (first_text, first_file) =
        empty_declaration_file(0, 0, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS);
    let (last_text, last_file) =
        empty_declaration_file(1, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS, 1);
    let plus_sources = SourceMap::build(vec![
        SourceFileInput { path: "src/a.zry".to_owned(), text: first_text },
        SourceFileInput { path: "src/b.zry".to_owned(), text: last_text },
    ])
    .expect("plus-one source map");
    let plus_syntax = verify_snapshot(
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![first_file, last_file],
            diagnostics: Vec::new(),
        },
        &plus_sources,
    )
    .expect("plus-one v4 budget");
    let entry = plus_sources.file_id(&entry_path).expect("entry");
    let plus =
        lower(SemanticInput::try_new(&plus_syntax, &plus_sources, entry).expect("plus-one input"))
            .expect_err("M3 nominal limit must fail");
    assert_eq!(plus[0].code(), "ZRYNA-M3201");
}

fn derived_value_fixture(result_count: usize) -> RawFunctionSyntax {
    assert!(result_count > 0);
    let span = zryna_source::UntrustedSpan { file: 0, start: 0, end: 1 };
    let child_count = result_count - 1;
    let mut expressions = (0..child_count)
        .map(|_| RawExpressionSyntax {
            span,
            kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
        })
        .collect::<Vec<_>>();
    expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: 0,
            open_paren_span: span,
            open_bracket_span: span,
            elements: (0..u32::try_from(child_count).expect("fixture expression ids")).collect(),
            close_bracket_span: span,
            close_paren_span: span,
        },
    });
    RawFunctionSyntax {
        span,
        export_span: None,
        function_span: span,
        name: RawIdentifierSyntax { text: "budget".to_owned(), span },
        parameters: Vec::new(),
        result_type: 0,
        body: RawFunctionBodySyntax {
            span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span,
                open_brace_span: span,
                statements: vec![0],
                close_brace_span: span,
            }],
            statements: vec![RawStatementSyntax {
                span,
                kind: RawStatementKind::Return {
                    keyword_span: span,
                    value: u32::try_from(child_count).expect("fixture root id"),
                    semicolon_span: span,
                },
            }],
            expressions,
        },
    }
}

#[allow(clippy::too_many_lines)]
fn authenticated_value_budget_fixture(with_parameter: bool) -> (String, RawProjectSyntaxSnapshot) {
    fn offset(text: &str) -> u32 {
        u32::try_from(text.len()).expect("fixture offset")
    }
    fn fixed_array_type(text: &mut String, types: &mut Vec<RawTypeSyntax>, length: usize) -> u32 {
        let start = offset(text);
        text.push_str("FixedArray<");
        let element_start = offset(text);
        text.push_str("i32");
        let element_end = offset(text);
        let element = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: element_start, end: element_end },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: element_start,
                        end: element_end,
                    },
                },
            },
        });
        let comma = offset(text);
        text.push_str(", ");
        let length_start = offset(text);
        let spelling = length.to_string();
        text.push_str(&spelling);
        let length_end = offset(text);
        let greater = offset(text);
        text.push('>');
        let end = offset(text);
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start, end },
            kind: RawTypeSyntaxKind::FixedArray {
                keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 10 },
                less_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 10,
                    end: start + 11,
                },
                element,
                comma_span: zryna_source::UntrustedSpan { file: 0, start: comma, end: comma + 1 },
                length_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: length_start,
                    end: length_end,
                },
                length: u32::try_from(length).expect("array length"),
                length_spelling: spelling,
                greater_than_span: zryna_source::UntrustedSpan { file: 0, start: greater, end },
            },
        });
        id
    }

    let mut text = "function budget(".to_owned();
    let mut types = Vec::new();
    let mut parameters = Vec::new();
    if with_parameter {
        let parameter_start = offset(&text);
        text.push_str("x: ");
        let type_start = offset(&text);
        text.push_str("i32");
        let type_end = offset(&text);
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: type_start, end: type_end },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start: type_start, end: type_end },
                },
            },
        });
        parameters.push(RawParameterSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: parameter_start, end: type_end },
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: parameter_start,
                    end: parameter_start + 1,
                },
            },
            type_syntax: 0,
        });
    }
    text.push_str("): ");
    let result_start = offset(&text);
    text.push_str("i32");
    let result_end = offset(&text);
    let result_type = u32::try_from(types.len()).expect("result type id");
    types.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: result_start, end: result_end },
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "i32".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: result_start, end: result_end },
            },
        },
    });
    text.push_str(" { ");
    let body_start = result_end + 1;
    let mut expressions = Vec::with_capacity(zryna_syntax::v4::MAX_EXPRESSIONS_PER_FUNCTION);
    let mut statements = Vec::with_capacity(5);
    for (local, element_count) in [4_095_usize, 4_095, 4_095, 4_094].into_iter().enumerate() {
        let statement_start = offset(&text);
        text.push_str("const ");
        let name_start = offset(&text);
        let name = format!("a{local}");
        text.push_str(&name);
        let name_end = offset(&text);
        text.push_str(": ");
        let declared_type = fixed_array_type(&mut text, &mut types, element_count);
        text.push(' ');
        let equals = offset(&text);
        text.push_str("= ");
        let constructor_start = offset(&text);
        let constructor_type = fixed_array_type(&mut text, &mut types, element_count);
        let open_paren = offset(&text);
        text.push_str("([");
        let open_bracket = open_paren + 1;
        let first_element = u32::try_from(expressions.len()).expect("first element id");
        for index in 0..element_count {
            if index > 0 {
                text.push_str(", ");
            }
            let start = offset(&text);
            text.push('0');
            expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
            });
        }
        let close_bracket = offset(&text);
        text.push_str("])");
        let close_paren = close_bracket + 1;
        let constructor_end = offset(&text);
        let initializer = u32::try_from(expressions.len()).expect("constructor id");
        expressions.push(RawExpressionSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: constructor_start,
                end: constructor_end,
            },
            kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
                type_syntax: constructor_type,
                open_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: open_paren,
                    end: open_paren + 1,
                },
                open_bracket_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: open_bracket,
                    end: open_bracket + 1,
                },
                elements: (first_element..initializer).collect(),
                close_bracket_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: close_bracket,
                    end: close_bracket + 1,
                },
                close_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: close_paren,
                    end: close_paren + 1,
                },
            },
        });
        let semicolon = offset(&text);
        text.push_str("; ");
        statements.push(RawStatementSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: statement_start,
                end: semicolon + 1,
            },
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: statement_start,
                    end: statement_start + 5,
                },
                mutable: false,
                name: RawIdentifierSyntax {
                    text: name,
                    span: zryna_source::UntrustedSpan { file: 0, start: name_start, end: name_end },
                },
                type_syntax: declared_type,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: equals,
                    end: equals + 1,
                },
                initializer,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: semicolon,
                    end: semicolon + 1,
                },
            },
        });
    }
    let return_start = offset(&text);
    text.push_str("return ");
    let value_start = offset(&text);
    text.push('0');
    let returned = u32::try_from(expressions.len()).expect("return value id");
    expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: value_start, end: value_start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let semicolon = offset(&text);
    text.push_str("; }");
    statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_start, end: semicolon + 1 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan {
                file: 0,
                start: return_start,
                end: return_start + 6,
            },
            value: returned,
            semicolon_span: zryna_source::UntrustedSpan {
                file: 0,
                start: semicolon,
                end: semicolon + 1,
            },
        },
    });
    let end = offset(&text);
    let body_end = end;
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: Vec::new(),
            functions: vec![RawFunctionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 0, end },
                export_span: None,
                function_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 8 },
                name: RawIdentifierSyntax {
                    text: "budget".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start: 9, end: 15 },
                },
                parameters,
                result_type,
                body: RawFunctionBodySyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: body_start, end: body_end },
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_start,
                            end: body_end,
                        },
                        open_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_start,
                            end: body_start + 1,
                        },
                        statements: (0..u32::try_from(statements.len()).expect("statement ids"))
                            .collect(),
                        close_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_end - 1,
                            end: body_end,
                        },
                    }],
                    statements,
                    expressions,
                },
            }],
        }],
        diagnostics: Vec::new(),
    };
    (text, raw)
}

#[test]
fn derived_ir_value_budgets_are_exact_and_plus_one_is_rejected() {
    let per_function = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    assert_eq!(derived_value_count(&derived_value_fixture(per_function)), per_function);
    assert_eq!(derived_value_count(&derived_value_fixture(per_function + 1)), per_function + 1);

    assert_eq!(value_budget_violation(0, per_function), None);
    assert_eq!(value_budget_violation(0, per_function + 1), Some(ValueBudgetLimit::Function));
    let per_program = zryna_ir::data_ownership_v1::MAX_VALUES_PER_PROGRAM;
    assert_eq!(value_budget_violation(per_program - per_function, per_function), None);
    assert_eq!(
        value_budget_violation(per_program - per_function + 1, per_function),
        Some(ValueBudgetLimit::Program)
    );
    assert_eq!(value_budget_violation(usize::MAX, per_function), Some(ValueBudgetLimit::Program));
}

#[test]
#[ignore = "authenticated exact/first-extra boundary runs in the full M3 preflight gate"]
fn authenticated_v4_derived_value_budget_is_exact_and_plus_one_fails_m3201() {
    let (exact_text, exact_raw) = authenticated_value_budget_fixture(false);
    let exact_sources = sources_for(&exact_text);
    let exact_syntax = verify_snapshot(exact_raw, &exact_sources).expect("exact value-budget v4");
    let exact_input = pair_input(&exact_syntax, &exact_sources);
    let mut exact_errors = Errors::new(&exact_sources);
    semantic_preflight(exact_input, &mut exact_errors);
    assert!(exact_errors.finish().is_empty(), "exact value budget must pass preflight");

    let (plus_text, plus_raw) = authenticated_value_budget_fixture(true);
    let plus_sources = sources_for(&plus_text);
    let plus_syntax = verify_snapshot(plus_raw, &plus_sources).expect("plus-one value-budget v4");
    let mut plus_errors = Errors::new(&plus_sources);
    semantic_preflight(pair_input(&plus_syntax, &plus_sources), &mut plus_errors);
    let diagnostics = plus_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    let primary = diagnostics[0].primary_span().expect("function source span");
    assert_eq!(
        (primary.start(), primary.end()),
        (0, u32::try_from(plus_text.len()).expect("fixture length"))
    );
}
