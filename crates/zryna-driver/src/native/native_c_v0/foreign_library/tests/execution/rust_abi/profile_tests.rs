//! Genuine source issuers and independently mutated actual data; no compiler/link/run step.

use super::capture;
use crate::native::native_c_v0::{
    foreign_library::{
        CapturedRustProfileLibrary, ForeignLibraryInput, RustProfileLibraryInput,
        capture_foreign_library, capture_rust_profile_library,
    },
    resource_identity::{HandleLinkRequirements, handle_link_requirements},
};
use sha2::{Digest as _, Sha256};

use super::object_fixture;
use object_fixture::ORIGINAL_RUST_OBJECT;
mod definition_controls;

const OBJECT_SHA: [u8; 32] = [
    0x6a, 0x77, 0xf3, 0x51, 0xde, 0x6f, 0xeb, 0xa5, 0x0f, 0x74, 0x38, 0x38, 0x54, 0x57, 0x94, 0xa1,
    0x4b, 0x83, 0x31, 0x12, 0x95, 0x24, 0x55, 0x61, 0x4e, 0x31, 0xd8, 0xbe, 0x41, 0x0d, 0x10, 0x14,
];
const SOURCE_SHA: [u8; 32] = [
    0x26, 0xc4, 0xf3, 0x29, 0x93, 0xbd, 0x1a, 0xab, 0xb1, 0x18, 0xb8, 0xa3, 0xdf, 0xe6, 0x69, 0x02,
    0xbc, 0x62, 0x38, 0x52, 0x56, 0x4f, 0x56, 0x27, 0x76, 0x21, 0x8c, 0x36, 0x67, 0x58, 0x82, 0x6b,
];

fn input<'a>(bytes: &'a [u8], digest: &'a [u8; 32]) -> RustProfileLibraryInput<'a> {
    RustProfileLibraryInput {
        library: ForeignLibraryInput {
            library_id: capture::LIBRARY_ID,
            header_bytes: capture::HEADER,
            policy_bytes: capture::POLICY,
            object_bytes: bytes,
            object_size: bytes.len(),
            object_sha256: digest,
            dependency_symbols: &[],
        },
        source_bytes: capture::FOREIGN_RUST_SOURCE.as_bytes(),
        source_size: capture::FOREIGN_RUST_SOURCE.len(),
        source_sha256: &SOURCE_SHA,
    }
}
fn scalar_requirements() -> HandleLinkRequirements {
    let original = capture::capture();
    handle_link_requirements(&capture::issue(&original).scalar_import)
        .expect("genuine original Rust-specific scalar issuer")
}
fn accepted(requirements: &HandleLinkRequirements) -> CapturedRustProfileLibrary {
    capture_rust_profile_library(requirements, &input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA))
        .expect("independently measured data profile, not a compiler grant")
}

#[test]
fn measured_rust_data_retains_original_scalar_and_byte_issuers() {
    assert_eq!(ORIGINAL_RUST_OBJECT.len(), 1064);
    assert_eq!(<[u8; 32]>::from(Sha256::digest(ORIGINAL_RUST_OBJECT)), OBJECT_SHA);
    assert_eq!(
        <[u8; 32]>::from(Sha256::digest(capture::FOREIGN_RUST_SOURCE.as_bytes())),
        SOURCE_SHA
    );
    let original = capture::capture();
    let issued = capture::issue(&original);
    for object in [&issued.scalar_import, &issued.byte_import] {
        let requirements = handle_link_requirements(object).expect("genuine original issuer");
        let captured = accepted(&requirements);
        captured.check_binding(&requirements).expect("same original capability");
        assert_eq!(captured.bytes(), ORIGINAL_RUST_OBJECT);
        assert_eq!(captured.source_bytes(), capture::FOREIGN_RUST_SOURCE.as_bytes());
        assert_eq!(captured.library_id(), capture::LIBRARY_ID);
        assert_eq!(captured.object_sha256(), &OBJECT_SHA);
        assert_eq!(captured.source_sha256(), &SOURCE_SHA);
        assert_eq!(captured.definitions().collect::<Vec<_>>(), ["add", "sum_bytes"]);
        assert_eq!(captured.requirements().object_sha256(), requirements.object_sha256());
    }
}

#[test]
fn captures_reject_reissued_and_changed_requirements_but_new_issuer_can_capture() {
    let original = capture::capture();
    let issued = capture::issue(&original);
    let scalar = handle_link_requirements(&issued.scalar_import).expect("scalar issuer");
    let bytes = handle_link_requirements(&issued.byte_import).expect("byte issuer");
    let captured = accepted(&scalar);
    assert!(captured.check_binding(&bytes).is_err());
    assert!(accepted(&bytes).check_binding(&scalar).is_err());
    let independent = scalar_requirements();
    assert_eq!(scalar.object_sha256(), independent.object_sha256());
    assert!(captured.check_binding(&independent).is_err());
    let independently_captured = accepted(&independent);
    independently_captured.check_binding(&independent).expect("new genuine issuer's own data");
    assert!(independently_captured.check_binding(&scalar).is_err());
}

#[test]
fn rust_profile_keeps_gcc_rejection_and_refuses_unneeded_pure_export_library() {
    let requirements = scalar_requirements();
    assert_eq!(
        capture_foreign_library(&requirements, &input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA).library)
            .expect_err("unchanged GCC inventory rejects Rust section family")
            .code(),
        "ZRYNA-C4104"
    );
    let original = capture::capture();
    let reverse = capture::issue(&original).reverse_export;
    let program = reverse.program();
    let entry = program
        .functions()
        .find(|function| function.name() == "add")
        .expect("original pure export")
        .entry()
        .symbol
        .clone();
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("exact native target");
    let pure = zryna_backend_native::native_c_v0::resources::emit_handle_entries(
        program,
        &[&entry],
        target,
    )
    .expect("genuine pure private entry, not fabricated export requirements");
    let pure_requirements = handle_link_requirements(&pure).expect("original pure issuer");
    assert!(pure_requirements.libraries().is_empty());
    assert_eq!(
        capture_rust_profile_library(&pure_requirements, &input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA))
            .expect_err("unneeded Rust library cannot attach to pure export")
            .code(),
        "ZRYNA-C4102"
    );
}

#[test]
fn independent_source_materials_reject_substitution_and_retain_immutable_data() {
    let requirements = scalar_requirements();
    for control in 0..8 {
        let mut altered = input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA);
        match control {
            0 => altered.source_size -= 1,
            1 => altered.source_sha256 = &[0; 32],
            2 => altered.library.library_id = "different-library@0",
            3 => altered.library.header_bytes = b"substituted header",
            4 => altered.library.policy_bytes = b"substituted policy",
            5 => altered.library.object_size -= 1,
            6 => altered.library.object_sha256 = &[0; 32],
            _ => altered.library.dependency_symbols = &["malloc"],
        }
        assert!(capture_rust_profile_library(&requirements, &altered).is_err());
    }
    let mut substituted_source = capture::FOREIGN_RUST_SOURCE.as_bytes().to_vec();
    substituted_source[0] ^= 1;
    let mut altered = input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA);
    altered.source_bytes = &substituted_source;
    assert_eq!(
        capture_rust_profile_library(&requirements, &altered)
            .expect_err("actual sourcebytes substituted with original expected SHA")
            .code(),
        "ZRYNA-C4102"
    );
    let mut invalid_utf8_source = capture::FOREIGN_RUST_SOURCE.as_bytes().to_vec();
    invalid_utf8_source[0] = 0xff;
    let invalid_utf8_digest: [u8; 32] = Sha256::digest(&invalid_utf8_source).into();
    let mut utf8_control = input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA);
    utf8_control.source_bytes = &invalid_utf8_source;
    utf8_control.source_sha256 = &invalid_utf8_digest;
    assert_eq!(
        capture_rust_profile_library(&requirements, &utf8_control)
            .expect_err("independently pinned invalid UTF8 data cannot be retained as Rust source")
            .code(),
        "ZRYNA-C4102"
    );
    let mut bytes = ORIGINAL_RUST_OBJECT.to_vec();
    let mut source = capture::FOREIGN_RUST_SOURCE.as_bytes().to_vec();
    let mut data = input(&bytes, &OBJECT_SHA);
    data.source_bytes = &source;
    let captured = capture_rust_profile_library(&requirements, &data).expect("original data");
    bytes.fill(0);
    source.fill(0);
    assert_eq!(captured.bytes(), ORIGINAL_RUST_OBJECT);
    assert_eq!(captured.source_bytes(), capture::FOREIGN_RUST_SOURCE.as_bytes());
    captured.check_binding(&requirements).expect("buffer replacement cannot alter snapshot");
}

#[test]
fn oversized_foreign_source_or_object_rejects_before_bad_identity_hashes() {
    let requirements = scalar_requirements();
    let oversized = vec![b'x'; zryna_backend_native::MAX_NATIVE_OBJECT_BYTES + 1];
    let mut source = input(ORIGINAL_RUST_OBJECT, &OBJECT_SHA);
    source.source_bytes = &oversized;
    source.source_size = oversized.len();
    source.source_sha256 = &[0; 32];
    assert_eq!(
        capture_rust_profile_library(&requirements, &source)
            .expect_err("prehash/precopy source bound")
            .code(),
        "ZRYNA-C4105"
    );
    let object = input(&oversized, &[0; 32]);
    assert_eq!(
        capture_rust_profile_library(&requirements, &object)
            .expect_err("prehash/precopy artifact bound")
            .code(),
        "ZRYNA-C4105"
    );
    accepted(&requirements);
}

#[test]
fn malformed_original_tables_symbols_and_metadata_reject_then_recover() {
    let requirements = scalar_requirements();
    // Independently decoded original offsets; mutant hashes isolate the structural guard.
    let controls = [
        (7, 3),
        (18, 183),
        (56, 1),
        (62, 2),
        (552, 1),
        (552 + 3 * 64 + 8, 3),
        (552 + 7 * 64 + 40, 2),
        (552 + 7 * 64 + 44, 1),
        (552 + 7 * 64 + 56, 23),
        (336, 1),
        (336 + 24 + 4, 0x12),
        (336 + 48 + 4, 0x22),
        (336 + 48 + 5, 2),
        (336 + 48 + 6, 4),
        (336 + 48 + 16, 0),
        (552 + 2 * 64 + 4, 4),
        (545, b'X'),
        (287 + 7, 0),
    ];
    for (offset, value) in controls {
        let mut bytes = ORIGINAL_RUST_OBJECT.to_vec();
        bytes[offset] = value;
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        assert_eq!(
            capture_rust_profile_library(&requirements, &input(&bytes, &digest))
                .expect_err("independently supplied hostile shape")
                .code(),
            "ZRYNA-C4104",
            "offset {offset}"
        );
        accepted(&requirements);
    }
    let mut trailing = ORIGINAL_RUST_OBJECT.to_vec();
    trailing.push(0);
    let digest: [u8; 32] = Sha256::digest(&trailing).into();
    assert!(capture_rust_profile_library(&requirements, &input(&trailing, &digest)).is_err());
}
