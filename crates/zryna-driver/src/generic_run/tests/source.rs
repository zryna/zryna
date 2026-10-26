//! Unchanged genuine scalar14 original, current native syntax and independent scoped NEW issuer.
use crate::{
    CommandFailure,
    generic_closure::{
        self,
        test_source::{CurrentSyntax, MemoryRoot},
    },
    source_scope::{self, GenericSourceReceipt},
};
use sha2::{Digest as _, Sha256};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn with<R>(
    inspect: impl FnOnce(
        &VerifiedExecution<'_>,
        &GenericSourceReceipt,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> R,
) -> R {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/m7-generic-full-profile-first-vertical/literals/scalar-i32-clone.zry"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        "81336e343ddb166fbeea0fb3fa3d882b4ed93766fb50b540504a3840ca999c81"
    );
    let root = MemoryRoot::new([("main.zry".into(), text.into())]);
    generic_closure::with_closure(
        &root,
        zryna_source::NormalizedSourcePath::new("main.zry").expect("entry"),
        &CurrentSyntax::ordinary(),
        &|| Ok(()),
        |closure, checkpoint| {
            source_scope::with_execution(closure, checkpoint, |program, receipt, checkpoint| {
                Ok(inspect(program, receipt, checkpoint))
            })
        },
    )
    .expect("original source session and independent NEW execution")
}
