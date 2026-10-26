//! Exact accepted construction and old-issuer opcode-free laundering controls.
use super::{authority::Authorities, fixture::Fixture};
use zryna_ir::generic_v1::owned_v2 as owned;

fn check_source(fixture: &Fixture, pass_only: bool) {
    let (linear, linux) = fixture.layouts();
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("runtime authority");
    owned::check_original_bodies_v4(&fixture.syntax, &fixture.sources)
        .expect("every original has private v4 opaque-frame proof");
    let claim = owned::produce_claim_v4(&fixture.syntax, &fixture.sources, &linear, &linux, &[])
        .expect("new private producer");
    let bytes = owned::wire::v4::encode(&claim).expect("v4 wire");
    let authorities = Authorities { fixture, linear: &linear, linux: &linux, runtime: &runtime };
    authorities.seal(&bytes).expect("new source/layout/typed/owner issuer");
    if pass_only {
        authorities.old_issuers_reject_copied_claims(&bytes);
    } else {
        authorities.construction_attacks(&claim);
    }
    authorities.foreign_source_rejects(&bytes);
    authorities.seal(&bytes).expect("pristine independent seal after attacks");
}

#[test]
fn exact_accepted_source_constructs_and_observes_copy_field() {
    check_source(&Fixture::new("construction", true), false);
}
#[test]
fn both_old_issuers_reject_generic_and_plain_struct_pass_only_signatures() {
    check_source(&Fixture::new("pass-generic", true), true);
    check_source(&Fixture::new("pass-plain", false), true);
}
