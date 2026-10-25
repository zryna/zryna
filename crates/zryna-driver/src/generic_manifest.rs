//! Separate bounded-generics metadata. A decoded document is never an execution authority.
mod model;
pub(crate) mod source;
mod validate;
pub(crate) use model::*;
use std::io::Write as _;
use zryna_diagnostics::Diagnostic;

pub(crate) const MANIFEST_NAME: &str = "zryna-generic-manifest-v1.json";
pub(crate) const FORMAT: &str = "zryna.generic-bundle.v1";
pub(crate) const PROFILE: &str = "bounded-generics-v1";
// Same bounded envelope as the existing versioned control-flow manifest.
const MAX_MANIFEST_BYTES: usize = 32 * 1024 * 1024;

fn invalid(reason: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C3410",
        None,
        format!("generic manifest metadata is inconsistent: {reason}"),
        "retain the source and generated files for diagnosis",
    )
}

/// Parses strict metadata only; this function cannot issue any verified program or grant.
#[cfg(test)]
pub(crate) fn decode(bytes: &[u8]) -> Result<GenericManifestWire, Diagnostic> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid("manifest envelope exceeds its existing bound"));
    }
    let wire = serde_json::from_slice(bytes).map_err(|_| invalid("closed JSON schema"))?;
    validate::check(&wire).map_err(invalid)?;
    Ok(wire)
}

/// Serializes already-owned metadata only after its closed shape has been validated.
pub(crate) fn encode(wire: &GenericManifestWire) -> Result<Vec<u8>, Diagnostic> {
    validate::check(wire).map_err(invalid)?;
    let mut writer = BoundedWriter { bytes: Vec::new() };
    serde_json::to_writer_pretty(&mut writer, wire)
        .map_err(|_| invalid("bounded JSON serialization"))?;
    writer.write_all(b"\n").map_err(|_| invalid("manifest envelope bound"))?;
    Ok(writer.bytes)
}

/// Fingerprint compact retained evidence under the same checked metadata envelope.
pub(crate) fn receipt_sha256(value: &impl serde::Serialize) -> Result<String, Diagnostic> {
    use sha2::{Digest, Sha256};
    let mut writer = BoundedWriter { bytes: Vec::new() };
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| invalid("bounded provenance serialization"))?;
    Ok(source::hex(&Sha256::digest(&writer.bytes).into()))
}

struct BoundedWriter {
    bytes: Vec<u8>,
}
impl std::io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let end = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|end| *end <= MAX_MANIFEST_BYTES)
            .ok_or_else(|| std::io::Error::other("generic manifest envelope bound"))?;
        self.bytes
            .try_reserve_exact(end - self.bytes.len())
            .map_err(|_| std::io::Error::other("generic manifest allocation"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
