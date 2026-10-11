//! Shared CLI path capture and diagnostic helpers.

use std::path::PathBuf;
use zryna_diagnostics::Diagnostic;

use crate::profile;

pub(super) fn absolute_workspace_path(path: &PathBuf) -> Result<PathBuf, Diagnostic> {
    if path.is_absolute() {
        Ok(path.clone())
    } else {
        Ok(std::env::current_dir().map_err(|_| cli_path_error())?.join(path))
    }
}

pub(super) fn cli_path_error() -> Diagnostic {
    profile::cli_path_error()
}
