use super::super::MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION;
use super::indexed_access_resources::at_capacity;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn cached_boundary(fixture: &Fixture, extra: bool) -> raw::Program {
    const ACTIVE: usize = 256;
    let entries = MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION / ACTIVE;
    assert_eq!(entries * ACTIVE, MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION);
    let mut program = at_capacity(fixture, ACTIVE);
    let function = &mut program.modules[0].functions[0];
    let tail = function.blocks[0].instructions.split_off(ACTIVE);
    let returns = function.blocks[0].terminators.clone();
    for id in 0..=entries {
        if id > 0 {
            function.blocks.push(raw::Block {
                id: raw::BlockId(u32::try_from(id).unwrap()),
                parameters: vec![],
                instructions: vec![],
                terminators: returns.clone(),
            });
        }
        function.blocks[id].terminators[0].kind = raw::Terminator::Jump(raw::Edge {
            target: raw::BlockId(u32::try_from(id + 1).unwrap()),
            arguments: vec![],
        });
    }
    function.blocks[entries].instructions = tail;
    if extra {
        let last_end = function.blocks[entries].instructions.pop().unwrap();
        function.blocks.push(raw::Block {
            id: raw::BlockId(u32::try_from(entries + 1).unwrap()),
            parameters: vec![],
            instructions: vec![last_end],
            terminators: returns,
        });
    } else {
        function.blocks[entries].terminators = returns;
    }
    program
}

#[test]
fn transient_indexed_resources_cache_exact_first_extra_and_recovery() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let verified = fixture.verify(cached_boundary(&fixture, false));
    let function = verified.modules().next().unwrap().functions().next().unwrap();
    assert_eq!(function.blocks().count(), 1_025);
    assert_eq!(function.places().count(), 3);
    let project = function.blocks().last().unwrap().instructions().next().unwrap();
    assert_eq!(project.failure_ended_borrows().count(), 256);
    fixture.rejects(cached_boundary(&fixture, true), "ZRYNA-I3201");
    fixture.verify(at_capacity(&fixture, 1));
}
