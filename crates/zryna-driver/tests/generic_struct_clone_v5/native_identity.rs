//! Actual physical pointers and clone-source roles, separate from string bytes.

pub(super) fn check(case: &str, fault: usize, identities: &[serde_json::Value]) {
    use super::oracle;
    use std::collections::{HashMap, HashSet};
    let expected = oracle::events(case, fault);
    assert_eq!(identities.len(), expected.len());
    let mut allocations = HashMap::<usize, (String, u64, bool)>::new();
    let mut addresses = HashSet::new();
    for (identity, event) in identities.iter().zip(expected) {
        let parts = event.splitn(3, ':').collect::<Vec<_>>();
        let role = parts[1].parse::<usize>().expect("fixed allocation or failure index");
        if parts[0] == "alloc" || parts[0] == "fail" {
            assert_eq!(identity["effect"], oracle::operation(case, role));
            let source_role = oracle::clone_source(case, role);
            assert_eq!(identity["sourceRole"], source_role);
            if source_role == 0 {
                assert_eq!(identity["sourcePointer"], "0");
            } else {
                let source = allocations.get(&source_role).expect("actual clone source allocated");
                assert!(!source.2, "actual source remains live during clone preparation/failure");
                assert_eq!(identity["sourcePointer"], source.0);
            }
        }
        if parts[0] == "fail" {
            assert_eq!(identity["operation"], "failure");
            assert_eq!(identity["call"], role);
            continue;
        }
        assert_eq!(identity["role"], role);
        let pointer = identity["pointer"].as_str().expect("actual pointer");
        assert_ne!(u64::from_str_radix(pointer, 16).expect("actual hexadecimal address"), 0);
        let length = identity["length"].as_u64().expect("actual allocation length");
        assert_eq!(length, u64::try_from(parts[2].len()).expect("small literal length"));
        if parts[0] == "alloc" {
            assert_eq!(identity["operation"], "allocate");
            assert!(addresses.insert(pointer.to_owned()), "distinct actual allocation identities");
            assert!(allocations.insert(role, (pointer.into(), length, false)).is_none());
        } else {
            assert_eq!(parts[0], "drop");
            assert_eq!(identity["operation"], "release");
            let allocation = allocations.get_mut(&role).expect("release of actual allocated role");
            assert_eq!(allocation.0, pointer);
            assert_eq!(allocation.1, length);
            assert!(!allocation.2, "exactly one release of each actual pointer");
            allocation.2 = true;
        }
    }
    assert_eq!(allocations.len(), if fault == 0 { oracle::allocations(case) } else { fault - 1 });
    assert!(
        allocations.values().all(|allocation| allocation.2),
        "all successful allocations released"
    );
}
