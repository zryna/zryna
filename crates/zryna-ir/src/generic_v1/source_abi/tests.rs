//! ABI diagnostic classification is reached through complete original-source and owner proofs.
mod fixture;
mod selection;
use crate::generic_v1::Failure;
use fixture::Fixture;

fn exported_name(failure: Failure, fixture: &Fixture) {
    let Failure::Diagnostics(errors) = failure else {
        panic!("source diagnostic required");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M7005");
    let name = errors[0].primary_span().expect("source exported name");
    assert_eq!((name.start(), name.end()), (16, 21));
    assert_eq!(name.file(), fixture.sources.verify_file_id(0).expect("entry"));
}

#[test]
fn string_parameter_result_and_each_individual_boundary_own_the_exported_name() {
    for (parameter, result) in [("String", "String"), ("String", "i32"), ("i32", "String")] {
        let fixture = Fixture::new(parameter, result);
        exported_name(fixture.owned().expect_err("scalar ABI excludes String"), &fixture);
    }
}

#[test]
fn copy_option_parameter_and_result_are_rejected_by_source_abi_in_both_seals() {
    let fixture = Fixture::new("Option<i32>", "Option<i32>");
    exported_name(fixture.owned().expect_err("owned seal excludes public Option"), &fixture);
    exported_name(fixture.copy().expect_err("Copy seal excludes public Option"), &fixture);
}

#[test]
fn bool_and_i32_exports_keep_both_existing_private_scalar_issuers() {
    for scalar in ["bool", "i32"] {
        let fixture = Fixture::new(scalar, scalar);
        fixture.owned().expect("owned private scalar ABI unchanged");
        fixture.copy().expect("Copy private scalar ABI unchanged");
    }
}
