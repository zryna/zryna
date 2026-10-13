//! Rust-specific fixture source issuers only; no Rust object, link or runtime admission.

use object::{Architecture, BinaryFormat, Object as _, ObjectKind};
use sha2::{Digest as _, Sha256};

mod capture;
mod object_fixture;
mod ordinary;
mod profile_tests;

#[test]
fn rust_fixture_documents_feed_genuine_source_machine_and_backend_issuers() {
    assert_eq!(capture::FOREIGN_RUST_SOURCE.len(), 1696);
    assert_eq!(
        format!("{:x}", Sha256::digest(capture::FOREIGN_RUST_SOURCE.as_bytes())),
        "26c4f32993bd1aabb118b8a3dfe66902bc623852564f562776218c366758826b",
        "exact independently authored foreign program bytes, not workspace unsafe code"
    );
    let original = capture::capture();
    let issued = capture::issue(&original);
    for (bytes, program) in [
        (issued.scalar_import.bytes(), issued.scalar_import.program()),
        (issued.byte_import.bytes(), issued.byte_import.program()),
        (issued.reverse_export.bytes(), issued.reverse_export.program()),
    ] {
        assert!(program.source().belongs_to(&original.sources));
        assert_eq!(program.target(), "x86_64-unknown-linux-gnu");
        let object =
            object::File::parse(bytes).expect("actual audited Zryna backend fixture object");
        assert_eq!(object.format(), BinaryFormat::Elf);
        assert_eq!(object.architecture(), Architecture::X86_64);
        assert_eq!(object.kind(), ObjectKind::Relocatable);
        assert!(object.is_little_endian());
        let imports = program
            .operations()
            .filter_map(|operation| {
                let declaration = operation.declaration();
                (declaration.library == capture::LIBRARY_ID).then_some(declaration.symbol.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            imports,
            ["add", "sum_bytes"],
            "both original foreign definitions remain required"
        );
    }
    let scalar = issued
        .scalar_import
        .imported_operations()
        .map(|operation| operation.declaration().symbol.as_str())
        .collect::<Vec<_>>();
    let bytes = issued
        .byte_import
        .imported_operations()
        .map(|operation| operation.declaration().symbol.as_str())
        .collect::<Vec<_>>();
    assert_eq!(scalar, ["add"]);
    assert_eq!(bytes, ["sum_bytes"]);
    assert_eq!(issued.scalar_import.entries().len(), 1);
    assert_eq!(issued.byte_import.entries().len(), 1);
    assert!(issued.reverse_export.header().contains("zryna_c_v0_e_add"));
    assert!(!issued.reverse_export.header().contains("sum_bytes"));
}

#[test]
fn independently_recaptured_equal_documents_do_not_replace_original_issuers() {
    let original = capture::capture();
    let issued = capture::issue(&original);
    let independent = capture::capture();
    for program in [
        issued.scalar_import.program(),
        issued.byte_import.program(),
        issued.reverse_export.program(),
    ] {
        assert!(program.source().belongs_to(&original.sources));
        assert!(!program.source().belongs_to(&independent.sources));
    }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
pub(super) fn m5_owned_sanitizer_runtime(
    parent: &std::path::Path,
) -> Result<serde_json::Value, zryna_diagnostics::Diagnostic> {
    ordinary::m5_owned_sanitizer_runtime(parent)
}
