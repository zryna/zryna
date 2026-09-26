use super::{PendingDefinition, RevisionCompiler, Server, profiles::AnalysisProfile};
use crate::{
    coordinates::{byte_range_to_positions, position_to_byte},
    params::{DefinitionParams, decode_params},
    protocol::{self, Incoming, RequestId},
};
use serde_json::{Value, json};
use std::time::Instant;
use zryna_driver::diagnostic_sessions::{DiagnosticQueryResponse, DiagnosticRevision, QueryStatus};
use zryna_source::SourceMap;

impl<Compiler: RevisionCompiler> Server<Compiler> {
    pub(super) fn definition(&mut self, message: Incoming) -> Vec<Value> {
        let Some(id) = message.id else {
            return Vec::new();
        };
        if self.profile == AnalysisProfile::ControlFlow {
            return vec![protocol::method_not_found(Some(&id))];
        }
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
        if self.pending.len() + self.pending_formatting.len() >= 32 {
            return vec![protocol::error(Some(&id), -32803, "Request limit exceeded")];
        }
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

    pub(super) fn definition_response(
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
}
