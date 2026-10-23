use super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use typed::continuation_test_support::{instruction, span, value};
#[derive(Clone, Copy)]
pub(super) enum Case {
    Struct,
    Array,
    OutOfOrder,
    NestedEnum,
}
pub(super) fn input(case: Case, reinitialize: bool, drop_root: bool) -> (raw::Program, TypeWorld) {
    let mut b = Builder::new();
    let string = b.declare(Kind::String, &[], &mut 0).expect("String");
    let aggregate = if matches!(case, Case::Array) {
        b.declare(Kind::FixedArray { length: 2 }, &[string], &mut 0).expect("array")
    } else {
        let id = b
            .declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
            .expect("nominal Struct");
        b.set_members(id, &[Some(string), Some(string)], &mut 0)
            .expect("exact declaration members");
        id
    };
    let en = if matches!(case, Case::NestedEnum) {
        let id = b
            .declare(Kind::Enum { module: 0, declaration: 1, arity: 0 }, &[], &mut 0)
            .expect("nominal Enum");
        b.set_members(id, &[Some(aggregate), None], &mut 0).expect("active payload");
        Some(id)
    } else {
        None
    };
    let world = b.seal(&mut 0).expect("actual independently checked closed source type world");
    let mut p = typed::continuation_test_support::push(
        &typed::continuation_test_support::Types::new(),
        None,
    );
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters = vec![raw::Parameter::Value(value(0, aggregate))];
    f.result_type = raw::ValueType::Stored(string);
    f.places =
        vec![raw::Place { id: 0, ty: aggregate, span: span(), kind: raw::PlaceKind::Parameter(0) }];
    f.blocks[0].instructions.clear();
    let base = if let Some(en) = en {
        f.places.push(raw::Place {
            id: 1,
            ty: en,
            span: span(),
            kind: raw::PlaceKind::Temporary(1),
        });
        f.places.push(raw::Place {
            id: 2,
            ty: aggregate,
            span: span(),
            kind: raw::PlaceKind::EnumPayload { base: 1, variant: 0 },
        });
        let mut make = instruction(raw::Operation::ClosedEnumConstruct {
            ty: en,
            ordinal: 0,
            payload: Some(0),
        });
        make.result = Some(value(1, en));
        f.blocks[0].instructions.push(make);
        2
    } else {
        0
    };
    let first = u32::try_from(f.places.len()).expect("small place");
    let order = if matches!(case, Case::OutOfOrder) { [1, 0] } else { [0, 1] };
    for ordinal in order {
        let id = u32::try_from(f.places.len()).expect("small place");
        let kind = if matches!(case, Case::Array) {
            raw::PlaceKind::FixedArrayConstant { base, index: ordinal }
        } else {
            raw::PlaceKind::StructField { base, ordinal }
        };
        f.places.push(raw::Place { id, ty: string, span: span(), kind });
    }
    let selected = first + u32::from(matches!(case, Case::OutOfOrder));
    let sibling = first + u32::from(!matches!(case, Case::OutOfOrder));
    let result = u32::from(en.is_some()) + 1;
    let temp = u32::try_from(f.places.len()).expect("small place");
    f.places.push(raw::Place {
        id: temp,
        ty: string,
        span: span(),
        kind: raw::PlaceKind::Temporary(result),
    });
    let mut take =
        instruction(raw::Operation::ReadPlace { place: selected, mode: raw::ReadMode::Move });
    take.result = Some(value(result, string));
    f.blocks[0].instructions.push(take);
    let actions = if reinitialize {
        f.blocks[0]
            .instructions
            .push(instruction(raw::Operation::InitializePlace { place: selected, value: result }));
        f.result_type = raw::ValueType::Unit;
        vec![raw::CleanupAction::DropPlace(0)]
    } else if drop_root {
        f.blocks[0].instructions.push(instruction(raw::Operation::DropPlace { place: 0 }));
        vec![]
    } else {
        vec![raw::CleanupAction::DropPlace(sibling)]
    };
    f.cleanup_plans = vec![raw::CleanupPlan { id: 0, span: span(), actions }];
    f.blocks[0].terminator = raw::Terminator::Return {
        value: if reinitialize { None } else { Some(result) },
        cleanup: 0,
    };
    (p, world)
}
