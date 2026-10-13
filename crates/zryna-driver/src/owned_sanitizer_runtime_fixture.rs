//! Explicit standalone ordinary fixture entry; absent from the production crate.
use std::path::Path;
/// Runs the independently admitted ordinary five-attempt fixture, with retained original receipts.
///
/// # Errors
/// Returns unavailable on any exact profile/material mismatch or observation failure.
pub fn m5_owned_sanitizer_runtime(
    parent: &Path,
) -> Result<serde_json::Value, zryna_diagnostics::Diagnostic> {
    crate::native::native_c_v0::foreign_library::m5_owned_sanitizer_runtime(parent)
}
