use super::generic_clone_fixture::{Fixture, GraphKind};
use super::generic_vec_observation;
use super::indexed_borrow_fixture::{Container, Element, Fixture as IndexedFixture};
use super::*;
use crate::data_ownership_v1::{VerifiedHandleAwareCloneSource, VerifiedHandleCloneRecipeKind};
use zryna_layout::TypeCategory;

fn seed(category: TypeCategory, graph: GraphKind) -> (Fixture, raw::Program) {
    let fixture = Fixture::with_graph(category, graph);
    let mut program = fixture.seed();
    let raw::InstructionKind::GenericClonePlace { place, cleanup, prefix_cleanup } =
        program.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("generic fixture operation");
    };
    program.modules[0].functions[0].blocks[0].instructions[0].kind =
        raw::InstructionKind::HandleAwareClonePlace { place, cleanup, prefix_cleanup };
    (fixture, program)
}

#[test]
fn seals_shared_and_weak_recursive_clone_recipe_without_unfolding_vec_cycles() {
    for graph in [GraphKind::RecursiveShared, GraphKind::Weak] {
        for category in [TypeCategory::Struct, TypeCategory::Vec] {
            let (fixture, raw) = seed(category, graph);
            let verified = fixture.verify(raw);
            let instruction = verified
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
                .next()
                .expect("clone");
            let clone = instruction.handle_aware_clone().expect("sealed handle clone");
            assert!(matches!(
                clone.source(),
                VerifiedHandleAwareCloneSource::Place(place) if place.index() == 0
            ));
            assert_eq!(clone.ty().index(), fixture.root.0);
            assert_eq!(clone.destination().index(), 2);
            assert_eq!(clone.result().index(), 3);
            let nodes = clone.frontier().nodes().collect::<Vec<_>>();
            assert!(nodes.len() < fixture.linear.types().count());
            assert!(
                nodes.iter().any(|node| matches!(
                    node.kind(),
                    VerifiedHandleCloneRecipeKind::VecEach { .. }
                ))
            );
            assert!(
                nodes
                    .iter()
                    .any(|node| matches!(node.kind(), VerifiedHandleCloneRecipeKind::Enum(_)))
            );
            let expected = if matches!(graph, GraphKind::Weak) {
                VerifiedHandleCloneRecipeKind::WeakCountClone
            } else {
                VerifiedHandleCloneRecipeKind::SharedCountClone
            };
            assert!(nodes.iter().any(|node| *node.kind() == expected));
            let prefix =
                instruction.handle_aware_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
            assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
            assert_eq!(prefix[0].root(), clone.destination());
            assert_eq!(&prefix[1..], instruction.derived_drop_actions().collect::<Vec<_>>());
        }
    }
}

#[test]
fn rejects_non_handle_roots_and_inexact_prefix_cleanup_deterministically() {
    let (ordinary, raw) = seed(TypeCategory::Struct, GraphKind::Ordinary);
    ordinary.rejects_case(raw, "ZRYNA-I3005", "ordinary generic-only graph");

    let (fixture, mut raw) = seed(TypeCategory::Struct, GraphKind::Shared);
    raw.modules[0].functions[0].cleanup_plans[1].actions.swap(0, 1);
    fixture.rejects_case(raw, "ZRYNA-I3013", "destination prefix is not unwound first");
}

#[test]
fn retains_generic_clone_handle_exclusion_as_a_separate_contract() {
    let fixture = Fixture::with_graph(TypeCategory::Struct, GraphKind::Shared);
    fixture.rejects_case(fixture.seed(), "ZRYNA-I3005", "generic clone remains handle-free");
}

#[test]
fn indexed_shared_and_weak_leaves_retain_exact_borrow_count_authority() {
    for element in [Element::Shared, Element::Weak] {
        let fixture = IndexedFixture::new(Container::Vec, element);
        let mut raw = generic_vec_observation::clone_seed(&fixture, raw::BorrowAccess::Shared);
        let raw::InstructionKind::GenericCloneBorrow { borrow, cleanup, prefix_cleanup } =
            raw.modules[0].functions[0].blocks[0].instructions[1].kind
        else {
            panic!("indexed clone fixture");
        };
        raw.modules[0].functions[0].blocks[0].instructions[1].kind =
            raw::InstructionKind::HandleAwareCloneBorrow { borrow, cleanup, prefix_cleanup };
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
        let begin = instructions[0].indexed_borrow().expect("indexed authority");
        let clone = instructions[1].handle_aware_clone().expect("handle leaf clone");
        assert!(matches!(
            clone.source(),
            VerifiedHandleAwareCloneSource::Borrow(source) if source == begin.borrow()
        ));
        assert_eq!(clone.ty(), begin.referent());
        let nodes = clone.frontier().nodes().collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert!(matches!(
            nodes[0].kind(),
            VerifiedHandleCloneRecipeKind::SharedCountClone
                | VerifiedHandleCloneRecipeKind::WeakCountClone
        ));
    }
}
