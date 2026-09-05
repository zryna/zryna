use super::generic_clone_fixture::Fixture;
use super::*;
use zryna_layout::TypeCategory;

fn projected(fixture: &Fixture) -> raw::Program {
    let mut raw = fixture.seed();
    let element = raw::TypeId(
        fixture
            .linear
            .types()
            .find(|ty| ty.category() == TypeCategory::FixedArray)
            .expect("array projection")
            .id()
            .index(),
    );
    let function = &mut raw.modules[0].functions[0];
    function.result = element;
    function.places[2].ty = element;
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: element,
        span: function.span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
    });
    function.blocks[0].instructions[0].result.as_mut().expect("clone result").ty = element;
    let raw::InstructionKind::GenericClonePlace { place, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        panic!("clone");
    };
    *place = raw::PlaceId(3);
    raw
}

#[test]
fn generic_clone_static_projection_has_exact_referent_and_retains_enclosing_owner() {
    let fixture = Fixture::new(TypeCategory::Struct);
    let raw = projected(&fixture);
    for _ in 0..2 {
        let verified = fixture.verify(raw.clone());
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instruction = block.instructions().next().expect("projected clone");
        let clone = instruction.generic_clone().expect("clone authority");
        assert!(
            matches!(clone.source(), super::super::VerifiedGenericCloneSource::Place(place) if place.index() == 3)
        );
        assert_ne!(clone.ty().index(), fixture.root.0);
        assert_eq!(clone.frontier().types().count(), 6);
        let pending = instruction.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(pending.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(), [1, 0]);
        assert!(pending.iter().all(|drop| drop.moved_projections().count() == 0));
        assert_eq!(pending, block.terminator().derived_drop_actions().collect::<Vec<_>>());
        let prefix = instruction.generic_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(prefix[0].root(), clone.destination());
        assert_eq!(&prefix[1..], pending.as_slice());
    }
}

#[test]
fn generic_clone_static_projection_rejects_wrong_referent_and_enclosing_exclusive_borrow() {
    let fixture = Fixture::new(TypeCategory::Struct);
    let seed = projected(&fixture);
    fixture.verify(seed.clone());
    let mut wrong = seed.clone();
    let function = &mut wrong.modules[0].functions[0];
    function.result = fixture.root;
    function.places[2].ty = fixture.root;
    function.blocks[0].instructions[0].result.as_mut().expect("clone result").ty = fixture.root;
    fixture.rejects_case(
        wrong,
        "ZRYNA-I3005",
        "projection cannot masquerade as its enclosing owner",
    );
    let mut blocked = seed;
    let function = &mut blocked.modules[0].functions[0];
    let span = function.span;
    function.blocks[0]
        .instructions
        .insert(0, begin_borrow(0, 0, raw::BorrowAccess::Exclusive, span));
    function.blocks[0].instructions.push(end_borrow(0, span));
    fixture.rejects_case(
        blocked,
        "ZRYNA-I3010",
        "enclosing exclusive borrow blocks projected clone",
    );
}
