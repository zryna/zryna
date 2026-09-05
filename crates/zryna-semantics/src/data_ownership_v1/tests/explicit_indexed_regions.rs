use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::generic_vec_fixture::Element;
use super::*;

#[test]
fn explicit_indexed_regions_root_shared_authority_agrees_with_indexed_container_region() {
    for container in [Container::Array(2), Container::Vec] {
        for element in [Element::I32, Element::String, Element::Struct, Element::Enum] {
            for exclusive in [false, true] {
                let (source, raw) =
                    fixture(container, &element, exclusive, Action::RootShared, None);
                let sources = sources_for(&source);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated root/indexed aliases");
                if exclusive {
                    let check = || {
                        lower(pair_input(&syntax, &sources))
                            .expect_err("exclusive indexed region excludes root shared access")
                    };
                    let first = check();
                    assert_eq!(first.len(), 1);
                    assert_eq!(first[0].code(), "ZRYNA-M3014", "{source}: {first:?}");
                    assert_eq!(first, check());
                } else {
                    let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                        panic!("compatible shared root/indexed borrows: {errors:?}\n{source}")
                    });
                    let function = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("function");
                    let instructions =
                        function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                    let indexed = instructions
                        .iter()
                        .find_map(|i| i.indexed_borrow())
                        .expect("indexed authority");
                    let whole = instructions
                        .iter()
                        .find(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                        .expect("whole container authority");
                    assert_eq!(whole.place_operands().collect::<Vec<_>>(), [indexed.container()]);
                    assert_eq!(whole.borrow_access(), Some(VerifiedBorrowAccess::Shared));
                    assert_eq!(
                        instructions
                            .iter()
                            .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                            .map(|i| i.borrow().expect("ended borrow"))
                            .collect::<Vec<_>>(),
                        [whole.borrow().expect("whole identity"), indexed.borrow()]
                    );
                }
            }
        }
    }
}
