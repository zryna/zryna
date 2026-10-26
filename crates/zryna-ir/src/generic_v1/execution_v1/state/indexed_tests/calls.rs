//! The actual call transition must forget discriminants an exclusive callee can replace.
use super::super::raw;
mod fixture;
use fixture::{derive, fixture};
#[test]
fn shared_call_preserves_a_valid_prior_container_element_refinement() {
    for dynamic in [false, true] {
        let (world, p) = fixture(dynamic, raw::Access::Shared);
        derive(&world, &p).expect("read-only call cannot change the selected enum discriminant");
    }
}
#[test]
fn exclusive_call_cannot_authorize_stale_static_or_indexed_enum_payload() {
    for dynamic in [false, true] {
        let (world, p) = fixture(dynamic, raw::Access::Exclusive);
        assert!(
            derive(&world, &p).is_err(),
            "callee may replace enum before success or later failure; old payload refinement is not authority"
        );
    }
}
