use super::super::BorrowIdentity;
use super::indexed_access::chained;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

#[test]
fn transient_indexed_edges_one_arm_begin_has_exact_nondominating_join_diagnostic() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut program = diamond(&fixture, raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    let at = function.span;
    let begin = function.blocks[0].instructions.remove(0);
    function.blocks[1].instructions.push(begin);
    fixture.rejects(program.clone(), "ZRYNA-I3011");
    let diagnostics = verify(
        program,
        &fixture.sources,
        fixture.sources.verify_file_id(0).expect("fixture source file"),
        fixture.linear.clone(),
        fixture.linux.clone(),
    )
    .expect_err("hostile authority must be rejected");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-I3011");
    assert_eq!(
        diagnostics[0].message(),
        "transient indexed identities or issuance order differ across a continuation"
    );
    assert_eq!(
        diagnostics[0].guidance(),
        "preserve the same live indexed authority on every incoming edge"
    );
    assert_eq!(diagnostics[0].primary_span(), Some(at));
    fixture.verify(diamond(&fixture, raw::BorrowAccess::Shared));
}

fn edge(target: u32) -> raw::Edge {
    raw::Edge { target: raw::BlockId(target), arguments: vec![] }
}

pub(super) fn diamond(fixture: &Fixture, access: raw::BorrowAccess) -> raw::Program {
    let mut program = chained(fixture, access);
    let function = &mut program.modules[0].functions[0];
    function.parameters.push(raw::ValueDefinition {
        id: raw::ValueId(5),
        ty: fixture.boolean,
        span: function.span,
    });
    let tail = function.blocks[0].instructions.split_off(1);
    let returns = function.blocks[0].terminators.clone();
    function.blocks[0].terminators[0].kind = raw::Terminator::Branch {
        condition: raw::ValueId(5),
        when_true: edge(1),
        when_false: edge(2),
    };
    for id in 1..=3 {
        function.blocks.push(raw::Block {
            id: raw::BlockId(id),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span: function.span,
                kind: raw::Terminator::Jump(edge(3)),
            }],
        });
    }
    function.blocks[3].instructions = tail;
    function.blocks[3].terminators = returns;
    program
}

#[test]
fn transient_indexed_edges_owned_commit_keeps_old_container_until_join() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::String);
        let mut program = diamond(&fixture, raw::BorrowAccess::Exclusive);
        let function = &mut program.modules[0].functions[0];
        function.cleanup_plans.pop();
        function.cleanup_plans[1].actions.remove(0);
        function.blocks[3].instructions = vec![
            raw::Instruction {
                result: None,
                span: function.span,
                kind: raw::InstructionKind::BorrowReplace {
                    borrow: raw::BorrowId(0),
                    value: raw::ValueId(4),
                },
            },
            end_borrow(0, function.span),
        ];
        let verified = fixture.verify(program.clone());
        let function = verified
            .modules()
            .next()
            .expect("verified fixture module")
            .functions()
            .next()
            .expect("verified fixture function");
        let block = function.blocks().nth(3).expect("joined continuation block");
        let commit = block.instructions().next().expect("continuation instruction");
        assert_eq!(
            commit
                .borrow_replacement()
                .expect("owned replacement")
                .old_value_drop()
                .referent()
                .index(),
            fixture.element.0
        );
        assert_eq!(commit.derived_drop_actions().count(), 0);
        assert_eq!(block.terminator().derived_drop_actions().count(), 2);
        let mut shared = program.clone();
        let raw::InstructionKind::BeginIndexedAccess { definition, .. } =
            &mut shared.modules[0].functions[0].blocks[0].instructions[0].kind
        else {
            panic!("begin")
        };
        definition.access = raw::BorrowAccess::Shared;
        fixture.rejects(shared, "ZRYNA-I3005");
        fixture.verify(program);
    }
}

#[test]
fn transient_indexed_edges_trap_ends_retained_authority_before_owner_cleanup() {
    let fixture = Fixture::new(Container::Vec, Element::Array);
    let mut program = diamond(&fixture, raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    let mut cleanup = function.cleanup_plans[0].clone();
    cleanup.id = raw::CleanupPlanId(3);
    function.cleanup_plans.push(cleanup);
    function.blocks[1].terminators[0].kind = raw::Terminator::Trap {
        identity: raw::TrapIdentity::BoundsV1,
        cleanup: raw::CleanupPlanId(3),
    };
    let verified = fixture.verify(program.clone());
    let function = verified
        .modules()
        .next()
        .expect("verified fixture module")
        .functions()
        .next()
        .expect("verified fixture function");
    let terminator = function.blocks().nth(1).expect("first outcome block").terminator();
    assert_eq!(
        terminator.failure_ended_borrows().map(BorrowIdentity::index).collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(terminator.continued_indexed_accesses().count(), 0);
    assert_eq!(terminator.derived_drop_actions().count(), 3);
    program.modules[0].functions[0].cleanup_plans[3].actions.pop();
    fixture.rejects(program, "ZRYNA-I3012");
}

#[test]
fn transient_indexed_edges_reject_live_backedge() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut program = diamond(&fixture, raw::BorrowAccess::Shared);
    program.modules[0].functions[0].blocks[1].terminators[0].kind = raw::Terminator::Jump(edge(1));
    fixture.rejects(program, "ZRYNA-I3011");
}

#[test]
fn transient_indexed_edges_reject_borrowed_owner_edge_transfer() {
    let fixture = Fixture::new(Container::Vec, Element::Array);
    let mut program = diamond(&fixture, raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    function.blocks[1].parameters.push(raw::ValueDefinition {
        id: raw::ValueId(6),
        ty: fixture.root,
        span: function.span,
    });
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.root,
        span: function.span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(6)),
    });
    let raw::Terminator::Branch { when_true, .. } = &mut function.blocks[0].terminators[0].kind
    else {
        panic!("branch")
    };
    when_true.arguments = vec![raw::ValueId(1)];
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(3),
        span: function.span,
        actions: [2, 3, 0].map(|id| raw::DropAction::DropPlace(raw::PlaceId(id))).into(),
    });
    function.blocks[1].terminators[0].kind = raw::Terminator::Trap {
        identity: raw::TrapIdentity::BoundsV1,
        cleanup: raw::CleanupPlanId(3),
    };
    fixture.verify(program.clone());
    let function = &mut program.modules[0].functions[0];
    let raw::Terminator::Branch { when_true, .. } = &mut function.blocks[0].terminators[0].kind
    else {
        panic!("branch")
    };
    when_true.arguments = vec![raw::ValueId(0)];
    function.cleanup_plans[3].actions =
        [2, 1, 3].map(|id| raw::DropAction::DropPlace(raw::PlaceId(id))).into();
    fixture.rejects(program.clone(), "ZRYNA-I3010");
    let errors = verify(
        program,
        &fixture.sources,
        fixture.sources.verify_file_id(0).expect("fixture source file"),
        fixture.linear.clone(),
        fixture.linux.clone(),
    )
    .expect_err("hostile authority must be rejected");
    assert!(
        errors
            .iter()
            .any(|error| error.message() == "owned edge argument overlaps an active borrow")
    );
}

#[test]
fn transient_indexed_edges_preserve_exact_identity_and_failure_cleanup() {
    for container in [Container::Array, Container::Vec] {
        for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
            let fixture = Fixture::new(container, Element::Array);
            let raw = diamond(&fixture, access);
            let verified = fixture.verify(raw.clone());
            let replay = fixture.verify(raw);
            assert_eq!(format!("{verified:?}"), format!("{replay:?}"));
            let function = verified
                .modules()
                .next()
                .expect("verified fixture module")
                .functions()
                .next()
                .expect("verified fixture function");
            for block in function.blocks().take(3) {
                let retained = block.terminator().continued_indexed_accesses().collect::<Vec<_>>();
                assert_eq!(retained.len(), 1);
                assert_eq!(retained[0].borrow().index(), 0);
                assert_eq!(retained[0].region().index(), 0);
                assert_eq!(retained[0].access(), access.into());
            }
            let block = function.blocks().nth(3).expect("joined continuation block");
            let project = block.instructions().next().expect("continuation instruction");
            assert_eq!(
                project.failure_ended_borrows().map(BorrowIdentity::index).collect::<Vec<_>>(),
                [0]
            );
            assert_eq!(project.derived_drop_actions().count(), 3);
            assert_eq!(block.terminator().continued_indexed_accesses().count(), 0);
        }
    }
}

#[test]
fn transient_indexed_edges_reject_unequal_join_and_live_return() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let valid = diamond(&fixture, raw::BorrowAccess::Shared);
    let mut unequal = valid.clone();
    let function = &mut unequal.modules[0].functions[0];
    function.blocks[1].instructions.push(end_borrow(0, function.span));
    fixture.rejects(unequal, "ZRYNA-I3011");
    let mut live = valid.clone();
    live.modules[0].functions[0].blocks[3].instructions.pop();
    fixture.rejects(live, "ZRYNA-I3011");
    fixture.verify(valid);
}

#[test]
fn transient_indexed_edges_reject_lexical_begin_and_bind() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    for bind in [false, true] {
        let mut program = diamond(&fixture, raw::BorrowAccess::Shared);
        let function = &mut program.modules[0].functions[0];
        // No child projection: a lexical authority itself must be rejected at the edge.
        function.blocks[3].instructions.remove(0);
        function.cleanup_plans.pop();
        if bind {
            function.blocks[0].instructions.push(raw::Instruction {
                result: None,
                span: function.span,
                kind: raw::InstructionKind::BindIndexedBorrow {
                    parent: raw::BorrowId(0),
                    borrow: raw::BorrowId(1),
                },
            });
        } else {
            let begin = &mut function.blocks[0].instructions[0];
            let raw::InstructionKind::BeginIndexedAccess { definition, index, cleanup } =
                begin.kind.clone()
            else {
                panic!("begin")
            };
            begin.kind = raw::InstructionKind::BeginIndexedBorrow { definition, index, cleanup };
            function.blocks[3].instructions[0] = end_borrow(0, function.span);
        }
        fixture.rejects(program, "ZRYNA-I3011");
    }
}
