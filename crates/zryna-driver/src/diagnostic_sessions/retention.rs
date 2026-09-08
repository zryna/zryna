use sha2::{Digest, Sha256};
use zryna_source::SourceMap;

use super::{DiagnosticSessionError, MAX_SESSION_CACHE_BYTES};

pub(super) fn checked_cache_charge(
    source_bytes: usize,
    report_bytes: usize,
    semantic_bytes: usize,
) -> Result<usize, DiagnosticSessionError> {
    let charge = source_bytes
        .checked_add(report_bytes)
        .and_then(|value| value.checked_add(semantic_bytes))
        .ok_or(DiagnosticSessionError::SessionCacheExhausted)?;
    if charge > MAX_SESSION_CACHE_BYTES {
        return Err(DiagnosticSessionError::SessionCacheExhausted);
    }
    Ok(charge)
}

pub(super) fn source_fingerprint_and_charge(
    sources: &SourceMap,
) -> Result<([u8; 32], usize), DiagnosticSessionError> {
    let mut entries = Vec::with_capacity(sources.len());
    let mut cache_bytes = 0_usize;
    for index in 0..sources.len() {
        let raw = u32::try_from(index).map_err(|_| DiagnosticSessionError::SourceInvariant)?;
        let id =
            sources.verify_file_id(raw).map_err(|_| DiagnosticSessionError::SourceInvariant)?;
        let source = sources.source(id).ok_or(DiagnosticSessionError::SourceInvariant)?;
        cache_bytes = cache_bytes
            .checked_add(source.path().as_str().len())
            .and_then(|value| value.checked_add(source.text().len()))
            .ok_or(DiagnosticSessionError::SessionCacheExhausted)?;
        let digest = Sha256::digest(source.text().as_bytes());
        entries.push((source.path().as_str(), hex(&digest)));
    }
    let encoded =
        serde_json::to_vec(&entries).map_err(|_| DiagnosticSessionError::SourceInvariant)?;
    Ok((Sha256::digest(encoded).into(), cache_bytes))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}
