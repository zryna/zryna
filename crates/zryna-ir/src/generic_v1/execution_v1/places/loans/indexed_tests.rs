//! Untrusted metadata inputs exercise the real owning kernel; no source/whole execution proof.
use crate::generic_v1::execution_v1::{
    places, raw,
    typed::continuation_test_support::{Types, push},
};
fn fixture() -> (Types, raw::Program, [raw::ValueType; 3]) {
    let w = Types::new();
    let mut p = push(&w, None);
    let f = &mut p.functions[0];
    f.parameters.push(raw::Parameter::Value(raw::ValueDefinition {
        id: 2,
        ty: raw::ValueType::Stored(w.integer),
    }));
    f.loans = vec![raw::Loan {
        id: 0,
        referent: w.string,
        access: raw::Access::Shared,
        origin: raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(0), index: 2 },
        region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
    }];
    let values = [
        raw::ValueType::Stored(w.vector),
        raw::ValueType::Stored(w.string),
        raw::ValueType::Stored(w.integer),
    ];
    (w, p, values)
}
#[test]
fn indexed_metadata_keeps_root_and_exact_element_without_fabricating_static_place() {
    let (w, p, values) = fixture();
    let table = places::check(&p.functions[0], &values, &w.world, &mut 0)
        .expect("metadata-only owning kernel admits exact indexed element");
    assert_eq!(table.loans[0].root, Some(0));
    assert_eq!(table.loans[0].local_place, None, "dynamic element is never a static Place");
    assert!(!table.loans[0].formal);
}
#[test]
fn indexed_metadata_rejects_wrong_element_and_noncontainer_base() {
    for wrong_element in [false, true] {
        let (w, mut p, values) = fixture();
        if wrong_element {
            p.functions[0].loans[0].referent = w.integer;
        } else {
            p.functions[0].loans[0].origin =
                raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(1), index: 2 };
        }
        assert!(places::check(&p.functions[0], &values, &w.world, &mut 0).is_err());
    }
}
#[test]
fn indexed_metadata_parent_permissions_regions_and_order_are_independent() {
    let (w, mut p, values) = fixture();
    let f = &mut p.functions[0];
    let mut parent = f.loans[0].clone();
    parent.origin = raw::LoanOrigin::Place(0);
    parent.referent = w.vector;
    let mut child = f.loans[0].clone();
    child.id = 1;
    child.origin = raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Loan(0), index: 2 };
    f.loans = vec![parent, child];
    places::check(f, &values, &w.world, &mut 0)
        .expect("shared indexed child retains exact checked parent");
    f.loans[1].access = raw::Access::Exclusive;
    assert!(
        places::check(f, &values, &w.world, &mut 0).is_err(),
        "shared parent never grants mutation"
    );
    f.loans[1].access = raw::Access::Shared;
    f.loans[0].region.span.start += 1;
    assert!(
        places::check(f, &values, &w.world, &mut 0).is_err(),
        "child cannot outlive exact parent"
    );
    f.loans[0].region.span = f.original_span;
    f.loans[0].origin = raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Loan(1), index: 2 };
    assert!(
        places::check(f, &values, &w.world, &mut 0).is_err(),
        "forward parent/cycle never grants authority"
    );
}

#[test]
fn even_unused_indexed_metadata_requires_a_real_i32_value() {
    for index in [u32::MAX, 0, 1] {
        let (w, mut p, values) = fixture();
        p.functions[0].loans[0].origin =
            raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(0), index };
        assert!(
            places::check(&p.functions[0], &values, &w.world, &mut 0).is_err(),
            "opaque indexed metadata cannot expose foreign, container or String values as i32"
        );
    }
    let (w, p, mut values) = fixture();
    values[2] = raw::ValueType::Unit;
    assert!(places::check(&p.functions[0], &values, &w.world, &mut 0).is_err());
}
