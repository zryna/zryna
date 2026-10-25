//! Genuine source -> NEW whole seal -> native lower/ELF; no raw proof adapters.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_indexed_continuations_source_seal/source.rs"]
mod continuations;
#[path = "generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
fn object(program: &VerifiedExecution<'_>, wire: &[u8]) {
    assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
    let mir = zryna_native_mir::generic_execution_v1::lower(program)
        .expect("genuine admitted indexed loans must lower; N7002 is owning behavior RED");
    assert!(std::ptr::eq(mir.program(), program));
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified existing Linux target");
    let artifact = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("actual prepared indexed addresses/bounds/cleanup enter native ELF");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(artifact.bytes(), &mir)
        .expect("independent exact new-seal runtime/helper/source-call inventory");
    assert!(!artifact.bytes().is_empty());
}
fn native(case: &str) {
    assert!(source::text(case).contains("export function entry("));
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, _, wire| object(program, wire));
    }
}
#[test]
fn copy_read_control() {
    native("copy-read-control");
}
#[test]
fn array_shared_copy() {
    native("array-shared-copy");
}
#[test]
fn vec_shared_clone() {
    native("vec-shared-clone");
}
#[test]
fn array_exclusive_copy() {
    native("array-exclusive-copy");
}
#[test]
fn vec_exclusive_replace() {
    native("vec-exclusive-replace");
}
#[test]
fn vec_indexed_replace() {
    native("vec-indexed-replace");
}
#[test]
fn array_indexed_rhs_match() {
    native("array-indexed-rhs-match");
}
#[test]
fn index_once_before_rhs() {
    native("index-once-before-rhs");
}
#[test]
fn chained_shared_copy() {
    native("chained-shared-copy");
}
#[test]
fn zero_length_bounds() {
    native("zero-length-bounds");
}
#[test]
fn called_indexed_shared_copy() {
    assert!(continuations::text("called-indexed-shared-copy").contains("export function entry("));
    for provider in continuations::PROVIDERS {
        continuations::with_program("called-indexed-shared-copy", provider, |program, _, wire| {
            object(program, wire);
        });
    }
}
#[test]
fn indexed_push_copy() {
    assert!(continuations::text("indexed-push-copy").contains("export function entry("));
    for provider in continuations::PROVIDERS {
        continuations::with_program("indexed-push-copy", provider, |program, _, wire| {
            object(program, wire);
        });
    }
}

fn rejection(
    sources: &zryna_source::SourceMap,
    syntax: &zryna_syntax::v5::VerifiedProjectSyntaxV5,
    row: &serde_json::Value,
) {
    let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
        zryna_ir::generic_v1::execution_v1::check_original_bodies(syntax, sources)
            .expect_err("source permission must fail before native authority")
    else {
        panic!("owning source diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), row["code"].as_str().expect("frozen diagnostic"));
    let span = sources
        .verify_span(zryna_source::UntrustedSpan {
            file: 0,
            start: u32::try_from(row["primary"]["start"].as_u64().expect("start"))
                .expect("bounded start"),
            end: u32::try_from(row["primary"]["end"].as_u64().expect("end")).expect("bounded end"),
        })
        .expect("source-owned primary span");
    assert_eq!(errors[0].primary_span(), Some(span));
}
#[test]
fn indexed_source_hostiles_still_fail_before_native_permission() {
    for case in [
        "dynamic-array-overlap",
        "vector-constant-overlap",
        "shared-element-write",
        "owner-move-while-indexed",
        "opaque-indexed-consume",
        "indexed-rhs-moves-owner",
    ] {
        let expected = source::diagnostic(case);
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            rejection(&sources, &syntax, &expected);
        }
    }
    let case = "indexed-push-rhs-relocates-owner";
    let expected = continuations::diagnostic(case);
    for provider in continuations::PROVIDERS {
        let (sources, syntax) = continuations::project(case, provider);
        rejection(&sources, &syntax, &expected);
    }
}
