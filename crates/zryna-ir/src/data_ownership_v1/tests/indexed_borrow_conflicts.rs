use super::indexed_borrow_fixture::{Container, Element, Fixture, indexed};
use super::*;

fn pair(
    fixture: &Fixture,
    access: [raw::BorrowAccess; 2],
    second: u32,
    static_second: bool,
) -> raw::Program {
    let mut raw = fixture.seed(access[0]);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let actions = function.cleanup_plans[0].actions.clone();
    function.cleanup_plans = (0..3)
        .map(|id| raw::CleanupPlan { id: raw::CleanupPlanId(id), span, actions: actions.clone() })
        .collect();
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(2) };
    let other = if static_second {
        function.cleanup_plans.remove(1);
        function.cleanup_plans[1].id = raw::CleanupPlanId(1);
        if let raw::Terminator::Return { cleanup, .. } = &mut function.blocks[0].terminators[0].kind
        {
            *cleanup = raw::CleanupPlanId(1);
        }
        begin_borrow(1, second, access[1], span)
    } else {
        indexed(1, second, 3, 1, access[1], span)
    };
    function.blocks[0].instructions =
        vec![indexed(0, 0, 2, 0, access[0], span), other, end_borrow(1, span), end_borrow(0, span)];
    raw
}

#[test]
fn indexed_borrow_overlap_uses_complete_container_for_all_access_pairs() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::String);
        for first in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
            for second in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
                let accesses = [first, second];
                fixture.verify(pair(&fixture, accesses, 1, false));
                fixture.verify(pair(&fixture, accesses, 1, true));
                for static_second in [false, true] {
                    let same = pair(&fixture, accesses, 0, static_second);
                    if accesses == [raw::BorrowAccess::Shared; 2] {
                        fixture.verify(same);
                    } else {
                        fixture.rejects(same, "ZRYNA-I3011");
                    }
                }
            }
        }
    }
}

#[test]
fn indexed_borrow_bounds_failure_ends_only_prior_live_authorities() {
    let fixture = Fixture::new(Container::Array, Element::String);
    let mut raw = pair(&fixture, [raw::BorrowAccess::Shared; 2], 0, false);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.blocks[0].instructions.insert(2, begin_borrow(2, 1, raw::BorrowAccess::Shared, span));
    function.blocks[0].instructions.insert(3, end_borrow(1, span));
    function.blocks[0].instructions.insert(4, indexed(3, 0, 3, 2, raw::BorrowAccess::Shared, span));
    function.blocks[0].instructions[5] = end_borrow(3, span);
    function.blocks[0].instructions.insert(6, end_borrow(2, span));
    let actions = function.cleanup_plans[0].actions.clone();
    function.cleanup_plans.push(raw::CleanupPlan { id: raw::CleanupPlanId(3), span, actions });
    if let raw::Terminator::Return { cleanup, .. } = &mut function.blocks[0].terminators[0].kind {
        *cleanup = raw::CleanupPlanId(3);
    }
    let verified = fixture.verify(raw);
    let instructions = verified
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .collect::<Vec<_>>();
    for (site, expected) in [(0, vec![]), (1, vec![0]), (4, vec![2, 0])] {
        assert_eq!(
            instructions[site]
                .failure_ended_borrows()
                .map(super::super::BorrowIdentity::index)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn indexed_array_borrow_conflicts_with_static_elements_in_both_directions() {
    let fixture = Fixture::new(Container::Array, Element::String);
    for accesses in [
        [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive],
        [raw::BorrowAccess::Exclusive, raw::BorrowAccess::Shared],
        [raw::BorrowAccess::Exclusive; 2],
    ] {
        for ordinal in [0, 1] {
            let mut raw = pair(&fixture, accesses, 3, true);
            let function = &mut raw.modules[0].functions[0];
            function.places.push(raw::Place {
                id: raw::PlaceId(3),
                ty: fixture.element,
                span: function.span,
                kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: ordinal },
            });
            fixture.rejects(raw.clone(), "ZRYNA-I3011");
            let function = &mut raw.modules[0].functions[0];
            function.blocks[0].instructions.swap(0, 1);
            if let raw::InstructionKind::BeginBorrow(definition) =
                &mut function.blocks[0].instructions[0].kind
            {
                definition.id = raw::BorrowId(0);
            }
            if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
                &mut function.blocks[0].instructions[1].kind
            {
                definition.id = raw::BorrowId(1);
            }
            fixture.rejects(raw, "ZRYNA-I3011");
        }
    }
}

#[test]
fn indexed_borrow_does_not_make_static_array_siblings_overlap() {
    let fixture = Fixture::new(Container::Array, Element::String);
    for first in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
        for second in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
            let mut raw = fixture.seed(first);
            let function = &mut raw.modules[0].functions[0];
            let span = function.span;
            for index in 0..2 {
                function.places.push(raw::Place {
                    id: raw::PlaceId(3 + index),
                    ty: fixture.element,
                    span,
                    kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index },
                });
            }
            function.blocks[0].instructions = vec![
                begin_borrow(0, 3, first, span),
                begin_borrow(1, 4, second, span),
                end_borrow(1, span),
                end_borrow(0, span),
            ];
            function.cleanup_plans.remove(0);
            function.cleanup_plans[0].id = raw::CleanupPlanId(0);
            if let raw::Terminator::Return { cleanup, .. } =
                &mut function.blocks[0].terminators[0].kind
            {
                *cleanup = raw::CleanupPlanId(0);
            }
            fixture.verify(raw);
        }
    }
}
