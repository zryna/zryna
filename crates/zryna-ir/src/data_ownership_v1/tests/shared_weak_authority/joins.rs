use super::*;

#[test]
fn weak_upgrade_joins_require_exact_retained_owners_after_success_binding_drop() {
    let fixture = Fixture::new(Payload::String);
    let mut raw = fixture.program();
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.blocks[1].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(5) },
    });
    for (block, argument) in [(1, 7), (2, 8)] {
        function.blocks[block].terminators[0].kind = raw::Terminator::Jump(raw::Edge {
            target: raw::BlockId(3),
            arguments: vec![raw::ValueId(argument)],
        });
    }
    function.cleanup_plans.truncate(6);
    function.cleanup_plans[5] = fixtures::cleanup(5, vec![4, 3, 2, 1], span);
    function.blocks.push(raw::Block {
        id: raw::BlockId(3),
        parameters: vec![raw::ValueDefinition { id: raw::ValueId(9), ty: raw::TypeId(1), span }],
        instructions: vec![],
        terminators: vec![fixtures::returning(9, 5, span)],
    });
    for _ in 0..2 {
        fixture.verify(raw.clone()).expect("success binding released before exact join");
    }
    let mut forged = raw.clone();
    forged.modules[0].functions[0].blocks[2].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(1) },
    });
    let first = diagnostic_trace(fixture.verify(forged.clone()).expect_err("one edge lost owner"));
    assert!(first.iter().any(|entry| entry.0 == "ZRYNA-I3010" && entry.1 ==
        "ownership, initialization, or active-enum state differs across a CFG join or backedge"));
    assert_eq!(first, diagnostic_trace(fixture.verify(forged).expect_err("same rejection")));
    fixture.verify(raw).expect("valid join recovery");
}
