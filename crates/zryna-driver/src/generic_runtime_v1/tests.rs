//! Genuine source owners exercise the production issuer without a linker or host process.

use super::*;
use crate::generic_recursive_source as source;
use sha2::{Digest as _, Sha256};

fn object(mir: &VerifiedProgram<'_, '_>) -> ValidatedNativeObjectArtifact {
    zryna_backend_native::generic_execution_v1::emit_object(
        mir,
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("documented cross-host native object target"),
    )
    .expect("same NEW-owner audited ordinary object")
}

#[test]
fn generic_runtime_source_requires_exact_new_mir_and_audited_ordinary_object() {
    for case in ["erased", "clone", "mixed"] {
        source::with_program(case, |program| {
            let mir = zryna_native_mir::generic_execution_v1::lower(program)
                .expect("genuine NEW native MIR");
            let ordinary = object(&mir);
            let source = RuntimeSource::issue(program, &mir, &ordinary)
                .expect("ordinary object material needs no host or linker");
            assert!(std::ptr::eq(source.program(), program));
            assert!(std::ptr::eq(source.mir(), &raw const mir));
            let identity = source.identity();
            assert_eq!(
                *identity.rendered_sha256(),
                <[u8; 32]>::from(Sha256::digest(source.bytes()))
            );
            assert_eq!(
                *identity.ordinary_object_sha256(),
                <[u8; 32]>::from(Sha256::digest(ordinary.bytes()))
            );
            assert_eq!(identity.target(), zryna_backend_native::NATIVE_OBJECT_TARGET);
            assert_eq!(
                identity.required_tls_bytes(),
                if mir.requires_recursive_arena() { 33_554_448 } else { 0 }
            );
            assert_eq!(identity.template_sha256(), &super::source::template_sha256());
            assert_eq!(
                *identity.header_sha256(),
                <[u8; 32]>::from(Sha256::digest(super::source::HEADER))
            );
            let linear = program.layouts(StorageTarget::Linear32V1);
            let linux = program.layouts(StorageTarget::LinuxX8664V1);
            assert_eq!(identity.universe_identity(), linux.universe_identity());
            assert_eq!(identity.linear_layout_fingerprint(), linear.fingerprint());
            assert_eq!(identity.native_layout_fingerprint(), linux.fingerprint());
            assert_eq!(
                *identity.runtime_abi_fingerprint(),
                program.runtime().identity().fingerprint()
            );
            assert!(
                !String::from_utf8_lossy(source.bytes())
                    .contains("/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */")
            );
            source::with_program(case, |foreign| {
                let other_mir = zryna_native_mir::generic_execution_v1::lower(foreign)
                    .expect("separate actual same-text owner");
                assert!(RuntimeSource::issue(program, &other_mir, &ordinary).is_err());
            });
        });
    }
}

#[test]
fn legacy_runtime_material_remains_the_exact_canonical_input() {
    assert_eq!(
        format!("{:x}", Sha256::digest(crate::ownership_runtime_v1::SOURCE)),
        "94ce0a5f0d40458a7dcc234b93f98a2d18c985ff1f60e285c238516bea3d0e26"
    );
    assert_eq!(crate::ownership_runtime_v1::SOURCE.len(), 17_461);
}
