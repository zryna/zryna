//! Direct canonical private-kernel contract tests; these create no execution proof or public ABI.
use sha2::{Digest, Sha256};
use std::process::Command;

#[test]
fn reserve_capacity_assembled_kernel_preserves_exact_implementation_bytes() {
    let expected = "987fa645b5f81a9c6fd03f7edcbd23203e7cc61e8aa206b1b08a5ef61d3b1421";
    assert_eq!(format!("{:x}", Sha256::digest(super::KERNEL.as_bytes())), expected);
    assert_eq!(super::IMPLEMENTATION, expected);
}

#[test]
fn reserve_capacity_element_limit_falls_back_to_required() {
    execute("element-fallback");
}

#[test]
fn reserve_capacity_byte_limit_falls_back_before_allocation_policy() {
    execute("byte-fallback");
}

#[test]
fn reserve_capacity_growth_identity_and_atomic_rejection_controls() {
    execute("controls");
}

fn execute(group: &str) {
    let file = std::env::temp_dir()
        .join(format!("zryna-private-reserve-{}-{group}.mjs", std::process::id()));
    let source = format!(
        "const $runtimeQuota=Object.freeze({{allocations:32,transitions:128}});\nconst $runtimeElements=Object.freeze({{0:Object.freeze({{stride:1,alignment:1}}),1:Object.freeze({{stride:4096,alignment:8}})}});\nconst $reserveExpected={};\n{}\n{}",
        include_str!("reserve_tests/EXPECTED.json"),
        super::KERNEL,
        include_str!("reserve_tests/cases.mjs")
    );
    std::fs::write(&file, source).expect("write exact private kernel and frozen contract cases");
    let output = Command::new("node").arg(&file).arg(group).output();
    std::fs::remove_file(&file).expect("remove private kernel test module");
    let output = output.expect("Node private kernel contract process");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
