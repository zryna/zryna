use std::{
    sync::{
        Arc, Weak,
        atomic::{AtomicU8, Ordering},
    },
    time::Instant,
};

use zryna_semantics::definition_queries::DefinitionLookup;
use zryna_source::{NormalizedSourcePath, SourceMapIdentity};

use super::{
    CANCELLED, Correlation, DiagnosticQueryResponse, DiagnosticRevision, DiagnosticSession, LIVE,
    MAX_IN_FLIGHT_REQUESTS, QUERY_DEADLINE, QueryReason, QueryStatus, RequestSlot, RevisionRecord,
    TIMED_OUT,
    scheduling::{decode_failure, stale},
};

/// One admitted definition request awaiting bounded completion.
#[derive(Debug)]
pub(crate) struct PendingDefinitionQuery {
    session: u64,
    correlation: Correlation,
    revision: DiagnosticRevision,
    record: Weak<RevisionRecord>,
    source_identity: SourceMapIdentity,
    path: String,
    byte_offset: u32,
    work_limit: u64,
    deadline: Instant,
    state: Arc<AtomicU8>,
}

impl DiagnosticSession {
    /// Validates and admits one closed definition request without performing semantic lookup.
    pub(crate) fn begin_definition(
        &mut self,
        bytes: &[u8],
        now: Instant,
    ) -> Result<PendingDefinitionQuery, DiagnosticQueryResponse> {
        let request = super::request::decode(bytes).map_err(decode_failure)?;
        let correlation = &request.correlation;
        self.expire_deadlines(now);
        if self.in_flight.contains_key(&correlation.request_id) {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Malformed,
                QueryReason::Shape,
            ));
        }
        if correlation.query_version != 1 {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Unsupported,
                QueryReason::Version,
            ));
        }
        let Some(record) = self.retained.back() else {
            return Err(stale(correlation));
        };
        if correlation.snapshot != record.description.handle.to_string()
            || correlation.revision != record.description.revision
        {
            return Err(stale(correlation));
        }
        if request.method != "definition" {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Unsupported,
                QueryReason::Method,
            ));
        }
        let Some((path, byte_offset)) = request.definition_params() else {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Malformed,
                QueryReason::Params,
            ));
        };
        let normalized = NormalizedSourcePath::new(path.clone()).ok();
        let source = normalized
            .as_ref()
            .and_then(|normalized| record.sources.file_id(normalized))
            .and_then(|file| record.sources.source(file).map(|source| (file, source)));
        let Some((file, source)) = source else {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Malformed,
                QueryReason::Source,
            ));
        };
        if source.path().as_str() != path
            || record.sources.span(file, byte_offset, byte_offset).is_err()
        {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Malformed,
                QueryReason::Source,
            ));
        }
        if self.in_flight.len() >= MAX_IN_FLIGHT_REQUESTS {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::OverBudget,
                QueryReason::Session,
            ));
        }
        let state = Arc::new(AtomicU8::new(LIVE));
        let deadline = now.checked_add(QUERY_DEADLINE).unwrap_or(now);
        self.in_flight
            .insert(correlation.request_id.clone(), RequestSlot { state: state.clone(), deadline });
        Ok(PendingDefinitionQuery {
            session: self.session,
            correlation: request.correlation,
            revision: record.description,
            record: Arc::downgrade(record),
            source_identity: record.sources.identity(),
            path,
            byte_offset,
            work_limit: request.work_limit,
            deadline,
            state,
        })
    }

    /// Completes one definition lookup and rechecks the active revision before publication.
    #[must_use]
    pub(crate) fn finish_definition(
        &mut self,
        pending: PendingDefinitionQuery,
        now: Instant,
    ) -> DiagnosticQueryResponse {
        self.release_definition(&pending);
        if pending.session != self.session {
            return stale(&pending.correlation);
        }
        if pending.state.load(Ordering::Acquire) == CANCELLED {
            return DiagnosticQueryResponse::failure(
                &pending.correlation,
                QueryStatus::Cancelled,
                QueryReason::Request,
            );
        }
        if pending.state.load(Ordering::Acquire) == TIMED_OUT || now >= pending.deadline {
            return DiagnosticQueryResponse::failure(
                &pending.correlation,
                QueryStatus::Cancelled,
                QueryReason::Deadline,
            );
        }
        let Some(record) = pending.record.upgrade() else {
            return stale(&pending.correlation);
        };
        let response = match record.definitions.as_deref() {
            None => DiagnosticQueryResponse::failure(
                &pending.correlation,
                QueryStatus::Unavailable,
                QueryReason::Analysis,
            ),
            Some(definitions) => {
                let normalized = NormalizedSourcePath::new(pending.path.clone()).ok();
                let file = normalized.as_ref().and_then(|path| record.sources.file_id(path));
                match file.map(|file| {
                    definitions.definition(file, pending.byte_offset, pending.work_limit)
                }) {
                    Some(DefinitionLookup::Found(span)) => match record.sources.resolve(span) {
                        Ok(location) => DiagnosticQueryResponse::definition_success(
                            &pending.correlation,
                            location.source().path().as_str(),
                            span.start(),
                            span.end(),
                        ),
                        Err(_) => DiagnosticQueryResponse::failure(
                            &pending.correlation,
                            QueryStatus::Unavailable,
                            QueryReason::Analysis,
                        ),
                    },
                    Some(DefinitionLookup::Absent) => DiagnosticQueryResponse::failure(
                        &pending.correlation,
                        QueryStatus::Absent,
                        QueryReason::Symbol,
                    ),
                    Some(DefinitionLookup::OverBudget) => DiagnosticQueryResponse::failure(
                        &pending.correlation,
                        QueryStatus::OverBudget,
                        QueryReason::Work,
                    ),
                    None => DiagnosticQueryResponse::failure(
                        &pending.correlation,
                        QueryStatus::Unavailable,
                        QueryReason::Analysis,
                    ),
                }
            }
        };
        if !self.is_active(pending.revision, pending.source_identity) {
            return stale(&pending.correlation);
        }
        response
    }

    fn release_definition(&mut self, pending: &PendingDefinitionQuery) {
        if self
            .in_flight
            .get(&pending.correlation.request_id)
            .is_some_and(|slot| Arc::ptr_eq(&slot.state, &pending.state))
        {
            self.in_flight.remove(&pending.correlation.request_id);
        }
    }
}
