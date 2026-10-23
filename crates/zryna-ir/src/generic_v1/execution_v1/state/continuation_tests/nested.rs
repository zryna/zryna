//! Real recursive nominal Vec field: RHS can own the very target being appended to.
use super::{Context, State, instructions, plan, raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use typed::continuation_test_support::{instruction, span, value};
fn fixture() -> (raw::Program, TypeWorld) {
    let mut b = Builder::new();
    let string = b.declare(Kind::String, &[], &mut 0).expect("String");
    let node =
        b.declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0).expect("Node");
    let vector = b.declare(Kind::Vec, &[node], &mut 0).expect("Vec<Node>");
    b.set_members(node, &[Some(vector), Some(string)], &mut 0)
        .expect("same-key indirect recursion");
    let world = b.seal(&mut 0).expect("finite original world");
    let mut p = typed::continuation_test_support::push(
        &typed::continuation_test_support::Types::new(),
        None,
    );
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters =
        vec![raw::Parameter::Value(value(0, node)), raw::Parameter::Value(value(1, node))];
    f.places = vec![
        raw::Place { id: 0, ty: node, span: span(), kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: 1,
            ty: vector,
            span: span(),
            kind: raw::PlaceKind::StructField { base: 0, ordinal: 0 },
        },
        raw::Place { id: 2, ty: node, span: span(), kind: raw::PlaceKind::Parameter(1) },
        raw::Place {
            id: 3,
            ty: string,
            span: span(),
            kind: raw::PlaceKind::StructField { base: 0, ordinal: 1 },
        },
    ];
    f.blocks[0].instructions[0] = instruction(raw::Operation::VecPush {
        vector: raw::PlaceOrLoan::Place(1),
        value: 1,
        failure: 0,
    });
    f.cleanup_plans[0].actions =
        vec![raw::CleanupAction::DropPlace(2), raw::CleanupAction::DropPlace(0)];
    (p, world)
}
#[test]
fn prepared_rhs_owning_target_is_rejected_after_offside_consume_without_moving_original() {
    let (p, world) = fixture();
    let t = typed::container_test_support::phase_inputs(&p, &world);
    let mut plan = plan::derive(&p, &t, &mut 0).expect("actual distinct Node RHS append");
    let c = Context { program: &p, typed: &t, function: 0 };
    let mut state = State::new(&c).expect("two complete Node owners");
    let before = state.copy().expect("original prepared snapshot");
    let hostile = instruction(raw::Operation::VecPush {
        vector: raw::PlaceOrLoan::Place(1),
        value: 0,
        failure: 0,
    });
    assert!(instructions::execute(&c, &mut state, 0, 0, &hostile, &mut plan, &mut 0).is_err());
    assert_eq!(state, before, "target-containing RHS is never consumed in original state");
}
#[test]
fn disjoint_static_field_loan_allows_append_but_exact_target_alias_rejects() {
    let (mut p, world) = fixture();
    let f = &mut p.functions[0];
    f.loans.push(raw::Loan {
        id: 0,
        referent: f.places[3].ty,
        access: raw::Access::Shared,
        region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Place(3),
    });
    f.blocks[0]
        .instructions
        .insert(0, instruction(raw::Operation::BeginLoan { loan: 0, failure: None }));
    f.blocks[0].instructions.push(instruction(raw::Operation::EndLoan { loan: 0 }));
    f.cleanup_plans[0].actions.insert(0, raw::CleanupAction::EndLoan(0));
    let t = typed::container_test_support::phase_inputs(&p, &world);
    plan::derive(&p, &t, &mut 0).expect("sibling String loan does not alias Node.vec");
    p.functions[0].loans[0].origin = raw::LoanOrigin::Place(1);
    p.functions[0].loans[0].referent = p.functions[0].places[1].ty;
    let t = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &t, &mut 0).is_err(), "exact target loan blocks direct append");
}
#[test]
fn actual_append_preparation_budget_rejects_before_commit_at_first_extra_and_overflow() {
    let (p, world) = fixture();
    let t = typed::container_test_support::phase_inputs(&p, &world);
    let mut plan = plan::derive(&p, &t, &mut 0).expect("pristine actual replay");
    let c = Context { program: &p, typed: &t, function: 0 };
    let run = |spent: &mut usize, plan: &mut plan::Plan| {
        let mut state = State::new(&c).expect("actual prepared state");
        let before = state.copy().expect("pre-instruction snapshot");
        let result = instructions::execute(
            &c,
            &mut state,
            0,
            0,
            &p.functions[0].blocks[0].instructions[0],
            plan,
            spent,
        );
        if result.is_err() {
            assert_eq!(state, before);
        }
        result
    };
    let mut cost = 0;
    run(&mut cost, &mut plan).expect("measure actual preparation");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    run(&mut exact, &mut plan).expect("exact remaining preparation credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(run(&mut spent, &mut plan).is_err());
        assert_eq!(spent, before);
    }
    let mut retry = 0;
    run(&mut retry, &mut plan).expect("pristine retry");
    assert_eq!(retry, cost);
}
