//! Exact generated binding/type and real CFG dominance controls, without source authority.
use super::{check, raw};
use crate::generic_v1::execution_v1::typed::{
    cfg::Graph,
    container_test_support,
    continuation_test_support::{Types, upgrade, value},
};
fn run(w: &Types, p: &raw::Program) -> Result<(), super::Failure> {
    let t = container_test_support::phase_inputs(p, &w.world);
    check(&p.functions[0], &t.functions[0], &w.world, 0, &mut 0)
}
#[test]
fn upgrade_requires_exact_weak_and_same_referent_shared_binding() {
    let w = Types::new();
    let p = upgrade(&w);
    run(&w, &p).expect("actual Weak<String> to Shared<String> branch schema");
    let mut wrong = upgrade(&w);
    wrong.functions[0].parameters[0] = raw::Parameter::Value(value(0, w.nominal_weak));
    wrong.functions[0].places[0].ty = w.nominal_weak;
    wrong.functions[0].blocks[1].value_parameters[0] = value(1, w.foreign_shared);
    wrong.functions[0].places[1].ty = w.foreign_shared;
    let raw::Terminator::WeakUpgradeBranch { success_binding, .. } =
        &mut wrong.functions[0].blocks[0].terminator
    else {
        unreachable!()
    };
    *success_binding = value(1, w.foreign_shared);
    assert!(
        run(&w, &wrong).is_err(),
        "equal-shaped A/B referents stay distinct in coherent binding"
    );
    let mut head = upgrade(&w);
    head.functions[0].parameters[0] = raw::Parameter::Value(value(0, w.shared));
    head.functions[0].places[0].ty = w.shared;
    assert!(run(&w, &head).is_err(), "Shared cannot masquerade as Weak");
}
#[test]
fn success_binding_is_first_only_and_cannot_dominate_expired_or_join() {
    let w = Types::new();
    let mut wrong = upgrade(&w);
    let raw::Terminator::WeakUpgradeBranch { success_binding, .. } =
        &mut wrong.functions[0].blocks[0].terminator
    else {
        unreachable!()
    };
    *success_binding = value(0, w.weak);
    assert!(Graph::build(&wrong.functions[0], w.world.len(), &mut 0).is_err());
    let mut expired = upgrade(&w);
    expired.functions[0].blocks[2].terminator =
        raw::Terminator::Return { value: Some(1), cleanup: 2 };
    let t = container_test_support::phase_inputs(&expired, &w.world);
    assert!(
        check(&expired.functions[0], &t.functions[0], &w.world, 2, &mut 0).is_err(),
        "success owner does not dominate expiry"
    );
    let mut extra = upgrade(&w);
    let raw::Terminator::WeakUpgradeBranch { expired, .. } =
        &mut extra.functions[0].blocks[0].terminator
    else {
        unreachable!()
    };
    expired.arguments.push(1);
    assert!(
        Graph::build(&extra.functions[0], w.world.len(), &mut 0).is_err(),
        "expiry has no generated argument"
    );
}
