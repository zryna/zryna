#[test]
fn complete_owner_state_and_cleanup_credits_reject_first_extra_and_overflow() {
    let mut credits = 0;
    super::charge(&mut credits, super::MAX_OWNER_UNITS).expect("exact complete credit ceiling");
    assert!(
        matches!(super::charge(&mut credits,1),Err(super::Failure::Diagnostics(v)) if v[0].code=="ZRYNA-I3201")
    );
    let mut overflow = usize::MAX;
    assert!(super::charge(&mut overflow, 1).is_err());
}
