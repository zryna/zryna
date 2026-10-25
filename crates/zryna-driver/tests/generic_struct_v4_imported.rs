//! Additive imported nominal qualification; the existing nested78 total is separate.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_struct_v4_imported/emit.rs"]
mod emit;
#[path = "generic_struct_v4_imported/execute.rs"]
mod execute;
#[path = "generic_struct_v4_imported/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_struct_v4_imported/source.rs"]
mod source;

#[test]
fn three_module_aliases_and_caller_decoys_share_one_closed_nominal_seal() {
    oracle::check_frozen_authority();
    execute::check_tools();
    source::with_program("imported-alias-decoy", |program, wire, evidence| {
        let artifacts = emit::all_targets("imported-alias-decoy", program, wire, evidence);
        let counts = execute::all_targets("imported-alias-decoy", &artifacts);
        assert_eq!(counts.observations, [1, 1, 1]);
        assert_eq!(counts.native_faults, 0);
        println!("Imported nominal same-seal: {}", artifacts.directory.display());
    });
}
