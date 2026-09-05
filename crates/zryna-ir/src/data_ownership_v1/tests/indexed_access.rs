use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

pub(super) fn chained(fixture: &Fixture, access: raw::BorrowAccess) -> raw::Program {
    let mut program = fixture.seed(access);
    let function = &mut program.modules[0].functions[0];
    let begin = &mut function.blocks[0].instructions[0];
    let raw::InstructionKind::BeginIndexedBorrow { definition, index, cleanup } =
        begin.kind.clone()
    else {
        panic!("begin")
    };
    begin.kind = raw::InstructionKind::BeginIndexedAccess { definition, index, cleanup };
    let mut cleanup = function.cleanup_plans[0].clone();
    cleanup.id = raw::CleanupPlanId(2);
    function.cleanup_plans.push(cleanup);
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::ProjectIndexedBorrow {
                parent: raw::BorrowId(0),
                borrow: raw::BorrowId(1),
                index: raw::ValueId(3),
                cleanup: raw::CleanupPlanId(2),
            },
        },
    );
    function.blocks[0].instructions[2].kind =
        raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(1) };
    program
}

#[test]
fn indexed_access_child_retains_region_access_and_failure_parent() {
    for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
        let fixture = Fixture::new(Container::Array, Element::Array);
        let verified = fixture.verify(chained(&fixture, access));
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let child = block.instructions().nth(1).expect("child");
        let view = child.indexed_projection().expect("sealed child");
        assert_eq!(view.parent().index(), 0);
        assert_eq!(view.borrow().index(), 1);
        assert_eq!(view.container().index(), 0);
        assert_eq!(view.index().index(), 3);
        assert_eq!(view.array_length(), Some(2));
        assert_eq!(view.access(), access.into());
        assert_eq!(child.failure_ended_borrows().map(|id| id.index()).collect::<Vec<_>>(), [0]);
        assert_eq!(child.derived_drop_actions().count(), 3);
    }
}

#[test]
fn indexed_access_rejects_lexical_parent_and_retired_parent_reads() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut lexical = chained(&fixture, raw::BorrowAccess::Shared);
    let begin = &mut lexical.modules[0].functions[0].blocks[0].instructions[0];
    let raw::InstructionKind::BeginIndexedAccess { definition, index, cleanup } =
        begin.kind.clone()
    else {
        panic!("begin")
    };
    begin.kind = raw::InstructionKind::BeginIndexedBorrow { definition, index, cleanup };
    fixture.rejects(lexical, "ZRYNA-I3005");
    let mut retired = chained(&fixture, raw::BorrowAccess::Shared);
    retired.modules[0].functions[0].blocks[0].instructions[2].kind =
        raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) };
    fixture.rejects(retired, "ZRYNA-I3011");
}

#[test]
fn indexed_access_rejects_non_array_parent_and_wrong_index_type() {
    let fixture = Fixture::new(Container::Array, Element::String);
    fixture.rejects(chained(&fixture, raw::BorrowAccess::Shared), "ZRYNA-I3005");
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut wrong = chained(&fixture, raw::BorrowAccess::Shared);
    if let raw::InstructionKind::ProjectIndexedBorrow { index, .. } =
        &mut wrong.modules[0].functions[0].blocks[0].instructions[1].kind
    {
        *index = raw::ValueId(0);
    }
    fixture.rejects(wrong, "ZRYNA-I3005");
}

#[test]
fn indexed_access_rejects_inactive_parent_repeated_identity_and_reused_cleanup() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut inactive = chained(&fixture, raw::BorrowAccess::Shared);
    let function = &mut inactive.modules[0].functions[0];
    function.blocks[0].instructions.insert(1, end_borrow(0, function.span));
    fixture.rejects(inactive, "ZRYNA-I3011");
    let mut repeated = chained(&fixture, raw::BorrowAccess::Shared);
    if let raw::InstructionKind::ProjectIndexedBorrow { borrow, .. } =
        &mut repeated.modules[0].functions[0].blocks[0].instructions[1].kind
    {
        *borrow = raw::BorrowId(0);
    }
    fixture.rejects(repeated, "ZRYNA-I3011");
    let mut reused = chained(&fixture, raw::BorrowAccess::Shared);
    if let raw::InstructionKind::ProjectIndexedBorrow { cleanup, .. } =
        &mut reused.modules[0].functions[0].blocks[0].instructions[1].kind
    {
        *cleanup = raw::CleanupPlanId(0);
    }
    fixture.rejects(reused, "ZRYNA-I3012");
    fixture.verify(chained(&fixture, raw::BorrowAccess::Shared));
}

#[test]
fn indexed_access_cannot_consume_formal_alias() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    function.borrow_parameters = vec![raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: fixture.element,
        access: raw::BorrowAccess::Shared,
        span: function.span,
    }];
    function.blocks[0].instructions[0].kind = raw::InstructionKind::ProjectIndexedBorrow {
        parent: raw::BorrowId(0),
        borrow: raw::BorrowId(1),
        index: raw::ValueId(3),
        cleanup: raw::CleanupPlanId(0),
    };
    function.blocks[0].instructions[1].kind =
        raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(1) };
    fixture.rejects(program.clone(), "ZRYNA-I3011");
    let errors = verify(
        program,
        &fixture.sources,
        fixture.sources.verify_file_id(0).expect("entry"),
        fixture.linear.clone(),
        fixture.linux.clone(),
    )
    .expect_err("formal alias cannot parent a projection");
    assert!(
        errors
            .iter()
            .any(|error| error.message()
                == "borrow parameter is not used by an access or direct call")
    );
}
