//! New target seam test-first binding. Compilation RED means missing APIs only.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_first_vertical/authority.rs"]
mod authority;
#[path = "generic_execution_v1_first_vertical/emit.rs"]
mod emit;
#[path = "generic_execution_v1_first_vertical/negative.rs"]
mod negative;
#[path = "generic_execution_v1_first_vertical/source.rs"]
mod source;
#[test]
fn genuine_new_source_seal_binds_only_the_three_new_target_owners() {
    authority::check_all();
    negative::check();
    for case in authority::POSITIVE {
        source::with_program(case, |program, wire, evidence| {
            let artifacts = emit::all(case, program, wire, evidence);
            assert!(artifacts.directory.join("same-new-seal.json").is_file());
        });
    }
}
