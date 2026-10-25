//! Genuine indexed native execution with canonical17 observations and unchanged source-derived oracles.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_indexed_continuations_source_seal/source.rs"]
mod continuations;
#[path = "generic_execution_v1_native_indexed_runtime/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_indexed_runtime/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_indexed_runtime/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_indexed_runtime/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
#[path = "generic_execution_v1_native_indexed_runtime/source_controls.rs"]
mod source_controls;
fn run(case: &str) {
    let additional = matches!(case, "called-indexed-shared-copy" | "indexed-push-copy");
    let text = if additional { continuations::text(case) } else { source::text(case) };
    assert_eq!(emit::digest(text.as_bytes()), oracle::case(case)["sourceSha256"]);
    for provider in source::PROVIDERS {
        let check = |program: &zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>,
                     _: &zryna_ir::generic_v1::execution_v1::raw::Program,
                     wire: &[u8]| {
            let artifact = emit::native(case, program, wire);
            let (observations, traps, faults) = execute::all(case, &artifact);
            let expected = oracle::case(case);
            let normal = expected["normal"].as_array().expect("frozen rows");
            let fault_rows = expected["faults"].as_array().expect("frozen faults");
            assert_eq!(faults, fault_rows.len());
            assert_eq!(observations, 2 * normal.len() + 2 * faults);
            assert_eq!(
                traps,
                2 * normal.iter().filter(|row| row.get("trap").is_some()).count() + faults
            );
            println!("native indexed {case}/{provider}: {}", artifact.directory.display());
        };
        if additional {
            continuations::with_program(case, provider, check);
        } else {
            source::with_program(case, provider, check);
        }
    }
}
#[test]
fn copy_read_control() {
    run("copy-read-control");
}
#[test]
fn array_shared_copy() {
    run("array-shared-copy");
}
#[test]
fn vec_shared_clone() {
    run("vec-shared-clone");
}
#[test]
fn array_exclusive_copy() {
    run("array-exclusive-copy");
}
#[test]
fn vec_exclusive_replace() {
    run("vec-exclusive-replace");
}
#[test]
fn vec_indexed_replace() {
    run("vec-indexed-replace");
}
#[test]
fn array_indexed_rhs_match() {
    run("array-indexed-rhs-match");
}
#[test]
fn index_once_before_rhs() {
    run("index-once-before-rhs");
}
#[test]
fn chained_shared_copy() {
    run("chained-shared-copy");
}
#[test]
fn zero_length_bounds() {
    run("zero-length-bounds");
}
#[test]
fn called_indexed_shared_copy() {
    run("called-indexed-shared-copy");
}
#[test]
fn indexed_push_copy() {
    run("indexed-push-copy");
}
#[test]
fn immutable_finite_oracle_census() {
    let mut normal = 0;
    let mut faults = 0;
    let mut bounds = 0;
    for case in [
        "copy-read-control",
        "array-shared-copy",
        "vec-shared-clone",
        "array-exclusive-copy",
        "vec-exclusive-replace",
        "vec-indexed-replace",
        "array-indexed-rhs-match",
        "index-once-before-rhs",
        "chained-shared-copy",
        "zero-length-bounds",
        "called-indexed-shared-copy",
        "indexed-push-copy",
    ] {
        let expected = oracle::case(case);
        let rows = expected["normal"].as_array().expect("normal");
        normal += rows.len();
        faults += expected["faults"].as_array().expect("faults").len();
        bounds += rows.iter().filter(|row| row.get("trap").is_some()).count();
    }
    assert_eq!((normal, faults, bounds), (54, 70, 28));
    assert_eq!(
        2 * (2 * normal + 2 * faults),
        496,
        "both providers with repeated normals and each fault/retry"
    );
    assert_eq!(
        2 * (2 * bounds + faults),
        252,
        "native SIGILLs include bounds and controlled allocation faults"
    );
}
