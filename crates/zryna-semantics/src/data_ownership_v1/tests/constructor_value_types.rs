use super::super::owned_constructor_plan::{ConstructorPlanError, ConstructorValueTypes};
use super::*;
use zryna_syntax::v4::RawExpressionKind;

#[test]
fn constructor_value_cache_rejects_gaps_duplicates_and_order_before_append_then_recovers() {
    let sources = sources_for("x");
    let at = sources.span(sources.verify_file_id(0).expect("file"), 0, 1).expect("span");
    let definition =
        |index| raw::ValueDefinition { id: raw::ValueId(index), ty: raw::TypeId(index), span: at };
    let instruction = |index| raw::Instruction {
        result: Some(definition(index)),
        span: at,
        kind: raw::InstructionKind::I32Literal(0),
    };
    let mut cache = ConstructorValueTypes::default();
    assert!(cache.record_parameter(&definition(1)).is_err());
    assert_eq!(cache, ConstructorValueTypes::default());
    cache.record_parameter(&definition(0)).expect("dense parameter");
    assert!(cache.record_parameter(&definition(0)).is_err());
    for ids in [[1, 3], [1, 1], [2, 1]] {
        assert_eq!(cache.observe(&ids.map(instruction)), Err(ConstructorPlanError::WrongShape));
        assert_eq!(cache.get(raw::ValueId(0)), Some(raw::TypeId(0)));
        assert_eq!(cache.get(raw::ValueId(1)), None, "no partial append");
    }
    let mut instructions = vec![instruction(1), instruction(2)];
    assert_eq!(cache.observe(&instructions), Ok(2));
    assert_eq!(cache.observe(&instructions), Ok(0), "no repeated scan");
    assert!(cache.record_parameter(&definition(3)).is_err());
    assert!(cache.observe(&instructions[..1]).is_err(), "append-only arena");
    assert_eq!(cache.get(raw::ValueId(2)), Some(raw::TypeId(2)));
    instructions.push(raw::Instruction {
        result: None,
        span: at,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    });
    instructions.push(instruction(3));
    assert_eq!(cache.observe(&instructions), Ok(2), "only new suffix scanned");
    assert_eq!(cache.observe(&instructions), Ok(0));
    assert_eq!(cache.get(raw::ValueId(3)), Some(raw::TypeId(3)));
    assert_eq!(cache.get(raw::ValueId(4)), None);
}

fn parameter_constructor() -> (String, RawProjectSyntaxSnapshot) {
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    let insertion =
        u32::try_from(source.find("()").expect("parameter list") + 1).expect("parameter offset");
    source.insert_str(insertion as usize, "flag: bool");
    let mut raw = shift_snapshot(response_snapshot(OWNED_PAIR_RESPONSE), insertion, 10);
    let at = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    raw.files[0].type_syntax.insert(
        2,
        RawTypeSyntax {
            span: at(insertion + 6, insertion + 10),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "bool".to_owned(),
                    span: at(insertion + 6, insertion + 10),
                },
            },
        },
    );
    let function = &mut raw.files[0].functions[0];
    function.result_type += 1;
    function.parameters.push(RawParameterSyntax {
        span: at(insertion, insertion + 10),
        name: RawIdentifierSyntax { text: "flag".to_owned(), span: at(insertion, insertion + 4) },
        type_syntax: 2,
    });
    for statement in &mut function.body.statements {
        if let RawStatementKind::LocalDeclaration { type_syntax, .. } = &mut statement.kind {
            *type_syntax += 1;
        }
    }
    let literal = function
        .body
        .expressions
        .iter_mut()
        .find(|expression| matches!(expression.kind, RawExpressionKind::BoolLiteral { .. }))
        .expect("bool field");
    source.replace_range(literal.span.start as usize..literal.span.end as usize, "flag");
    literal.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "flag".to_owned(), span: literal.span },
    };
    (source, raw)
}

#[test]
fn constructor_dense_lookup_handles_parameters_and_direct_partial_transfer_emission() {
    for (source, snapshot) in
        [parameter_constructor(), owned_pair_partial_local_transfer_snapshot()]
    {
        let sources = sources_for(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("authenticated source constructor");
        let program = lower(pair_input(&syntax, &sources)).expect("full constructor verification");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let count = function
            .blocks()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::StructConstruct)
            .count();
        assert_eq!(count, if source.contains("flag: bool)") { 1 } else { 2 });
        assert!(lower(pair_input(&syntax, &sources)).is_ok(), "deterministic valid replay");
    }
}
