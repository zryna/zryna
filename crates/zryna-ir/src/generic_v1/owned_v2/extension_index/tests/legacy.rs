//! Both old issuer domains retain None and never invoke the private index builder.
use super::super::statistics;
use crate::generic_v1::owned_v2;

use crate::generic_v1::source_abi::tests::fixture;

#[test]
fn old_v2_v3_seals_keep_no_index_and_no_index_budget_or_reserves() {
    let fixture = fixture::Fixture::new("i32", "i32");
    fixture.owned().expect("old scalar owner entrypoint remains valid");
    fixture.copy().expect("old Copy entrypoint remains valid");
    let claim = owned_v2::produce_claim(
        &fixture.syntax,
        &fixture.sources,
        &fixture.linear,
        &fixture.linux,
        &[],
    )
    .expect("genuine unchanged original scalar claims");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&fixture.linear, &fixture.linux)
            .expect("runtime claims"),
        &fixture.linear,
        &fixture.linux,
    )
    .expect("runtime authority");
    for version in [2, 3] {
        let decoded = if version == 2 {
            owned_v2::wire::decode(&owned_v2::wire::encode(&claim).expect("v2 bytes"))
        } else {
            owned_v2::wire::v3::decode(&owned_v2::wire::v3::encode(&claim).expect("v3 bytes"))
        }
        .expect("old untrusted decoding");
        statistics::reset();
        let program = owned_v2::verify(
            decoded,
            &fixture.syntax,
            &fixture.sources,
            fixture.sources.verify_file_id(0).expect("entry"),
            &fixture.linear,
            &fixture.linux,
            &runtime,
        )
        .expect("old source/type/ownership issuer");
        assert!(program.extension_index.is_none());
        assert_eq!(program.extension(0, 0), None);
        assert_eq!(statistics::observed(), (None, None, 0));
    }
}
