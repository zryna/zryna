//! Actual same-seal discovery first-extra control; no forged TypeView/Clone node.
use super::*;
use crate::generic_execution_v1::tests::source;
#[test]
fn clone_model_metadata_rejects_before_nonempty_reserve() {
    source::program("nominal-enum-clone", "native", |p| {
        let mut spent = crate::generic_execution_v1::MAX_BYTES
            - p.layouts(StorageTarget::Linear32V1).types().len();
        work::reset_seen();
        assert!(collect(p, &mut spent).is_err());
        assert_eq!(work::seen(), 0);
        let mut spent = 0;
        collect(p, &mut spent).expect("fixed genuine source/target control");
        // One retained type table plus the Signal frontier's three-row child census.
        // Scalar child expansion is empty; the first-extra branch above still reserves zero.
        assert_eq!(work::seen(), 2);
    });
}

#[test]
fn exact_signal_children_count_none_rows_before_visiting_or_reserving() {
    source::program("nominal-enum-clone", "native", |program| {
        let node = program
            .operations()
            .find_map(zryna_ir::generic_v1::execution_v1::VerifiedOperation::clone_frontier)
            .expect("actual source Clone")
            .frontier();
        assert_eq!(node.variants().len(), 3);
        assert_eq!(node.variants().filter(|(_, child)| child.is_some()).count(), 2);
        let cost = 3 + 3 * size_of::<VerifiedCloneNode<'_, '_>>();
        let mut spent = crate::generic_execution_v1::MAX_BYTES - cost + 1;
        let mut stack = Vec::new();
        work::reset_seen();
        assert!(expand(node, &mut stack, &mut spent).is_err());
        assert!(stack.is_empty());
        assert_eq!(work::seen(), 0);
        let mut spent = crate::generic_execution_v1::MAX_BYTES - cost;
        assert_eq!(expand(node, &mut stack, &mut spent).expect("exact child census"), 3);
        assert_eq!(spent, crate::generic_execution_v1::MAX_BYTES);
        assert_eq!(stack.len(), 2);
        assert_eq!(work::seen(), 1);
    });
}
