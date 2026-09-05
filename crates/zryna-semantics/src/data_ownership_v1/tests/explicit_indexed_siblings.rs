use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::generic_vec_fixture::Element;
use super::*;

#[test]
fn explicit_indexed_siblings_static_array_elements_are_disjoint_but_dynamic_indices_conflict() {
    for element in [Element::I32, Element::String, Element::Struct, Element::Enum] {
        for index in [Some(0), None] {
            let (source, raw) =
                fixture(Container::Array(2), &element, true, Action::Conflict, index);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources)
                .expect("authenticated static/dynamic element regions");
            if index.is_none() {
                let check = || {
                    lower(pair_input(&syntax, &sources)).expect_err("dynamic whole-array conflict")
                };
                let errors = check();
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].code(), "ZRYNA-M3014");
                assert_eq!(errors, check());
                continue;
            }
            let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                panic!("disjoint static array elements: {errors:?}\n{source}")
            });
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let begins = instructions
                .iter()
                .filter(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                .collect::<Vec<_>>();
            assert_eq!(begins.len(), 2);
            assert!(instructions.iter().all(|i| i.indexed_borrow().is_none()));
            let mut bases = Vec::new();
            for (index, begin) in begins.iter().enumerate() {
                assert_eq!(begin.borrow_access(), Some(VerifiedBorrowAccess::Exclusive));
                assert!(begin.cleanup().is_none());
                let place = begin.place_operands().next().expect("static element");
                let kind = function.places().find(|p| p.id() == place).expect("element").kind();
                let VerifiedPlaceKind::FixedArrayConstant { base, index: actual } = kind else {
                    panic!("static element");
                };
                assert_eq!(actual, u32::try_from(index).expect("index"));
                bases.push(base);
            }
            assert_eq!(bases[0], bases[1]);
            assert_eq!(
                instructions
                    .iter()
                    .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .map(|i| i.borrow())
                    .collect::<Vec<_>>(),
                [begins[1].borrow(), begins[0].borrow()]
            );
        }
    }
}

#[test]
fn explicit_indexed_siblings_distinct_static_vec_containers_preserve_exact_regions_and_failure_end_order()
 {
    for action in [Action::SiblingVec, Action::SameVec] {
        let (source, raw) = fixture(Container::Array(2), &Element::Vec, true, action, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated static Vec siblings");
        if matches!(action, Action::SameVec) {
            let check = || {
                lower(pair_input(&syntax, &sources))
                    .expect_err("same Vec container conflicts even for unequal indices")
            };
            let errors = check();
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M3014");
            assert_eq!(errors, check());
            continue;
        }
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("disjoint Vec siblings: {errors:?}\n{source}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let begins =
                instructions.iter().filter(|i| i.indexed_borrow().is_some()).collect::<Vec<_>>();
            assert_eq!(begins.len(), 2);
            let first = begins[0].indexed_borrow().expect("first Vec");
            let second = begins[1].indexed_borrow().expect("second Vec");
            assert_ne!(first.container(), second.container());
            assert_eq!(first.referent(), second.referent());
            assert_eq!(first.array_length(), None);
            assert_eq!(second.array_length(), None);
            let mut roots = Vec::new();
            for (ordinal, container) in
                [first.container(), second.container()].into_iter().enumerate()
            {
                let kind =
                    function.places().find(|p| p.id() == container).expect("Vec container").kind();
                let VerifiedPlaceKind::FixedArrayConstant { base, index } = kind else {
                    panic!("static Vec container");
                };
                assert_eq!(index, u32::try_from(ordinal).expect("ordinal"));
                roots.push(base);
            }
            assert_eq!(roots[0], roots[1]);
            assert_eq!(begins[1].failure_ended_borrows().collect::<Vec<_>>(), [first.borrow()]);
            assert!(
                begins[1]
                    .derived_drop_actions()
                    .any(|a| a.root() == roots[0] && a.moved_projections().len() == 0)
            );
            assert_eq!(
                instructions
                    .iter()
                    .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .map(|i| i.borrow().expect("end"))
                    .collect::<Vec<_>>(),
                [second.borrow(), first.borrow()]
            );
            assert!(!block.terminator().derived_drop_actions().any(|a| a.root() == roots[0]));
        }
    }
}
