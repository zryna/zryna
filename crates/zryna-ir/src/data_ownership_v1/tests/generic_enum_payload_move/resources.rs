use super::*;

#[test]
fn generic_enum_payload_move_cleanup_preflight_exact_extra_and_checked_overflow_recover() {
    let fixture = Fixture::new(TypeCategory::Enum);
    let valid = seed(&fixture, false);
    check(&fixture, valid.clone()).expect("authenticated seed");
    // Counter-only boundary: repeated raw plans do not claim complete program authentication.
    let mut raw = valid.clone();
    let template = raw.modules[0].functions[0].cleanup_plans[0].clone();
    raw.modules[0].functions[0]
        .cleanup_plans
        .resize(MAX_CLEANUP_PLANS_PER_FUNCTION, template.clone());
    let mut exact = Errors::default();
    super::super::super::preflight(&raw, &fixture.linear, &mut exact);
    assert!(exact.is_empty());
    raw.modules[0].functions[0].cleanup_plans.push(template);
    let mut extra = Errors::default();
    super::super::super::preflight(&raw, &fixture.linear, &mut extra);
    let errors = extra.finish();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    let mut overflow = Errors::default();
    assert_eq!(
        super::super::super::checked_add(
            usize::MAX,
            1,
            "enum payload cleanup count",
            &mut overflow
        ),
        usize::MAX
    );
    assert_eq!(overflow.finish()[0].code(), "ZRYNA-I3201");
    check(&fixture, valid).expect("recovery after terminal preflight");
}
