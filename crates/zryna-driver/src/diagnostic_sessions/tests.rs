use std::time::Instant;

use serde_json::json;
use zryna_source::{SourceFileInput, SourceMap};

use super::{DiagnosticRevision, DiagnosticSession};

mod bounds;
mod definition;
mod hostile;
mod lifecycle;

fn sources(path: &str, text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: path.to_owned(), text: text.to_owned() }])
        .unwrap_or_else(|error| panic!("test source must be valid: {error}"))
}

fn request(
    id: &str,
    revision: DiagnosticRevision,
    work: u64,
    method: &str,
    params: serde_json::Value,
) -> Vec<u8> {
    let mut request = json!({
        "query_version": 1,
        "request_id": id,
        "snapshot": revision.handle().to_string(),
        "revision": revision.revision(),
        "method": method,
        "limits": { "work": work, "results": 10_000 }
    });
    request["params"] = params;
    serde_json::to_vec(&request).unwrap_or_else(|error| panic!("test request must encode: {error}"))
}

fn ready_session(text: &str) -> (DiagnosticSession, DiagnosticRevision, Instant) {
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    let revision = session
        .admit_diagnostics(sources("src/main.zry", text), &[])
        .unwrap_or_else(|error| panic!("empty diagnostics must be admitted: {error}"));
    (session, revision, Instant::now())
}
