use super::*;

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
fn pair_oracle_lowers_to_sealed_copy_aggregate_ir() {
    let sources = pair_sources();
    let raw = decode_snapshot(PAIR_JSON).expect("Pair v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("Pair v4 authority");

    let program = lower(pair_input(&syntax, &sources)).expect("Pair must lower and verify");

    assert_eq!(program.modules().len(), 1);
    assert_eq!(
        program.runtime_abi().type_universe_identity(),
        program.verified_ir().type_universe_identity()
    );
    assert_eq!(
        program.runtime_abi().linear32_fingerprint(),
        *program.verified_ir().linear32_layouts().fingerprint()
    );
    assert_eq!(
        program.runtime_abi().linux_x86_64_fingerprint(),
        *program.verified_ir().linux_x86_64_layouts().fingerprint()
    );
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
