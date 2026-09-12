//! Bounded canonical JSON shared by installed distribution records.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use zryna_diagnostics::Diagnostic;

use super::admission_error;

pub(super) const MAX_RECORD_BYTES: usize = 256 * 1024;

pub(super) fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Diagnostic> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(admission_error("distribution record exceeds 262144 bytes"));
    }
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0_u8;
    for byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
        } else if *byte == b'"' {
            quoted = true;
        } else if *byte == b'{' || *byte == b'[' {
            depth += 1;
            if depth > 12 {
                return Err(admission_error("distribution JSON nesting exceeds 12"));
            }
        } else if *byte == b'}' || *byte == b']' {
            depth = depth.checked_sub(1).ok_or_else(|| admission_error("invalid JSON nesting"))?;
        }
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| admission_error("distribution record is not valid UTF-8 JSON"))?;
    if canonical(&value)? != bytes {
        return Err(admission_error("distribution record is noncanonical or has duplicate keys"));
    }
    serde_json::from_value(value)
        .map_err(|_| admission_error("distribution record schema mismatch"))
}

pub(super) fn canonical(value: &impl Serialize) -> Result<Vec<u8>, Diagnostic> {
    let mut value = serde_json::to_value(value)
        .map_err(|_| admission_error("distribution record cannot be serialized"))?;
    sort_keys(&mut value);
    let mut bytes = serde_json::to_vec(&value)
        .map_err(|_| admission_error("distribution record cannot be serialized"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sort_keys(value: &mut Value) {
    match value {
        Value::Object(object) => {
            let mut entries = std::mem::take(object).into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            for (key, mut value) in entries {
                sort_keys(&mut value);
                object.insert(key, value);
            }
        }
        Value::Array(array) => array.iter_mut().for_each(sort_keys),
        _ => {}
    }
}
