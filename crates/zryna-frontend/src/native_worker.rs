//! Native syntax producer for the existing bounded, verified worker protocol.
//!
//! This producer receives only source text over stdin. It does not resolve imports, inspect
//! host paths, launch processes, or grant authority to its untrusted syntax output.

use std::{
    fmt,
    io::{self, BufRead, Read, Write},
};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, SeqAccess, Visitor},
};
use serde_json::{Value, json};
use zryna_diagnostics::Diagnostic;
use zryna_source::{
    MAX_SOURCE_FILE_BYTES, MAX_SOURCE_FILES, MAX_SOURCE_PATH_BYTES, SourceFileInput, SourceMap,
};

use crate::{
    MAX_HANDSHAKE_RESPONSE_BYTES, MAX_WORKER_REQUEST_BYTES, SourceInput, native_lexer,
    native_parser, syntax_v2, syntax_v3, syntax_v4,
};

/// Exact native producer identity; it is distinct from the bootstrap and private test harness.
pub const NATIVE_PROVIDER: &str = "zryna-native";
/// Exact compiler-owned producer version.
pub const NATIVE_PROVIDER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A rejected native worker session. No error confers verified syntax authority.
#[derive(Debug)]
pub struct NativeWorkerError(&'static str);

impl fmt::Display for NativeWorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for NativeWorkerError {}

#[derive(Deserialize)]
#[serde(tag = "method", deny_unknown_fields)]
enum Request {
    #[serde(rename = "handshake")]
    Handshake { id: u32 },
    #[serde(rename = "analyze")]
    Analyze { id: u32, params: Analysis },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Analysis {
    schema_version: u32,
    #[serde(deserialize_with = "source_files")]
    files: Vec<SourceInput>,
}

fn source_files<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<SourceInput>, D::Error> {
    struct Files;
    impl<'de> Visitor<'de> for Files {
        type Value = Vec<SourceInput>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a bounded authoritative source set")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut files = Vec::new();
            let mut total = 0_usize;
            while let Some(file) = sequence.next_element::<SourceInput>()? {
                total = total
                    .checked_add(file.text.len())
                    .ok_or_else(|| A::Error::custom("native source size overflow"))?;
                if files.len() == MAX_SOURCE_FILES
                    || file.path.len() > MAX_SOURCE_PATH_BYTES
                    || file.text.len() > MAX_SOURCE_FILE_BYTES
                    || total > native_lexer::MAX_SOURCE_BYTES_PER_PROJECT
                {
                    return Err(A::Error::custom("native source budget exceeded"));
                }
                files.push(file);
            }
            Ok(files)
        }
    }
    de.deserialize_seq(Files)
}

fn request(
    reader: &mut impl BufRead,
    maximum: usize,
) -> Result<Option<Request>, NativeWorkerError> {
    let mut bytes = Vec::new();
    (&mut *reader)
        .take((maximum + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(|_| NativeWorkerError("native worker request I/O failed"))?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > maximum || bytes.last() != Some(&b'\n') {
        return Err(NativeWorkerError("native worker request framing or byte limit rejected"));
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| NativeWorkerError("native worker requires a closed protocol request"))
}

struct BoundedBuffer {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for BoundedBuffer {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("native worker response byte limit exceeded"));
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn respond(
    writer: &mut impl Write,
    value: &impl Serialize,
    maximum: usize,
) -> Result<(), NativeWorkerError> {
    let mut bounded = BoundedBuffer { bytes: Vec::new(), maximum };
    serde_json::to_writer(&mut bounded, value)
        .map_err(|_| NativeWorkerError("native worker response encoding or byte limit rejected"))?;
    writer
        .write_all(&bounded.bytes)
        .and_then(|()| writer.write_all(b"\n"))
        .and_then(|()| writer.flush())
        .map_err(|_| NativeWorkerError("native worker response I/O failed"))
}

fn rejection(diagnostic: &Diagnostic) -> Value {
    json!({"id":2,"error":{"code":diagnostic.code(),"message":diagnostic.message()}})
}

fn candidate(protocol: u32, input: Analysis) -> Result<Value, NativeWorkerError> {
    if input.schema_version != protocol {
        return Err(NativeWorkerError("native worker analysis protocol mismatch"));
    }
    let sources = SourceMap::build(
        input
            .files
            .into_iter()
            .map(|file| SourceFileInput { path: file.path, text: file.text })
            .collect(),
    )
    .map_err(|_| NativeWorkerError("native worker source admission rejected"))?;
    let lexed = match native_lexer::lex(&sources) {
        Ok(value) => value,
        Err(error) => return Ok(rejection(error.diagnostic())),
    };
    let raw = match protocol {
        2 => native_parser::parse_v2_recovering_candidate(&sources, &lexed)
            .map(|value| serde_json::to_value(value))
            .map_err(|error| rejection(error.diagnostic())),
        3 => native_parser::v3::parse_v3_straight_line_candidate(&sources, &lexed)
            .map(|value| serde_json::to_value(value))
            .map_err(|error| rejection(error.diagnostic())),
        4 => native_parser::v4::parse_v4_candidate(&sources, &lexed)
            .map(|value| serde_json::to_value(value))
            .map_err(|error| rejection(error.diagnostic())),
        _ => return Err(NativeWorkerError("native worker protocol is unsupported")),
    };
    match raw {
        Ok(value) => value
            .map(|raw| json!({"id":2,"result":raw}))
            .map_err(|_| NativeWorkerError("native worker candidate encoding failed")),
        Err(response) => Ok(response),
    }
}

/// Serves precisely one native handshake and one analysis over stdin/stdout.
///
/// The caller must consume output through the matching original `WorkerFrontend` verifier.
/// The owning worker process supplies the bounded wall-time and cleanup protocol.
///
/// # Errors
/// Rejects unknown versions, requests, source maps, framing, output overflow and I/O failures.
pub fn run_native_worker_stdio(protocol: u32) -> Result<(), NativeWorkerError> {
    run(protocol, &mut io::stdin().lock(), &mut io::stdout().lock())
}

fn run(
    protocol: u32,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<(), NativeWorkerError> {
    let maximum = match protocol {
        2 => syntax_v2::MAX_RESPONSE_BYTES,
        3 => syntax_v3::MAX_RESPONSE_BYTES,
        4 => syntax_v4::MAX_RESPONSE_BYTES,
        _ => return Err(NativeWorkerError("native worker protocol is unsupported")),
    };
    if !matches!(request(reader, MAX_WORKER_REQUEST_BYTES)?, Some(Request::Handshake { id: 1 })) {
        return Err(NativeWorkerError("native worker requires the exact initial handshake"));
    }
    let mut capabilities = json!({"module_resolution":false,"semantic_diagnostics":false});
    if protocol >= 3 {
        capabilities["control_flow_v1"] = json!(true);
    }
    if protocol == 4 {
        capabilities["data_ownership_syntax_v1"] = json!(true);
    }
    respond(
        writer,
        &json!({"id":1,"result":{"provider":NATIVE_PROVIDER,
        "provider_version":NATIVE_PROVIDER_VERSION,"protocol_version":protocol,
        "capabilities":capabilities}}),
        MAX_HANDSHAKE_RESPONSE_BYTES,
    )?;
    let Some(Request::Analyze { id: 2, params }) = request(reader, MAX_WORKER_REQUEST_BYTES)?
    else {
        return Err(NativeWorkerError("native worker requires one correlated analysis"));
    };
    let response = candidate(protocol, params)?;
    respond(writer, &response, maximum)?;
    if request(reader, MAX_WORKER_REQUEST_BYTES)?.is_some() {
        return Err(NativeWorkerError("native worker rejects additional requests"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
