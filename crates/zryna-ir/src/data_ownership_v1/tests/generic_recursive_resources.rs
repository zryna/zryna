use super::generic_clone_fixture::{Fixture, GraphKind};
use super::*;
use zryna_layout::TypeCategory;

#[test]
fn recursive_clone_resource_preflight_is_exact_checked_and_replay_stable() {
    let fixture = Fixture::with_graph(TypeCategory::Vec, GraphKind::Recursive);
    let seed = fixture.seed();
    fixture.verify(seed.clone());
    let mut program = seed.clone();
    let template = program.modules[0].functions[0].cleanup_plans[0].clone();
    program.modules[0].functions[0]
        .cleanup_plans
        .resize(MAX_CLEANUP_PLANS_PER_FUNCTION, template.clone());
    let mut exact = Errors::default();
    super::super::preflight(&program, &fixture.linear, &mut exact);
    assert!(exact.is_empty());

    program.modules[0].functions[0].cleanup_plans.push(template);
    let check = || {
        let mut errors = Errors::default();
        super::super::preflight(&program, &fixture.linear, &mut errors);
        errors.finish()
    };
    let first = check();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-I3201");
    assert_eq!(first, check());

    let mut overflow = Errors::default();
    assert_eq!(
        super::super::checked_add(usize::MAX, 1, "recursive clone cleanup count", &mut overflow),
        usize::MAX
    );
    assert_eq!(overflow.finish()[0].code(), "ZRYNA-I3201");
    fixture.verify(seed);
}
