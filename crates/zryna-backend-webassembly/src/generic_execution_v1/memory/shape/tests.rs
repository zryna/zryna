//! Actual inventory rejects first-extra metadata before its first nonempty reserve.
use super::*;
use crate::generic_execution_v1::tests::source;
#[test]
fn inventory_prepays_first_type_index_reserve() {
    source::program("nominal-enum-clone", "native", |p| {
        let mut spent = crate::generic_execution_v1::MAX_BYTES
            - p.functions().len()
            - p.layouts(StorageTarget::Linear32V1).types().len();
        crate::generic_execution_v1::work::reset_seen();
        assert!(inventory(p, &mut spent).is_err());
        assert_eq!(crate::generic_execution_v1::work::seen(), 0);
        let mut spent = 0;
        inventory(p, &mut spent).expect("fixed genuine source/target control");
        assert_eq!(crate::generic_execution_v1::work::seen(), 1);
    });
}
