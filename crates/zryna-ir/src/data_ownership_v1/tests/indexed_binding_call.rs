use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::indexed_borrow_owned::call_replacement;
use super::*;

#[test]
fn indexed_binding_call_uses_lexical_child_and_preserves_exact_trap_cleanup() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::Array);
        let mut program = call_replacement(&fixture);
        let function = &mut program.modules[0].functions[0];
        let begin = &mut function.blocks[0].instructions[0];
        let raw::InstructionKind::BeginIndexedBorrow { definition, index, cleanup } =
            begin.kind.clone()
        else {
            panic!("begin")
        };
        begin.kind = raw::InstructionKind::BeginIndexedAccess { definition, index, cleanup };
        function.blocks[0].instructions.insert(
            1,
            raw::Instruction {
                result: None,
                span: function.span,
                kind: raw::InstructionKind::BindIndexedBorrow {
                    parent: raw::BorrowId(0),
                    borrow: raw::BorrowId(1),
                },
            },
        );
        let raw::InstructionKind::DirectCall { arguments, .. } =
            &mut function.blocks[0].instructions[2].kind
        else {
            panic!("call")
        };
        *arguments.last_mut().expect("borrow suffix") = raw::CallArgument::Borrow(raw::BorrowId(1));
        function.blocks[0].instructions[3].kind =
            raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(1) };
        let verified = fixture.verify(program.clone());
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let binding =
            block.instructions().nth(1).expect("bind").indexed_binding().expect("binding");
        let call = block.instructions().nth(2).expect("call");
        assert_eq!(call.failure_ended_borrows().collect::<Vec<_>>(), [binding.borrow()]);
        assert_eq!(
            call.derived_drop_actions().map(|drop| drop.root().index()).collect::<Vec<_>>(),
            [1, 0]
        );
        let raw::InstructionKind::DirectCall { arguments, .. } =
            &mut program.modules[0].functions[0].blocks[0].instructions[2].kind
        else {
            panic!("call")
        };
        *arguments.last_mut().expect("borrow suffix") = raw::CallArgument::Borrow(raw::BorrowId(0));
        fixture.rejects(program, "ZRYNA-I3011");
    }
}
