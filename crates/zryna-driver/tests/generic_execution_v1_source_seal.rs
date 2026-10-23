//! Genuine current native source parsing and NEW issuer controls; no backend claim.
#[path = "generic_execution_v1_first_vertical/authority.rs"]
mod authority;
#[path = "generic_execution_v1_first_vertical/negative.rs"]
mod negative;
#[path = "generic_execution_v1_first_vertical/source.rs"]
mod source;
#[test]
fn five_exact_originals_and_both_provider_packets_reach_the_new_execution_seal() {
    authority::check_all();
    let mut issued = 0;
    for case in authority::POSITIVE {
        source::with_program(case, |program, wire, evidence| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert!(!evidence.source.is_empty());
            assert!(!evidence.native.is_empty());
            assert!(!evidence.typescript.is_empty());
            assert_eq!(program.scalar_abi().exports().len(), 1);
            issued += 1;
        });
    }
    assert_eq!(issued, 5, "genuine independent issuances, no target execution counted");
}
#[test]
fn unused_opaque_original_never_reaches_a_claim_or_backend() {
    negative::check();
}
