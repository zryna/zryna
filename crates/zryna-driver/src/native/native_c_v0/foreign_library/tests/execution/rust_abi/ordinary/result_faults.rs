//! Actual private hostile Rust-result observations; no pristine or production grant.
use super::*;
mod clients;
mod measurement;
mod observe;
mod source;
mod tests;
use measurement::HostileObservation;
use source::Kind;

struct Request<'a> {
    requirements: &'a HandleLinkRequirements,
    library: &'a CapturedRustProfileLibrary,
    hostile: &'a HostileObservation,
}
impl Request<'_> {
    fn check(&self) -> Result<(), Diagnostic> {
        self.library.check_binding(self.requirements)?;
        self.hostile.check()?;
        if self.library.bytes() != self.hostile.object()
            || self.library.source_bytes() != self.hostile.source()
        {
            return Err(rejected());
        }
        Ok(())
    }
}
