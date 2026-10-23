use super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use typed::continuation_test_support::{span, value};
#[derive(Clone, Copy)]
pub(super) enum Case {
    Single,
    Reordered,
    TwoHop,
}
pub(super) fn input(case: Case, appended: bool) -> (raw::Program, TypeWorld) {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("String");
    let world = builder.seal(&mut 0).expect("checked original String world");
    let mut p = typed::continuation_test_support::push(
        &typed::continuation_test_support::Types::new(),
        None,
    );
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters = (0..3).map(|id| raw::Parameter::Value(value(id, string))).collect();
    f.result_type = raw::ValueType::Unit;
    f.places = (0..3)
        .map(|id| raw::Place { id, ty: string, span: span(), kind: raw::PlaceKind::Parameter(id) })
        .collect();
    let arguments = if matches!(case, Case::Reordered) { vec![2, 0] } else { vec![0] };
    let mut parameters = Vec::new();
    for offset in 0..arguments.len() {
        let id = 3 + u32::try_from(offset).expect("small fixture");
        parameters.push(value(id, string));
        f.places.push(raw::Place {
            id,
            ty: string,
            span: span(),
            kind: raw::PlaceKind::Temporary(id),
        });
    }
    f.blocks = vec![
        raw::Block {
            id: 0,
            value_parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: raw::Terminator::Jump(raw::Edge { target: 1, arguments }),
        },
        raw::Block {
            id: 1,
            value_parameters: parameters,
            instructions: Vec::new(),
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        },
    ];
    let expected = match case {
        Case::Single => {
            if appended {
                vec![3, 2, 1]
            } else {
                vec![2, 1, 3]
            }
        }
        Case::Reordered => vec![3, 1, 4],
        Case::TwoHop => {
            f.places.push(raw::Place {
                id: 4,
                ty: string,
                span: span(),
                kind: raw::PlaceKind::Temporary(4),
            });
            f.blocks[1].terminator =
                raw::Terminator::Jump(raw::Edge { target: 2, arguments: vec![3] });
            f.blocks.push(raw::Block {
                id: 2,
                value_parameters: vec![value(4, string)],
                instructions: Vec::new(),
                terminator: raw::Terminator::Return { value: None, cleanup: 0 },
            });
            vec![2, 1, 4]
        }
    };
    f.cleanup_plans = vec![raw::CleanupPlan {
        id: 0,
        span: span(),
        actions: expected.into_iter().map(raw::CleanupAction::DropPlace).collect(),
    }];
    (p, world)
}
