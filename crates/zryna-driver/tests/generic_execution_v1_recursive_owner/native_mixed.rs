//! Actual Native mixed VM witness using the original same-owner audited object and frozen oracle.
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
#[path = "native_mixed/calls.rs"]
mod calls;
#[path = "native_mixed/emit.rs"]
mod emit;
#[path = "native_mixed/execute.rs"]
mod execute;
#[path = "native_mixed/expected.rs"]
mod expected;
#[path = "../generic_execution_v1_native_containers/metadata.rs"]
mod metadata;
#[path = "native_mixed/projection.rs"]
mod projection;
#[path = "native_mixed/verify.rs"]
mod verify;
pub(super) fn run(
    program: &VerifiedExecution<'_>,
    mir: &VerifiedProgram<'_, '_>,
    object: &[u8],
    directory: &Path,
) {
    assert!(std::ptr::eq(program, mir.program()));
    assert!(mir.requires_recursive_arena());
    let original = super::pinned(
        "PRE_EXECUTION_ORACLE.json",
        "bc6fdcf4d69acbd15cd5585b22bdd1a1d193be794c73177e03268d59902f4266",
    );
    let oracle = serde_json::from_slice(&original).expect("original complete mixed oracle");
    let addendum = expected::load();
    emit::prepare(program, mir, object, &original, &oracle, directory);
    execute::all(directory, &oracle, &addendum);
    assert_eq!(std::fs::read(directory.join("program.o")).expect("original object"), object);
}
