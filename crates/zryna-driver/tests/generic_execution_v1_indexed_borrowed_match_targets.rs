//! One genuine source seal supplies all three targets; fixed scalar and bounds oracles predate output.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_indexed_borrowed_match_targets/emit.rs"]
mod emit;
#[path = "generic_execution_v1_indexed_borrowed_match_targets/native.rs"]
mod native;
#[path = "generic_execution_v1_indexed_borrowed_match_source_seal/negative.rs"]
mod negative;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_indexed_borrowed_match_source_seal/source.rs"]
mod source;
use std::process::Command;

fn actual(case: &str) {
    let bytes =
        include_bytes!("generic_execution_v1_indexed_borrowed_match_targets/SCALAR_EXPECTED.json");
    assert_eq!(
        emit::digest(bytes),
        "1d49c3c77c20d7a0e577852ff9d2fcf6b458b6de22757e2d563f685dbcf08db4"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(bytes).expect("pre-output source oracle");
    let expected = oracle["cases"]
        .as_array()
        .expect("three source cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same source row");
    assert_eq!(emit::digest(source::text(case).as_bytes()), expected["source_sha256"]);
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, _, wire| {
            let directory = emit::all(case, provider, program, wire, bytes);
            let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/generic_execution_v1_indexed_borrowed_match_targets/portable.mjs");
            let mut command = Command::new("node");
            command.arg(script).arg(&directory).arg(case);
            let output = process::run(command, 20);
            assert!(
                output.status.success(),
                "portable {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty());
            let actual: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("actual portable observations");
            assert_eq!(actual["case"], case);
            std::fs::write(directory.join("portable-actual.json"), &output.stdout)
                .expect("retain actual portable rows");
            native::check(&directory, expected);
            println!("indexed borrowed targets {case}/{provider}: {}", directory.display());
        });
    }
}

#[test]
fn nominal_shared_join_all_targets() {
    actual("indexed-nominal-shared-join");
}
#[test]
fn option_exclusive_join_all_targets() {
    actual("indexed-option-exclusive-join");
}
#[test]
fn chained_shared_join_all_targets() {
    actual("indexed-chain-nominal-shared-join");
}
