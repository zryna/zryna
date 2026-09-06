//! Native private-stage identity and stable diagnostics.

use std::fs;

use zryna_diagnostics::Diagnostic;

use super::native_error;

#[derive(Clone, Copy)]
pub(super) struct NativeStageIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
}

pub(super) fn native_stage_identity(
    metadata: &fs::Metadata,
) -> Result<NativeStageIdentity, Diagnostic> {
    use std::os::unix::fs::MetadataExt;

    if !metadata.is_dir() {
        return Err(native_stage_error());
    }
    Ok(NativeStageIdentity { device: metadata.dev(), inode: metadata.ino() })
}

pub(super) fn native_stage_error() -> Diagnostic {
    native_error(
        "ZRYNA-N4015",
        "native private staging directory identity changed during the operation",
        "retry without another process modifying the compiler-owned staging directory",
    )
}

pub(super) fn stage_cleanup_warning() -> Diagnostic {
    Diagnostic::warning(
        "ZRYNA-N4016",
        None,
        "native operation finished but its private staging directory could not be fully removed",
        "inspect and remove the exact sibling .zryna-link staging directory after confirming no operation is using it",
    )
}

pub(super) fn staging_write_error() -> Diagnostic {
    native_error(
        "ZRYNA-N4015",
        "native private staging input could not be written and synchronized",
        "use a writable declared output root with sufficient space",
    )
}
