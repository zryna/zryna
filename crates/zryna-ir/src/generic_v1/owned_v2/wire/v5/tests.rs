//! Inherited accounting primitives remain checked before any vector reservation.
use super::super::{MAX_CHILDREN, Reader};

#[test]
fn v5_retains_exact_child_credit_and_rejects_extra_and_arithmetic_overflow() {
    let data = [1, 0, 0, 0, 9, 0, 0, 0];
    let mut reader = Reader { bytes: &data, position: 0, children: MAX_CHILDREN - 1 };
    assert_eq!(reader.vector(1, 4, Reader::u32).expect("exact child credit"), vec![9]);
    for children in [MAX_CHILDREN, usize::MAX] {
        let mut reader = Reader { bytes: &data, position: 0, children };
        assert!(reader.vector(1, 4, Reader::u32).is_err());
        assert_eq!(reader.position, 4, "reject before reading/reserving vector children");
    }
    let data = [2, 0, 0, 0];
    let mut reader = Reader { bytes: &data, position: 0, children: 0 };
    assert!(reader.vector(2, usize::MAX, Reader::u32).is_err());
    let mut reader = Reader { bytes: &data, position: usize::MAX, children: 0 };
    assert!(reader.take(1).is_err());
}
