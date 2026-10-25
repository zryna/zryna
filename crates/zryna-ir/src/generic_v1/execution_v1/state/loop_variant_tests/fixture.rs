use super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
pub(super) use typed::continuation_test_support::{instruction, span, value};

pub(super) fn whole() -> (raw::Program, TypeWorld) {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("String");
    let boolean = builder.declare(Kind::Bool, &[], &mut 0).expect("bool");
    let en = builder.declare(Kind::Option, &[string], &mut 0).expect("Option");
    let world = builder.seal(&mut 0).expect("checked source type world");
    let mut p = typed::continuation_test_support::push(
        &typed::continuation_test_support::Types::new(),
        None,
    );
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters =
        vec![raw::Parameter::Value(value(0, string)), raw::Parameter::Value(value(1, boolean))];
    f.result_type = raw::ValueType::Unit;
    f.places = vec![
        raw::Place { id: 0, ty: string, span: span(), kind: raw::PlaceKind::Parameter(0) },
        raw::Place { id: 1, ty: en, span: span(), kind: raw::PlaceKind::Temporary(2) },
        raw::Place { id: 2, ty: en, span: span(), kind: raw::PlaceKind::Local(0) },
        raw::Place { id: 3, ty: en, span: span(), kind: raw::PlaceKind::Temporary(3) },
    ];
    let make = |id, ordinal, payload| raw::Instruction {
        site_span: span(),
        result: Some(value(id, en)),
        operation: raw::Operation::ClosedEnumConstruct { ty: en, ordinal, payload },
    };
    let jump = || raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![] });
    f.blocks = vec![
        raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![
                make(2, 1, Some(0)),
                instruction(raw::Operation::InitializePlace { place: 2, value: 2 }),
            ],
            terminator: jump(),
        },
        raw::Block {
            id: 1,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::Branch {
                condition: 1,
                yes: raw::Edge { target: 2, arguments: vec![] },
                no: raw::Edge { target: 3, arguments: vec![] },
            },
        },
        raw::Block {
            id: 2,
            value_parameters: vec![],
            instructions: vec![
                make(3, 0, None),
                instruction(raw::Operation::ReplacePlace { place: 2, value: 3 }),
            ],
            terminator: jump(),
        },
        raw::Block {
            id: 3,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        },
    ];
    f.loans.clear();
    f.cleanup_plans = vec![raw::CleanupPlan {
        id: 0,
        span: span(),
        actions: vec![raw::CleanupAction::DropPlace(2)],
    }];
    (p, world)
}

pub(super) fn partial() -> (raw::Program, TypeWorld) {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("String");
    let boolean = builder.declare(Kind::Bool, &[], &mut 0).expect("bool");
    let pair = builder
        .declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
        .expect("pair");
    builder.set_members(pair, &[Some(string), Some(string)], &mut 0).expect("pair fields");
    let en = builder.declare(Kind::Option, &[pair], &mut 0).expect("Option");
    let world = builder.seal(&mut 0).expect("checked partial enum world");
    let (mut p, _) = whole();
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters =
        vec![raw::Parameter::Value(value(0, pair)), raw::Parameter::Value(value(1, boolean))];
    f.result_type = raw::ValueType::Stored(string);
    f.places[0].ty = pair;
    for place in &mut f.places[1..] {
        place.ty = en;
    }
    f.places[3].ty = pair;
    f.places[3].kind = raw::PlaceKind::EnumPayload { base: 2, variant: 1 };
    f.places.extend([
        raw::Place {
            id: 4,
            ty: string,
            span: span(),
            kind: raw::PlaceKind::StructField { base: 3, ordinal: 0 },
        },
        raw::Place {
            id: 5,
            ty: string,
            span: span(),
            kind: raw::PlaceKind::StructField { base: 3, ordinal: 1 },
        },
        raw::Place { id: 6, ty: string, span: span(), kind: raw::PlaceKind::Temporary(3) },
    ]);
    let initial = &mut f.blocks[0].instructions[0];
    initial.result = Some(value(2, en));
    initial.operation =
        raw::Operation::ClosedEnumConstruct { ty: en, ordinal: 1, payload: Some(0) };
    f.blocks[0].instructions.push(raw::Instruction {
        site_span: span(),
        result: Some(value(3, string)),
        operation: raw::Operation::ReadPlace { place: 4, mode: raw::ReadMode::Move },
    });
    f.blocks[2].instructions.clear();
    f.blocks[3].terminator = raw::Terminator::Return { value: Some(3), cleanup: 0 };
    f.cleanup_plans[0].actions = vec![raw::CleanupAction::DropPlace(5)];
    (p, world)
}
