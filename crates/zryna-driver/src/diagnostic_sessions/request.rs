use serde::Deserialize;
use serde_json::Value;

use super::{
    Correlation, MAX_ID_BYTES, MAX_QUERY_RESULTS, MAX_QUERY_WORK, MAX_REQUEST_BYTES,
    MAX_REQUEST_DEPTH, MAX_REVISION,
};

#[derive(Debug)]
pub(super) enum DecodeError {
    RequestBytes,
    RequestDepth,
    ParseShape,
    UnsafeCorrelation,
    CorrelatedShape(Correlation),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLimits {
    work: u64,
    results: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    query_version: u64,
    request_id: String,
    snapshot: String,
    revision: u64,
    method: String,
    params: Value,
    limits: WireLimits,
}

#[derive(Debug)]
pub(super) struct ParsedRequest {
    pub(super) correlation: Correlation,
    pub(super) method: String,
    params: Value,
    pub(super) work_limit: u64,
}

impl ParsedRequest {
    pub(super) fn validate_diagnostic_params(&self) -> bool {
        self.params.as_object().is_some_and(serde_json::Map::is_empty)
    }
}

pub(super) fn decode(bytes: &[u8]) -> Result<ParsedRequest, DecodeError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(DecodeError::RequestBytes);
    }
    std::str::from_utf8(bytes).map_err(|_| DecodeError::ParseShape)?;
    if exceeds_depth(bytes) {
        return Err(DecodeError::RequestDepth);
    }
    let request: WireRequest = serde_json::from_slice(bytes).map_err(|_| DecodeError::ParseShape)?;
    if !valid_id(&request.request_id)
        || !valid_id(&request.snapshot)
        || request.revision == 0
        || request.revision > MAX_REVISION
    {
        return Err(DecodeError::UnsafeCorrelation);
    }
    let correlation = Correlation {
        query_version: request.query_version,
        request_id: request.request_id,
        snapshot: request.snapshot,
        revision: request.revision,
    };
    if request.limits.work == 0
        || request.limits.work > MAX_QUERY_WORK
        || request.limits.results == 0
        || request.limits.results > MAX_QUERY_RESULTS
    {
        return Err(DecodeError::CorrelatedShape(correlation));
    }
    Ok(ParsedRequest {
        correlation,
        method: request.method,
        params: request.params,
        work_limit: request.limits.work,
    })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ID_BYTES && value.is_ascii()
}

fn exceeds_depth(bytes: &[u8]) -> bool {
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_REQUEST_DEPTH {
                    return true;
                }
            }
            b'}' | b']' if depth > 0 => depth -= 1,
            _ => {}
        }
    }
    false
}
