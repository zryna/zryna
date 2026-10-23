//! Test-first genuine whole source seals must enter the new native owner.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_containers/source.rs"]
mod source;
fn lower(case: &str) {
    assert!(source::SETTLED.contains(&case));
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let native = zryna_native_mir::generic_execution_v1::lower(program)
                .expect("desired genuine container native admission; owning current N7002 is behavioral RED");
            assert!(std::ptr::eq(native.program(), program));
            assert_eq!(native.functions().len(), program.functions().len());
        });
    }
}
#[test]
fn genuine_vec_copy_enters_native_mir() {
    lower("vec-copy");
}
#[test]
fn genuine_vec_owned_preparation_enters_native_mir() {
    lower("vec-owned-prepare");
}
#[test]
fn genuine_shared_weak_count_enters_native_mir() {
    lower("shared-weak-count");
}
#[test]
fn genuine_unused_opaque_count_templates_enter_native_mir() {
    lower("opaque-count-unused");
}
