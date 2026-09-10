use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::resolver::ResolveError;

pub(crate) const MAX_WIRE_BYTES: usize = 65_536;

pub(crate) fn parse<T: DeserializeOwned>(input: &[u8]) -> Result<T, ResolveError> {
    if input.len() > MAX_WIRE_BYTES {
        return Err(ResolveError::budget("wire bytes exceed 65536"));
    }
    check_depth(input)?;
    let value: Value =
        serde_json::from_slice(input).map_err(|_| ResolveError::wire("invalid UTF-8 or JSON"))?;
    if encode(&value)? != input {
        return Err(ResolveError::wire("JSON bytes are not canonical"));
    }
    serde_json::from_value(value)
        .map_err(|_| ResolveError::schema("closed record schema rejected input"))
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ResolveError> {
    let value = serde_json::to_value(value)
        .map_err(|_| ResolveError::wire("record cannot be serialized"))?;
    let mut bytes = serde_json::to_vec(&value)
        .map_err(|_| ResolveError::wire("record cannot be serialized"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn digest<T: Serialize>(kind: &str, value: &T) -> Result<String, ResolveError> {
    let mut hasher = Sha256::new();
    hasher.update(b"ZRYNA-PACKAGE-RELEASE-V1\0");
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(encode(value)?);
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check_depth(input: &[u8]) -> Result<(), ResolveError> {
    let mut depth = 0_u8;
    let mut quoted = false;
    let mut escaped = false;
    for &byte in input {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if matches!(byte, b'{' | b'[') {
            depth =
                depth.checked_add(1).ok_or_else(|| ResolveError::budget("nesting exceeds 6"))?;
            if depth > 6 {
                return Err(ResolveError::budget("nesting exceeds 6"));
            }
        } else if matches!(byte, b'}' | b']') {
            depth = depth.saturating_sub(1);
        }
    }
    Ok(())
}
