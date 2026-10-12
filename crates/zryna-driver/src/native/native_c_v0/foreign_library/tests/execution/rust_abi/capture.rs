//! Draft library-specific source-to-machine issuers. No foreign object is accepted here.

use zryna_backend_native::native_c_v0::{
    ValidatedScalarExports,
    resources::{ValidatedHandleEntries, emit_byte_entries, emit_handle_entries},
};
use zryna_native_mir::native_c_v0::VerifiedMirProgram;
use zryna_semantics::native_c_v0::{
    LibraryMaterial,
    body::{VerifiedPrivateBoundaries, compose_private_boundaries, verify_bodies},
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::native_c_source_v0::authenticate_sources;

pub(super) const LIBRARY_ID: &str = "fixture-rust-c-v0@0";

include!("../../../../../../../../../tests/native-c-rust-v0/library.rs");
pub(super) const HEADER: &[u8] =
    include_bytes!("../../../../../../../../../tests/native-c-rust-v0/library.h");
pub(super) const POLICY: &[u8] =
    include_bytes!("../../../../../../../../../tests/native-c-rust-v0/library-policy.json");
const DECLARATIONS: &[u8] =
    include_bytes!("../../../../../../../../../tests/native-c-rust-v0/declarations.ffi.json");
const SCALAR: &str =
    include_str!("../../../../../../../../../tests/native-c-rust-v0/source-scalar.zry");
const BUFFER: &str =
    include_str!("../../../../../../../../../tests/native-c-rust-v0/source-buffer.zry");

/// Retains actual source and original private authorities, never object-derived claims.
pub(super) struct Capture {
    pub(super) sources: SourceMap,
    pub(super) authority: VerifiedPrivateBoundaries,
}

/// This unexecuted draft performs genuine syntax, declarations and body admission.
pub(super) fn capture() -> Capture {
    let sources = SourceMap::build(vec![
        SourceFileInput {
            path: "tests/native-c-rust-v0/source-buffer.zry".into(),
            text: BUFFER.into(),
        },
        SourceFileInput {
            path: "tests/native-c-rust-v0/source-scalar.zry".into(),
            text: SCALAR.into(),
        },
    ])
    .expect("original Rust-specific fixture source map");
    let syntax = authenticate_sources(&sources).expect("original Rust-specific syntax sites");
    let declarations = zryna_semantics::native_c_v0::verify(
        DECLARATIONS,
        &sources,
        &syntax,
        &[LibraryMaterial { library_id: LIBRARY_ID, header_bytes: HEADER, policy_bytes: POLICY }],
        zryna_syntax::native_c_v0::TARGET,
    )
    .expect("original Rust-specific declarations and materials");
    let bodies = verify_bodies(&sources, &declarations).expect("original complete fixture bodies");
    let authority = compose_private_boundaries(&sources, &bodies)
        .expect("original private language and runtime issuers");
    Capture { sources, authority }
}

/// Actual emission is UNRUN; all three results must retain this original source authority.
pub(super) struct IssuedArtifacts {
    pub(super) scalar_import: ValidatedHandleEntries,
    pub(super) byte_import: ValidatedHandleEntries,
    pub(super) reverse_export: ValidatedScalarExports,
}

fn machine(captured: &Capture) -> VerifiedMirProgram {
    let ir = zryna_native_c_ir::lower(&captured.sources, &captured.authority)
        .expect("genuine Rust-specific original IR issuer");
    zryna_native_mir::native_c_v0::lower(&ir)
        .expect("genuine Rust-specific original machine issuer")
}

fn entry(machine: &VerifiedMirProgram, name: &str) -> String {
    machine
        .functions()
        .find(|function| function.name() == name)
        .expect("exact original fixture function")
        .entry()
        .symbol
        .clone()
}

pub(super) fn issue(captured: &Capture) -> IssuedArtifacts {
    let machine = machine(captured);
    let scalar = entry(&machine, "imported");
    let bytes = entry(&machine, "sum");
    let target = || {
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("exact accepted Linux x86-64 target")
    };
    let scalar_import = emit_handle_entries(&machine, &[&scalar], target())
        .expect("genuine audited scalar-import entry");
    let byte_import = emit_byte_entries(&machine, &[&bytes], target())
        .expect("genuine audited borrowed-byte entry");
    let reverse_export = zryna_backend_native::native_c_v0::emit_scalar_exports(&machine, target())
        .expect("genuine audited pure reverse scalar export");
    IssuedArtifacts { scalar_import, byte_import, reverse_export }
}
