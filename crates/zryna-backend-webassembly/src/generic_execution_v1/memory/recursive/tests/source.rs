//! Genuine frozen TypeScript DTOs undergo all independent original/new-execution issuers.
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::{self, VerifiedExecution};
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1 as semantic, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn pinned(name: &str, expected: &str) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-recursive-owner");
    let bytes = std::fs::read(root.join(name)).expect("actual frozen source/provider input");
    assert_eq!(digest(&bytes), expected);
    bytes
}
pub(super) fn with_program<T>(owning: bool, run: impl FnOnce(&VerifiedExecution<'_>) -> T) -> T {
    let (text, packet) = if owning {
        (
            pinned(
                "literals/drop-only.zry",
                "3d097ad3baedcc0f192bac0f5681bd2b0f92ec94d7091d9d30dcd260d96de8dd",
            ),
            pinned(
                "typescript/drop-only.json",
                "7727781a1fc16115e9da0b89a13bbda777e80cfac362587ccc6b3da47ac9f0e6",
            ),
        )
    } else {
        pinned(
            "memory/ERASED_EXPECTED_FREEZE.json",
            "e04d5f0e2f7fbc61c1522d73c4576a6653e953a61702c960eeffd2f679054c8b",
        );
        // Static authority correction was frozen before first semantic/memory execution;
        // retain the original wording as history rather than asserting an incorrect Copy rule.
        pinned(
            "memory/ERASED_EXPECTATION_STATIC_CORRECTION.json",
            "c8a9570fb03f6db22631e8ad49055ee70472cab6acf7c610ad06727169eb6dbc",
        );
        (
            pinned(
                "memory/erased.zry",
                "245a9558deb2a770cc130b8f67a6a11f67f0d034e934d0297556f5ba39b94d86",
            ),
            pinned(
                "memory/erased.typescript.json",
                "9940e343e0031a3adf13a351bd10a0c5e832e2ae3c89c2d83e4590b617f63532",
            ),
        )
    };
    issue(text, &packet, run)
}
pub(super) fn with_clone<T>(run: impl FnOnce(&VerifiedExecution<'_>) -> T) -> T {
    issue(
        pinned(
            "literals/clone.zry",
            "30b5f845909ccac7e6f9276a078594570b50cb1fc8ae7533658e9a568bb87f21",
        ),
        &pinned(
            "typescript/clone.json",
            "e65cd5fcc52a1b2f4591785f5846d16d89ffe013d85db8b742f79d933f27c546",
        ),
        run,
    )
}
pub(super) fn mixed_oracle() -> serde_json::Value {
    serde_json::from_slice(&pinned(
        "mixed/PRE_EXECUTION_ORACLE.json",
        "bc6fdcf4d69acbd15cd5585b22bdd1a1d193be794c73177e03268d59902f4266",
    ))
    .expect("unchanged pre-execution source oracle")
}
pub(super) fn with_mixed<T>(run: impl FnOnce(&VerifiedExecution<'_>) -> T) -> T {
    issue(
        pinned(
            "mixed/mixed-catalogue.zry",
            "5d28cef948a289b7e808b5efe8a94c4c4d7f73ef5d47896195e1e2fc6077f8b6",
        ),
        &pinned(
            "mixed/mixed.typescript.json",
            "918cd89ca415c91acbf589cb6a52aacbd113facdb2e24febd48a4ffe2fbea9b5",
        ),
        run,
    )
}
fn issue<T>(text: Vec<u8>, packet: &[u8], run: impl FnOnce(&VerifiedExecution<'_>) -> T) -> T {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("original UTF8"),
    }])
    .expect("original source map");
    let syntax =
        v5::verify_snapshot(v5::decode_snapshot(packet).expect("actual strict DTO"), &sources)
            .expect("exact original syntax owner");
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
    )
    .expect("original declarations");
    let bodies =
        check_execution_body_types(&declarations).expect("actual accepted execution bodies");
    let instances = semantic::discover(&bodies).expect("original closed instances");
    let linear =
        verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear layouts");
    let native =
        verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("native layouts");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &native).expect("runtime candidate"),
        &linear,
        &native,
    )
    .expect("same-universe runtime");
    let claim = semantic::produce_claim(&instances, &linear, &native)
        .expect("original source execution claim");
    let wire = execution_v1::wire::encode(&claim).expect("distinct execution carrier");
    let program = execution_v1::verify(
        execution_v1::wire::decode(&wire).expect("strict execution carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &native,
        &runtime,
    )
    .expect("independent new whole execution issuer");
    run(&program)
}
