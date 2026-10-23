//! Linux same-seal Struct-v4 execution, with independent prefix-fault observations.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_struct_v4/emit.rs"]
mod emit;
#[path = "generic_struct_v4/execute.rs"]
mod execute;
#[path = "generic_struct_v4/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4/process.rs"]
mod process;
#[path = "generic_struct_v4/source.rs"]
mod source;

#[test]
fn authentic_native_and_typescript_structs_share_one_three_target_prefix_fault_seal() {
    oracle::check_frozen_authority();
    execute::check_tools();
    for case in
        [source::Case::Construction, source::Case::DeclarationOrder, source::Case::Reordered]
    {
        source::with_program(case, |program, wire| {
            let artifacts = emit::all_targets(case, program, wire);
            execute::all_targets(case, &artifacts);
            println!("Struct v4 same-seal {:?}: {}", case, artifacts.directory.display());
        });
    }
}
