use super::{
    super::{audit, emit_command_self_check},
    fixtures,
};

#[test]
fn authentic_command_component_passes_full_audit_and_preserves_exact_core() {
    let program = fixtures::program();
    let component = emit_command_self_check(&program, &fixtures::sources(), "add", &[20, 22], 42)
        .expect("authentic emitted component must pass before any mutation tests");
    assert_eq!(component.core().bytes(), crate::emit(&program).expect("core emission").bytes());
    assert_ne!(component.bytes(), component.core().bytes());
    component.revalidate(&program).expect("matching program binding");
    assert_eq!(component.bridge_revision(), "zryna.command-self-check.v1");
}

#[test]
fn whole_binary_audit_rejects_version_changes_and_core_only_input() {
    let program = fixtures::program();
    let component = emit_command_self_check(&program, &fixtures::sources(), "add", &[20, 22], 42)
        .expect("positive baseline");
    let check = |bytes: &[u8]| {
        audit::audit(bytes, &component.core, &component.invocation, &component.world)
    };
    check(component.bytes()).expect("independent baseline audit");
    assert!(check(component.core().bytes()).is_err());
    let mut changed = component.bytes().to_vec();
    let name = b"wasi:cli/environment@0.2.12";
    let offset =
        changed.windows(name.len()).position(|bytes| bytes == name).expect("environment import");
    changed[offset + name.len() - 1] = b'3';
    assert!(check(&changed).is_err());
}

#[test]
fn bridge_substitution_and_noncanonical_result_instructions_are_rejected() {
    let program = fixtures::program();
    let component = emit_command_self_check(&program, &fixtures::sources(), "add", &[20, 22], 42)
        .expect("positive baseline");
    let wrong = emit_command_self_check(&program, &fixtures::sources(), "add", &[20, 22], 41)
        .expect("a deliberate wrong expected result is a valid command arrangement");
    assert!(
        audit::audit(wrong.bytes(), &component.core, &component.invocation, &component.world)
            .is_err()
    );
    let mut bridge = super::super::bridge::encode(&component.invocation);
    super::super::bridge_audit::audit(&bridge, &component.invocation).expect("bridge baseline");
    let comparison = bridge.len() - 2;
    assert_eq!(bridge[comparison], 0x47);
    bridge[comparison] = 0x6a;
    assert!(super::super::bridge_audit::audit(&bridge, &component.invocation).is_err());
}

#[test]
fn bridge_argument_envelope_accepts_exact_arity_and_rejects_first_extra() {
    let exact = fixtures::program_with_arity(32);
    emit_command_self_check(&exact, &fixtures::sources(), "add", &[0; 32], 0)
        .expect("exact bridge argument envelope");
    let extra = fixtures::program_with_arity(33);
    let error = emit_command_self_check(&extra, &fixtures::sources(), "add", &[0; 33], 0)
        .err()
        .expect("first extra bridge argument");
    assert_eq!(error.code(), "ZRYNA-W4010");
}
