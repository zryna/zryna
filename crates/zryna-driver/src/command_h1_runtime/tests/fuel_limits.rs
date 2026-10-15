//! Private raw-core metering; not public H1 source or artifact admission.

use crate::runtime_boundary_tests as fuel_meter;

#[test]
fn private_h1_fixed_fuel100000_measured_exhaustion_and_next_call_rejection() {
    let engine = super::super::envelope::engine().expect("unchanged H1 engine");
    fuel_meter::measured_production_exhaustion(
        &engine,
        super::super::envelope::limits(),
        super::super::envelope::FUEL,
    );
    super::fresh_recovery();
}
