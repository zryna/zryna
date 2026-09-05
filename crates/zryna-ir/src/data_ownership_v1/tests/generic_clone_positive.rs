use super::generic_clone_fixture::{Fixture, GraphKind};
use super::*;
use zryna_layout::TypeCategory;

#[test]
fn generic_clone_recursive_vec_indirection_seals_each_reachable_type_once() {
    let fixture = Fixture::with_graph(TypeCategory::Vec, GraphKind::Recursive);
    for _ in 0..2 {
        let verified = fixture.verify(fixture.seed());
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
        let clone = instruction.generic_clone().expect("recursive clone");
        let frontier = clone.frontier();
        assert_eq!(frontier.clone_operation().source(), clone.source());
        let types = frontier.types().collect::<Vec<_>>();
        assert_eq!(types.len(), 8);
        assert!(types.windows(2).all(|pair| pair[0].id().index() < pair[1].id().index()));
        let inner = types.iter().find(|ty| ty.nominal_identity() == Some((0, 0))).expect("inner");
        assert_eq!(inner.fields().last().expect("recursive Vec field").ty(), clone.ty());
    }
}

#[test]
fn generic_clone_seals_recursive_type_frontier_and_separate_pending_prefix_cleanup() {
    for (category, type_count) in [
        (TypeCategory::Struct, 7),
        (TypeCategory::Enum, 4),
        (TypeCategory::FixedArray, 6),
        (TypeCategory::Vec, 8),
    ] {
        let fixture = Fixture::new(category);
        let raw = fixture.seed();
        let mut previous = None;
        for _ in 0..2 {
            let verified = fixture.verify(raw.clone());
            let function =
                verified.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instruction = block.instructions().next().expect("clone");
            let clone = instruction.generic_clone().expect("sealed generic clone");
            assert!(
                matches!(clone.source(), super::super::VerifiedGenericCloneSource::Place(place) if place.index() == 0)
            );
            assert_eq!(clone.destination().index(), 2);
            assert_eq!(clone.result().index(), 3);
            assert_eq!(clone.ty().index(), fixture.root.0);
            assert_eq!(clone.cleanup().index(), 0);
            assert_eq!(clone.prefix_cleanup().index(), 1);
            let types = clone.frontier().types().map(|ty| ty.id().index()).collect::<Vec<_>>();
            assert_eq!(types.len(), type_count);
            assert!(types.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(types.contains(&fixture.root.0));
            assert!(types.contains(&fixture.string.0));
            assert!(types.contains(&fixture.integer.0), "all enum alternatives enter the frontier");
            let prepare = instruction.derived_drop_actions().collect::<Vec<_>>();
            let prefix =
                instruction.generic_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
            assert_eq!(prepare.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(), [1, 0]);
            assert_eq!(
                prefix.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(),
                [2, 1, 0]
            );
            assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
            assert_eq!(&prefix[1..], prepare.as_slice());
            assert_eq!(prepare, block.terminator().derived_drop_actions().collect::<Vec<_>>());
            assert!(prepare.iter().all(|drop| drop.moved_projections().count() == 0));
            let prefix_site =
                function.cleanup_plans().find(|plan| plan.id().index() == 1).expect("prefix site");
            assert_eq!(prefix_site.site().role(), VerifiedCleanupRole::GenericClonePrefixFailure);
            let observation = (types, prepare, prefix);
            if let Some(previous) = previous.replace(observation.clone()) {
                assert_eq!(previous, observation);
            }
        }
    }
}
