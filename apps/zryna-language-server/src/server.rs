use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Display,
    time::Instant,
};

use serde_json::{Value, json};
use zryna_driver::diagnostic_sessions::{
    DiagnosticQueryResponse, DiagnosticRevision, DiagnosticSession, PendingDefinitionQuery,
    QueryStatus, ToolingCompiler,
};
use zryna_source::SourceMap;

use crate::{
    coordinates::{PositionEncoding, byte_range_to_positions, position_to_byte},
    diagnostics::{decode_report, standard_diagnostics},
    documents::{Document, source_map},
    params::{
        CancelParams, DefinitionParams, DidChangeParams, DidCloseParams, DidOpenParams,
        InitializeParams, decode_params, normalize_root_uri, path_below_root,
    },
    protocol::{
        self, Incoming, RequestId, empty_params, log_invalid, log_message, response_message,
    },
};

/// Compiler boundary used by the transport to admit one immutable source revision.
pub trait RevisionCompiler {
    /// Stable configuration or admission failure.
    type Error: Display;

    /// Analyzes and admits the exact source map into the driver-owned session.
    ///
    /// # Errors
    ///
    /// Returns the compiler boundary's stable configuration or admission failure.
    fn admit(
        &mut self,
        session: &mut DiagnosticSession,
        sources: SourceMap,
    ) -> Result<DiagnosticRevision, Self::Error>;
}

impl RevisionCompiler for ToolingCompiler {
    type Error = zryna_driver::diagnostic_sessions::ToolingCompilerError;

    fn admit(
        &mut self,
        session: &mut DiagnosticSession,
        sources: SourceMap,
    ) -> Result<DiagnosticRevision, Self::Error> {
        ToolingCompiler::admit(self, session, sources)
    }
}

#[derive(Clone, Debug)]
struct ActiveSnapshot {
    revision: DiagnosticRevision,
    sources: SourceMap,
}

struct PendingDefinition {
    id: RequestId,
    query: PendingDefinitionQuery,
    revision: DiagnosticRevision,
    sources: SourceMap,
}

/// One bounded LSP connection and its immutable compiler revisions.
pub struct Server<Compiler> {
    compiler: Compiler,
    session: DiagnosticSession,
    root_uri: Option<String>,
    encoding: PositionEncoding,
    documents: BTreeMap<String, Document>,
    active: Option<ActiveSnapshot>,
    pending: VecDeque<PendingDefinition>,
    initialized: bool,
    shutting_down: bool,
    exit: bool,
}

impl<Compiler: RevisionCompiler> Server<Compiler> {
    /// Creates an uninitialized connection.
    ///
    /// # Errors
    ///
    /// Returns an error only if process-local session identity is exhausted.
    pub fn new(
        compiler: Compiler,
    ) -> Result<Self, zryna_driver::diagnostic_sessions::DiagnosticSessionError> {
        Ok(Self {
            compiler,
            session: DiagnosticSession::try_new()?,
            root_uri: None,
            encoding: PositionEncoding::Utf16,
            documents: BTreeMap::new(),
            active: None,
            pending: VecDeque::new(),
            initialized: false,
            shutting_down: false,
            exit: false,
        })
    }

    /// Decodes and handles one complete JSON-RPC payload.
    #[must_use]
    pub fn handle_bytes(&mut self, bytes: &[u8]) -> Vec<Value> {
        match protocol::decode(bytes) {
            Ok(message) => self.handle(message),
            Err(error) => vec![error],
        }
    }

    /// Finishes all admitted definition requests after queued cancellation or edits were observed.
    #[must_use]
    pub fn finish_pending(&mut self) -> Vec<Value> {
        let mut output = Vec::new();
        while let Some(pending) = self.pending.pop_front() {
            let response = self.session.finish_definition(pending.query, Instant::now());
            output.push(self.definition_response(
                &pending.id,
                pending.revision,
                &pending.sources,
                &response,
            ));
        }
        output
    }

    /// Returns whether a valid `exit` notification ended this connection.
    #[must_use]
    pub const fn should_exit(&self) -> bool {
        self.exit
    }

    /// Returns whether definition work is waiting for queued cancellation or revision changes.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    fn handle(&mut self, message: Incoming) -> Vec<Value> {
        if self.shutting_down && message.method != "exit" && message.method != "$/cancelRequest" {
            return message.id.as_ref().map_or_else(Vec::new, |id| {
                vec![protocol::error(Some(id), -32600, "Server is shutting down")]
            });
        }
        match message.method.as_str() {
            "initialize" => self.initialize(message),
            "initialized"
                if message.id.is_none()
                    && self.initialized
                    && empty_params(message.params.as_ref()) =>
            {
                Vec::new()
            }
            "textDocument/didOpen" if message.id.is_none() && self.initialized => {
                self.did_open(message.params)
            }
            "textDocument/didChange" if message.id.is_none() && self.initialized => {
                self.did_change(message.params)
            }
            "textDocument/didClose" if message.id.is_none() && self.initialized => {
                self.did_close(message.params)
            }
            "textDocument/definition" if message.id.is_some() && self.initialized => {
                self.definition(message)
            }
            "$/cancelRequest" if message.id.is_none() && self.initialized => {
                self.cancel(message.params)
            }
            "shutdown" if message.id.is_some() && self.initialized => {
                let Some(id) = message.id.as_ref() else {
                    return Vec::new();
                };
                if !empty_params(message.params.as_ref()) {
                    return vec![protocol::invalid_params(Some(id))];
                }
                self.shutting_down = true;
                vec![protocol::response(id, &Value::Null)]
            }
            "exit"
                if message.id.is_none()
                    && self.shutting_down
                    && empty_params(message.params.as_ref()) =>
            {
                self.exit = true;
                Vec::new()
            }
            _ => message
                .id
                .as_ref()
                .map_or_else(Vec::new, |id| vec![protocol::method_not_found(Some(id))]),
        }
    }

    fn initialize(&mut self, message: Incoming) -> Vec<Value> {
        let Some(id) = message.id.as_ref() else {
            return Vec::new();
        };
        if self.initialized {
            return vec![protocol::invalid_request()];
        }
        let Some(params) = decode_params::<InitializeParams>(message.params) else {
            return vec![protocol::invalid_params(Some(id))];
        };
        let Some(root_uri) = normalize_root_uri(&params.root_uri) else {
            return vec![protocol::invalid_params(Some(id))];
        };
        self.encoding = PositionEncoding::select(
            params.capabilities.general.as_ref().and_then(|g| g.position_encodings.as_deref()),
        );
        self.root_uri = Some(root_uri);
        self.initialized = true;
        vec![protocol::response(
            id,
            &json!({
                "capabilities": {
                    "positionEncoding": self.encoding.as_str(),
                    "textDocumentSync": {"openClose":true,"change":1},
                    "definitionProvider": true
                },
                "serverInfo": {"name":"zryna-language-server","version":env!("CARGO_PKG_VERSION")}
            }),
        )]
    }

    fn did_open(&mut self, params: Option<Value>) -> Vec<Value> {
        let Some(params) = decode_params::<DidOpenParams>(params) else {
            return vec![log_invalid()];
        };
        let item = params.text_document;
        let Some(path) = self.path_for_uri(&item.uri) else {
            return vec![log_invalid()];
        };
        if item.language_id != "zryna" || item.version < 0 || !self.documents.is_empty() {
            return vec![log_invalid()];
        }
        let mut candidate = self.documents.clone();
        candidate
            .insert(item.uri.clone(), Document { path, text: item.text, version: item.version });
        self.admit_documents(candidate)
    }

    fn did_change(&mut self, params: Option<Value>) -> Vec<Value> {
        let Some(params) = decode_params::<DidChangeParams>(params) else {
            return vec![log_invalid()];
        };
        let [change] = params.content_changes.as_slice() else {
            return vec![log_invalid()];
        };
        let mut candidate = self.documents.clone();
        let Some(document) = candidate.get_mut(&params.text_document.uri) else {
            return vec![log_invalid()];
        };
        if params.text_document.version <= document.version {
            return vec![log_invalid()];
        }
        document.text.clone_from(&change.text);
        document.version = params.text_document.version;
        self.admit_documents(candidate)
    }

    fn did_close(&mut self, params: Option<Value>) -> Vec<Value> {
        let Some(params) = decode_params::<DidCloseParams>(params) else {
            return vec![log_invalid()];
        };
        let Some(closed) = self.documents.remove(&params.text_document.uri) else {
            return vec![log_invalid()];
        };
        let mut output = vec![protocol::notification(
            "textDocument/publishDiagnostics",
            &json!({
                "uri":params.text_document.uri,"version":closed.version,"diagnostics":[]
            }),
        )];
        if self.documents.is_empty() {
            match DiagnosticSession::try_new() {
                Ok(session) => self.session = session,
                Err(error) => output.push(log_message(&error.to_string())),
            }
            self.active = None;
        } else {
            output.extend(self.rebuild());
        }
        output
    }

    fn definition(&mut self, message: Incoming) -> Vec<Value> {
        let Some(id) = message.id else {
            return Vec::new();
        };
        let Some(params) = decode_params::<DefinitionParams>(message.params) else {
            return vec![protocol::invalid_params(Some(&id))];
        };
        let Some(active) = self.active.as_ref() else {
            return vec![protocol::error(Some(&id), -32803, "Analysis unavailable")];
        };
        let Some(document) = self.documents.get(&params.text_document.uri) else {
            return vec![protocol::invalid_params(Some(&id))];
        };
        let Some(byte_offset) = position_to_byte(&document.text, params.position, self.encoding)
        else {
            return vec![protocol::invalid_params(Some(&id))];
        };
        let request = json!({
            "query_version":1,
            "request_id":id.internal(),
            "snapshot":active.revision.handle().to_string(),
            "revision":active.revision.revision(),
            "method":"definition",
            "params":{"path":document.path,"byte_offset":byte_offset},
            "limits":{"work":100_000,"results":1}
        });
        let Ok(bytes) = serde_json::to_vec(&request) else {
            return vec![protocol::error(Some(&id), -32603, "Internal error")];
        };
        match self.session.begin_definition(&bytes, Instant::now()) {
            Ok(query) => {
                self.pending.push_back(PendingDefinition {
                    id,
                    query,
                    revision: active.revision,
                    sources: active.sources.clone(),
                });
                Vec::new()
            }
            Err(response) => {
                vec![self.definition_response(&id, active.revision, &active.sources, &response)]
            }
        }
    }

    fn cancel(&mut self, params: Option<Value>) -> Vec<Value> {
        let Some(params) = decode_params::<CancelParams>(params) else {
            return vec![log_invalid()];
        };
        let Some(id) = protocol::decode_id(params.id) else {
            return vec![log_invalid()];
        };
        let _ = self.session.cancel(id.internal());
        Vec::new()
    }

    fn rebuild(&mut self) -> Vec<Value> {
        let sources = match source_map(&self.documents) {
            Ok(sources) => sources,
            Err(error) => {
                self.active = None;
                if let Ok(session) = DiagnosticSession::try_new() {
                    self.session = session;
                }
                return vec![log_message(&error.to_string())];
            }
        };
        self.admit_sources(sources)
    }

    fn admit_documents(&mut self, documents: BTreeMap<String, Document>) -> Vec<Value> {
        let sources = match source_map(&documents) {
            Ok(sources) => sources,
            Err(error) => return vec![log_message(&error.to_string())],
        };
        self.documents = documents;
        self.admit_sources(sources)
    }

    fn admit_sources(&mut self, sources: SourceMap) -> Vec<Value> {
        match self.compiler.admit(&mut self.session, sources.clone()) {
            Ok(revision) => {
                self.active = Some(ActiveSnapshot { revision, sources });
                self.publish_diagnostics(revision)
            }
            Err(error) => {
                let fallback = self.session.admit_unready(sources.clone());
                self.active = fallback.ok().map(|revision| ActiveSnapshot { revision, sources });
                vec![log_message(&error.to_string())]
            }
        }
    }

    fn publish_diagnostics(&mut self, revision: DiagnosticRevision) -> Vec<Value> {
        let request = json!({
            "query_version":1,"request_id":format!("diagnostics:{}", revision.revision()),
            "snapshot":revision.handle().to_string(),"revision":revision.revision(),
            "method":"diagnostics","params":{},"limits":{"work":100_000,"results":10_000}
        });
        let Ok(bytes) = serde_json::to_vec(&request) else {
            return vec![log_message("diagnostic request encoding failed")];
        };
        let pending = match self.session.begin_diagnostics(&bytes, Instant::now()) {
            Ok(pending) => pending,
            Err(response) => return vec![log_message(response_message(&response))],
        };
        let response = self.session.finish_diagnostics(pending, Instant::now());
        let Some(encoded) =
            response.encoded().and_then(|value| serde_json::from_str::<Value>(value).ok())
        else {
            return vec![log_message(response_message(&response))];
        };
        let Some(report) = decode_report(&encoded) else {
            return vec![log_message("diagnostic response validation failed")];
        };
        let versions = self
            .documents
            .iter()
            .map(|(uri, document)| json!({"uri":uri,"version":document.version}))
            .collect::<Vec<_>>();
        let mut output = vec![protocol::notification(
            "zryna/publishDiagnostics",
            &json!({
                "snapshot":revision.handle().to_string(),"revision":revision.revision(),"documents":versions,"report":report
            }),
        )];
        let Some(standard) = standard_diagnostics(&report, &self.documents, self.encoding) else {
            output.push(log_message("diagnostic coordinate conversion failed"));
            return output;
        };
        output.extend(standard.into_iter().map(|(uri, version, diagnostics)| {
            protocol::notification(
                "textDocument/publishDiagnostics",
                &json!({
                    "uri":uri,"version":version,"diagnostics":diagnostics
                }),
            )
        }));
        output
    }

    fn definition_response(
        &self,
        id: &RequestId,
        revision: DiagnosticRevision,
        sources: &SourceMap,
        response: &DiagnosticQueryResponse,
    ) -> Value {
        match response.status() {
            QueryStatus::Absent => protocol::response(id, &Value::Null),
            QueryStatus::Ok => self
                .definition_success(id, revision, sources, response)
                .unwrap_or_else(|| protocol::error(Some(id), -32603, "Internal error")),
            QueryStatus::Stale => protocol::error(Some(id), -32801, "Content modified"),
            QueryStatus::Cancelled => protocol::error(Some(id), -32800, "Request cancelled"),
            QueryStatus::Malformed => protocol::invalid_params(Some(id)),
            QueryStatus::Unsupported => protocol::method_not_found(Some(id)),
            QueryStatus::Unavailable => protocol::error(Some(id), -32803, "Analysis unavailable"),
            QueryStatus::OverBudget => protocol::error(Some(id), -32803, "Request limit exceeded"),
        }
    }

    fn definition_success(
        &self,
        id: &RequestId,
        revision: DiagnosticRevision,
        sources: &SourceMap,
        response: &DiagnosticQueryResponse,
    ) -> Option<Value> {
        let value: Value = serde_json::from_str(response.encoded()?).ok()?;
        if value.get("snapshot")?.as_str()? != revision.handle().to_string()
            || value.get("revision")?.as_u64()? != revision.revision()
            || value.get("request_id")?.as_str()? != id.internal()
        {
            return None;
        }
        let locations = value.get("result")?.get("locations")?.as_array()?;
        let [location] = locations.as_slice() else {
            return None;
        };
        let path = location.get("path")?.as_str()?;
        let start = u32::try_from(location.get("byte_start")?.as_u64()?).ok()?;
        let end = u32::try_from(location.get("byte_end")?.as_u64()?).ok()?;
        let (uri, document) = self.documents.iter().find(|(_, document)| document.path == path)?;
        let normalized = zryna_source::NormalizedSourcePath::new(path.to_owned()).ok()?;
        let file = sources.file_id(&normalized)?;
        let span = sources.span(file, start, end).ok()?;
        let resolved = sources.resolve(span).ok()?;
        if resolved.source().text() != document.text {
            return None;
        }
        let range = byte_range_to_positions(&document.text, start, end, self.encoding)?;
        Some(protocol::response(id, &json!({"uri":uri,"range":range})))
    }

    fn path_for_uri(&self, uri: &str) -> Option<String> {
        let root = self.root_uri.as_ref()?;
        path_below_root(root, uri)
    }
}
