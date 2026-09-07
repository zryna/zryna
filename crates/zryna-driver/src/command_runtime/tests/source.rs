use super::{
    super::{CommandHostPolicy, prepare_command_self_check},
    fixtures,
};

#[test]
fn real_source_command_executes_typed_success_wrapping_and_expected_failure() {
    let frontend = fixtures::frontend();
    let sources = fixtures::sources();
    let wit = fixtures::wit();
    let policy = CommandHostPolicy::deny_all();
    for (arguments, expected, result) in
        [([20, 22], 42, Ok(())), ([i32::MAX, 1], i32::MIN, Ok(())), ([20, 22], 41, Err(()))]
    {
        let prepared = prepare_command_self_check(
            &frontend, &sources, &wit, "add", &arguments, expected, policy,
        )
        .expect("real source must pass frontend, IR, composition and component audits");
        assert!(prepared.diagnostics().is_empty());
        assert_eq!(prepared.execute(policy).expect("bounded command runtime"), result);
    }
}

#[test]
fn command_rejects_stale_binding_and_host_policy_before_runtime_creation() {
    let frontend = fixtures::frontend();
    let sources = fixtures::sources();
    let policy = CommandHostPolicy::deny_all();
    let mut prepared = prepare_command_self_check(
        &frontend,
        &sources,
        &fixtures::wit(),
        "add",
        &[20, 22],
        42,
        policy,
    )
    .expect("matching source baseline");
    let changed = CommandHostPolicy { revision: "changed-policy" };
    assert_eq!(prepared.execute(changed).expect_err("stale policy")[0].code(), "ZRYNA-C4020");
    prepared.binding[0] ^= 1;
    assert_eq!(prepared.execute(policy).expect_err("stale binding")[0].code(), "ZRYNA-C4020");
}
