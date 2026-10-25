use super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use typed::continuation_test_support::{instruction, span, value};
#[derive(Clone, Copy)]
pub(super) enum Case {
    ReadMove,
    ReadThenInstall,
    FreshInstall,
    ReplaceEarly,
    ReplaceLate,
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
    f.blocks[0].instructions.clear();
    let expected = match case {
        Case::ReadMove | Case::ReadThenInstall => {
            f.places.push(raw::Place {
                id: 3,
                ty: string,
                span: span(),
                kind: raw::PlaceKind::Temporary(3),
            });
            let mut take =
                instruction(raw::Operation::ReadPlace { place: 0, mode: raw::ReadMode::Move });
            take.result = Some(value(3, string));
            f.blocks[0].instructions.push(take);
            if matches!(case, Case::ReadThenInstall) {
                f.places.push(raw::Place {
                    id: 4,
                    ty: string,
                    span: span(),
                    kind: raw::PlaceKind::Local(0),
                });
                f.blocks[0]
                    .instructions
                    .push(instruction(raw::Operation::InitializePlace { place: 4, value: 3 }));
                vec![2, 1, 4]
            } else {
                vec![2, 1, 3]
            }
        }
        Case::FreshInstall => {
            f.places.push(raw::Place {
                id: 3,
                ty: string,
                span: span(),
                kind: raw::PlaceKind::Local(0),
            });
            f.blocks[0]
                .instructions
                .push(instruction(raw::Operation::InitializePlace { place: 3, value: 0 }));
            if appended { vec![3, 2, 1] } else { vec![2, 1, 3] }
        }
        Case::ReplaceEarly | Case::ReplaceLate => {
            let (place, rhs) = if matches!(case, Case::ReplaceEarly) { (1, 0) } else { (0, 1) };
            f.blocks[0]
                .instructions
                .push(instruction(raw::Operation::ReplacePlace { place, value: rhs }));
            vec![2, place]
        }
    };
    f.cleanup_plans = vec![raw::CleanupPlan {
        id: 0,
        span: span(),
        actions: expected.into_iter().map(raw::CleanupAction::DropPlace).collect(),
    }];
    f.blocks[0].terminator = raw::Terminator::Return { value: None, cleanup: 0 };
    (p, world)
}
