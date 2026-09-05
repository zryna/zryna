use super::explicit_indexed_fixture::{Container, call_fixture};
use super::generic_vec_fixture::Element;
use super::*;

#[test]
fn explicit_indexed_calls_retain_caller_container_and_transfer_only_value_arguments() {
    for container in [Container::Array(2), Container::Vec] {
        for element in [
            Element::I32,
            Element::String,
            Element::Struct,
            Element::Enum,
            Element::Array,
            Element::Vec,
        ] {
            for exclusive in [false, true] {
                let (source, raw) = call_fixture(container, &element, exclusive);
                let body = &raw.files[0].functions[0].body;
                let rhs_argument = body
                    .expressions
                    .iter()
                    .find_map(|expression| {
                        if let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } =
                            &expression.kind
                        {
                            Some(body.expressions[arguments[1] as usize].span)
                        } else {
                            None
                        }
                    })
                    .expect("source second argument is the RHS value");
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).unwrap_or_else(|errors| {
                    panic!("authenticated borrowed call syntax: {errors:?}\n{source}")
                });
                for _ in 0..2 {
                    let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                        panic!("{container:?} {element:?} {exclusive}: {errors:?}\n{source}")
                    });
                    let caller = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("caller");
                    let block = caller.blocks().next().expect("block");
                    let instructions = block.instructions().collect::<Vec<_>>();
                    let begin = instructions
                        .iter()
                        .find_map(|i| i.indexed_borrow())
                        .expect("caller indexed alias");
                    let position = instructions
                        .iter()
                        .position(|i| i.kind() == VerifiedInstructionKind::DirectCall)
                        .expect("direct call");
                    let call = instructions[position];
                    let arguments = call.call_arguments().collect::<Vec<_>>();
                    assert_eq!(arguments.len(), 2);
                    assert_eq!(arguments[1], VerifiedCallArgument::Borrow(begin.borrow()));
                    let VerifiedCallArgument::Value(rhs) = arguments[0] else {
                        panic!("ordinary exact RHS value");
                    };
                    assert!(
                        instructions[..position]
                            .iter()
                            .any(|i| i.result() == Some(rhs)
                                && i.span() == span(&sources, rhs_argument)),
                        "canonical value argument retains its source second-argument preparation"
                    );
                    assert_eq!(call.failure_ended_borrows().collect::<Vec<_>>(), [begin.borrow()]);
                    if matches!(container, Container::Vec) || !matches!(element, Element::I32) {
                        assert!(
                            call.derived_drop_actions().any(|a| a.root() == begin.container()
                                && a.moved_projections().len() == 0)
                        );
                    } else {
                        assert!(
                            !call.derived_drop_actions().any(|a| a.root() == begin.container()),
                            "Copy arrays have no pending drop obligation"
                        );
                    }
                    if !matches!(element, Element::I32) {
                        let owner = caller
                            .places()
                            .find(|p| p.kind() == VerifiedPlaceKind::Temporary(rhs))
                            .expect("prepared argument owner");
                        assert!(
                            !call.derived_drop_actions().any(|a| a.root() == owner.id()),
                            "owned argument transferred before CallTrap"
                        );
                    }
                    assert!(
                        instructions[position + 1..]
                            .iter()
                            .any(|i| i.kind() == VerifiedInstructionKind::EndBorrow
                                && i.borrow() == Some(begin.borrow()))
                    );
                    assert!(
                        !block
                            .terminator()
                            .derived_drop_actions()
                            .any(|a| a.root() == begin.container())
                    );
                }
            }
        }
    }
}

#[test]
fn explicit_indexed_calls_owned_formals_clone_and_replace_the_exact_referent() {
    for element in [
        Element::I32,
        Element::String,
        Element::Struct,
        Element::Enum,
        Element::Array,
        Element::Vec,
    ] {
        for exclusive in [false, true] {
            let (source, raw) = call_fixture(Container::Vec, &element, exclusive);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated formal body");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{element:?} {exclusive}: {errors:?}\n{source}"));
            let callee =
                program.modules().next().expect("module").functions().nth(1).expect("callee");
            let instructions =
                callee.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            assert!(
                instructions.iter().all(|i| i.indexed_borrow().is_none()),
                "callee receives exact referent, not its container"
            );
            let read = instructions
                .iter()
                .find(|i| {
                    matches!(
                        i.kind(),
                        VerifiedInstructionKind::BorrowRead
                            | VerifiedInstructionKind::GenericCloneBorrow
                    )
                })
                .expect("formal read");
            let borrow = read.borrow().expect("formal authority");
            if matches!(element, Element::I32) {
                assert_eq!(read.kind(), VerifiedInstructionKind::BorrowRead);
            } else {
                assert!(read.generic_clone().is_some());
                assert_eq!(read.failure_ended_borrows().collect::<Vec<_>>(), [borrow]);
            }
            let replacement = instructions.iter().find(|i| {
                matches!(
                    i.kind(),
                    VerifiedInstructionKind::BorrowWrite | VerifiedInstructionKind::BorrowReplace
                )
            });
            assert_eq!(replacement.is_some(), exclusive);
            if let Some(replacement) = replacement {
                assert_eq!(replacement.borrow(), Some(borrow));
                if !matches!(element, Element::I32) {
                    let view = replacement.borrow_replacement().expect("owned formal sink");
                    assert_eq!(view.referent(), read.generic_clone().expect("owned clone").ty());
                    assert_eq!(view.old_value_drop().borrow(), borrow);
                }
            }
        }
    }
}
