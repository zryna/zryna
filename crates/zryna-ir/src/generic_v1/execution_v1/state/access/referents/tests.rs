//! Rooted projection/reborrow identity and external ownership stay distinct.
use super::{Context, referent_place};
use crate::generic_v1::execution_v1::{
    places, raw,
    type_world::{Builder, Kind, TypeWorld},
    typed,
};
use typed::continuation_test_support::{Types, push, span, value};
fn fixture() -> (TypeWorld, raw::Program, u32) {
    let mut b = Builder::new();
    let integer = b.declare(Kind::I32, &[], &mut 0).expect("i32");
    let array = b.declare(Kind::FixedArray { length: 2 }, &[integer], &mut 0).expect("array");
    let world = b.seal(&mut 0).expect("actual finite type kernel");
    let mut p = push(&Types::new(), None);
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters = vec![
        raw::Parameter::Value(value(0, array)),
        raw::Parameter::Loan(raw::LoanDefinition {
            id: 4,
            referent: array,
            access: raw::Access::Shared,
        }),
    ];
    f.places =
        vec![raw::Place { id: 0, ty: array, span: span(), kind: raw::PlaceKind::Parameter(0) }];
    for (id, index) in [(1, 0), (2, 1), (3, 1)] {
        f.places.push(raw::Place {
            id,
            ty: integer,
            span: span(),
            kind: raw::PlaceKind::FixedArrayConstant { base: 0, index },
        });
    }
    let origins = [
        raw::LoanOrigin::Place(0),
        raw::LoanOrigin::StaticProjection {
            parent: 0,
            projection: raw::StaticProjection::FixedArrayConstant(1),
        },
        raw::LoanOrigin::InternalReborrow(1),
        raw::LoanOrigin::Place(3),
        raw::LoanOrigin::Formal(1),
        raw::LoanOrigin::StaticProjection {
            parent: 4,
            projection: raw::StaticProjection::FixedArrayConstant(1),
        },
    ];
    f.loans = origins
        .into_iter()
        .enumerate()
        .map(|(index, origin)| {
            let id = u32::try_from(index).expect("small actual fixture");
            raw::Loan {
                id,
                referent: if matches!(id, 0 | 4) { array } else { integer },
                access: raw::Access::Shared,
                origin,
                region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
            }
        })
        .collect();
    f.blocks[0].instructions.clear();
    f.blocks[0].terminator = raw::Terminator::Return { value: None, cleanup: 0 };
    f.cleanup_plans = vec![raw::CleanupPlan { id: 0, span: span(), actions: vec![] }];
    (world, p, array)
}
#[test]
fn canonical_rooted_reborrow_and_projection_do_not_manufacture_external_places() {
    let (world, p, _) = fixture();
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    let c = Context { program: &p, typed: &typed, function: 0 };
    for (id, expected) in
        [(0, Some(0)), (1, Some(2)), (2, Some(2)), (3, Some(2)), (4, None), (5, None)]
    {
        assert_eq!(referent_place(&c, id).expect("exact checked retained referent"), expected);
    }
}
#[test]
fn rooted_projection_requires_a_real_checked_backing_place() {
    let (world, mut p, array) = fixture();
    p.functions[0].places.truncate(1);
    p.functions[0].loans.truncate(3);
    p.functions[0].parameters.truncate(1);
    assert!(
        places::check(&p.functions[0], &[raw::ValueType::Stored(array)], &world, &mut 0).is_err(),
        "rooted projection cannot silently become an external/formal referent"
    );
}
