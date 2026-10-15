//! Original declaration/source issuer versus the retained resolved package subset.
use super::{CompilePlanRequest, NativeRecipeError, denied};

pub(super) fn check(request: &CompilePlanRequest<'_>) -> Result<(), NativeRecipeError> {
    if !request.requirements.object().program().source().belongs_to(request.sources)
        || request.requirements.object().program().source().target() != "x86_64-unknown-linux-gnu"
        || request.requirements.libraries().len() != 1
    {
        return Err(denied(
            "TRUST-NATIVE",
            "original single-library Linux FFI source authority is required",
        ));
    }
    let package = request
        .resolution
        .sources()
        .iter()
        .find(|package| package.package_id() == request.ffi_package_id)
        .ok_or_else(|| denied("TRUST-SOURCE", "FFI package is absent from the retained graph"))?;
    let zryna = package
        .files()
        .iter()
        .filter(|file| {
            file.path().rsplit_once('.').is_some_and(|(_, extension)| extension == "zry")
        })
        .collect::<Vec<_>>();
    if zryna.is_empty() || zryna.len() != request.sources.len() {
        return Err(denied(
            "TRUST-SOURCE",
            "FFI source map must match the complete retained package Zryna subset",
        ));
    }
    for (index, expected) in zryna.iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| denied("TRUST-SOURCE", "FFI source count is invalid"))?;
        let file = request
            .sources
            .verify_file_id(index)
            .ok()
            .and_then(|id| request.sources.source(id))
            .ok_or_else(|| denied("TRUST-SOURCE", "FFI source identity is absent"))?;
        if file.path().as_str() != expected.path() || file.text().as_bytes() != expected.bytes() {
            return Err(denied(
                "TRUST-SOURCE",
                "FFI source bytes differ from the retained package",
            ));
        }
    }
    Ok(())
}
