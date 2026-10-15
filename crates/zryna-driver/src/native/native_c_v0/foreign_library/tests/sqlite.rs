//! Distinct SQLite source/declaration and ownership issuers; no library execution grant.

use super::*;
use zryna_semantics::native_c_v0::{
    LibraryMaterial,
    body::{VerifiedPrivateBoundaries, compose_private_boundaries, verify_bodies},
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::native_c_source_v0::authenticate_sources;

const ID: &str = "fixture-sqlite-c-v0@0";
const SOURCE_PATH: &str = "tests/native-c-sqlite-v0/source-buffer.zry";
const SOURCE: &str =
    include_str!("../../../../../../../tests/native-c-sqlite-v0/source-buffer.zry");
const HEADER: &[u8] = include_bytes!("../../../../../../../tests/native-c-sqlite-v0/library.h");
const POLICY: &[u8] =
    include_bytes!("../../../../../../../tests/native-c-sqlite-v0/library-policy.json");
const DECLARATIONS: &[u8] =
    include_bytes!("../../../../../../../tests/native-c-sqlite-v0/declarations.ffi.json");

fn sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: SOURCE_PATH.into(), text: SOURCE.into() }])
        .expect("independent complete SQLite source capture")
}

fn capture() -> (SourceMap, VerifiedPrivateBoundaries) {
    let sources = sources();
    let syntax = authenticate_sources(&sources).expect("actual SQLite source syntax");
    let declarations = zryna_semantics::native_c_v0::verify(
        DECLARATIONS,
        &sources,
        &syntax,
        &[LibraryMaterial { library_id: ID, header_bytes: HEADER, policy_bytes: POLICY }],
        zryna_syntax::native_c_v0::TARGET,
    )
    .expect("fixed independent SQLite declarations, header and policy");
    let bodies = verify_bodies(&sources, &declarations).expect("complete source body replay");
    let authority = compose_private_boundaries(&sources, &bodies)
        .expect("original private ownership and layout issuer");
    (sources, authority)
}

fn requirements(
    sources: &SourceMap,
    authority: &VerifiedPrivateBoundaries,
) -> HandleLinkRequirements {
    let ir = zryna_native_c_ir::lower(sources, authority).expect("original source-bound IR");
    let machine = zryna_native_mir::native_c_v0::lower(&ir).expect("independent machine verifier");
    let symbol = machine
        .functions()
        .find(|f| f.name() == "copied")
        .expect("copy entry")
        .entry()
        .symbol
        .clone();
    let object = zryna_backend_native::native_c_v0::resources::emit_byte_entries(
        &machine,
        &[&symbol],
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("exact Linux target"),
    )
    .expect("independently audited SQLite byte entry");
    super::super::super::resource_identity::handle_link_requirements(&object)
        .expect("exact original link requirements")
}

fn wire(mut value: serde_json::Value) -> Vec<u8> {
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec(&value).expect("canonical hostile input");
    bytes.push(b'\n');
    bytes
}

#[test]
fn sqlite_source_ownership_contract_reaches_original_ir_machine_and_object() {
    let (sources, authority) = capture();
    let requirements = requirements(&sources, &authority);
    assert!(requirements.object().program().source().belongs_to(&sources));
    assert_eq!(requirements.libraries().len(), 1);
    assert_eq!(requirements.libraries()[0].id(), ID);
    assert_eq!(requirements.libraries()[0].header_sha256(), &sha(HEADER));
    assert_eq!(requirements.libraries()[0].policy_sha256(), &sha(POLICY));
    assert_eq!(
        requirements
            .object()
            .imported_operations()
            .map(|op| op.declaration().symbol.as_str())
            .collect::<Vec<_>>(),
        ["sqlite_fixture_copy_bytes", "sqlite_fixture_release_bytes"]
    );
    assert!(requirements.private_runtime_source().is_some());
}

#[test]
fn sqlite_equal_source_bytes_cannot_replace_the_original_issuer() {
    let (first_sources, first) = capture();
    let (second_sources, second) = capture();
    assert!(zryna_native_c_ir::lower(&second_sources, &first).is_err());
    assert!(zryna_native_c_ir::lower(&first_sources, &second).is_err());
    let first = requirements(&first_sources, &first);
    let second = requirements(&second_sources, &second);
    assert_eq!(first.object_sha256(), second.object_sha256());
    assert!(!first.object().program().source().belongs_to(&second_sources));
}

#[test]
fn sqlite_material_and_target_substitution_reject_before_a_foreign_call() {
    let sources = sources();
    let syntax = authenticate_sources(&sources).expect("original syntax");
    for (id, header, policy, target) in [
        ("fixture-c-v0@0", HEADER, POLICY, zryna_syntax::native_c_v0::TARGET),
        (ID, &b"substituted same-name header"[..], POLICY, zryna_syntax::native_c_v0::TARGET),
        (ID, HEADER, &b"{}\n"[..], zryna_syntax::native_c_v0::TARGET),
        (ID, HEADER, POLICY, "wasm32-unknown-unknown"),
    ] {
        assert!(
            zryna_semantics::native_c_v0::verify(
                DECLARATIONS,
                &sources,
                &syntax,
                &[LibraryMaterial { library_id: id, header_bytes: header, policy_bytes: policy }],
                target,
            )
            .is_err(),
            "substituted SQLite material or target must reject"
        );
    }
    capture();
}

#[test]
fn sqlite_independent_policy_rejects_allocator_release_and_abi_substitutions() {
    let sources = sources();
    let syntax = authenticate_sources(&sources).expect("original syntax");
    let original: serde_json::Value = serde_json::from_slice(DECLARATIONS).expect("fixed sidecar");
    for (pointer, value) in [
        (
            "/operations/0/resources/1/allocator",
            serde_json::json!("fixture-c-v0@0/fixture_copy_bytes"),
        ),
        (
            "/operations/0/resources/1/release",
            serde_json::json!("fixture-c-v0@0/fixture_release_bytes"),
        ),
        ("/operations/0/resources/1/kind", serde_json::json!("fixture-c-v0@0/owned_bytes")),
        ("/operations/0/resources/0/maxBytes", serde_json::json!(4097)),
        ("/operations/0/execution", serde_json::json!("callback")),
        ("/operations/0/parameters/1/abi", serde_json::json!("c-i32")),
        ("/operations/1/resources/0/allocator", serde_json::json!("fixture-c-v0@0/fixture_open")),
        ("/operations/1/result", serde_json::json!("c-i32")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).expect("independent exact mutation field") = value;
        assert!(
            zryna_semantics::native_c_v0::verify(
                &wire(changed),
                &sources,
                &syntax,
                &[LibraryMaterial { library_id: ID, header_bytes: HEADER, policy_bytes: POLICY }],
                zryna_syntax::native_c_v0::TARGET,
            )
            .is_err(),
            "fixed reviewed policy must reject {pointer}"
        );
    }
    capture();
}
