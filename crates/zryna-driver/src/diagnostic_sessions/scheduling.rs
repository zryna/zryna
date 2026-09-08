use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Instant,
};

use zryna_diagnostics::protocol_v2;
use zryna_source::SourceMapIdentity;

use super::{
    CANCELLED, Correlation, DiagnosticQueryResponse, DiagnosticRevision, DiagnosticSession, LIVE,
    MAX_IN_FLIGHT_REQUESTS, PendingDiagnosticQuery, QUERY_DEADLINE, QueryReason, QueryStatus,
    RequestSlot, TIMED_OUT,
};

impl DiagnosticSession {
    /// Validates and admits one closed diagnostics request without performing its query work.
    ///
    /// # Errors
    ///
    /// Returns a complete correlated failure when safe correlation fields were decoded, otherwise
    /// an uncorrelated rejection. No partial diagnostic result is returned.
    pub(crate) fn begin_diagnostics(
        &mut self,
        bytes: &[u8],
        now: Instant,
    ) -> Result<PendingDiagnosticQuery, DiagnosticQueryResponse> {
        let request = super::request::decode(bytes).map_err(|error| match error {
            super::request::DecodeError::RequestBytes => DiagnosticQueryResponse::uncorrelated(
                QueryStatus::OverBudget,
                QueryReason::RequestBytes,
            ),
            super::request::DecodeError::RequestDepth => DiagnosticQueryResponse::uncorrelated(
                QueryStatus::OverBudget,
                QueryReason::RequestDepth,
            ),
            super::request::DecodeError::Shape => {
                DiagnosticQueryResponse::uncorrelated(QueryStatus::Malformed, QueryReason::Shape)
            }
        })?;
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
        if request.method != "diagnostics" {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Unsupported,
                QueryReason::Method,
            ));
        }
        if !request.validate_diagnostic_params() {
            return Err(DiagnosticQueryResponse::failure(
                correlation,
                QueryStatus::Malformed,
                QueryReason::Params,
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
        Ok(PendingDiagnosticQuery {
            session: self.session,
            correlation: request.correlation,
            revision: record.description,
            record: Arc::downgrade(record),
            source_identity: record.sources.identity(),
            work_limit: request.work_limit,
            deadline,
            state,
        })
    }

    /// Cancels one admitted request and immediately releases its queue slot.
    #[must_use]
    pub(crate) fn cancel(&mut self, request_id: &str) -> bool {
        let Some(slot) = self.in_flight.remove(request_id) else {
            return false;
        };
        let _ = slot.state.compare_exchange(LIVE, CANCELLED, Ordering::AcqRel, Ordering::Acquire);
        true
    }

    /// Completes one query against its retained source and atomically rechecks the active revision.
    #[must_use]
    pub(crate) fn finish_diagnostics(
        &mut self,
        pending: PendingDiagnosticQuery,
        now: Instant,
    ) -> DiagnosticQueryResponse {
        self.release(&pending);
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
        if pending.state.load(Ordering::Acquire) == TIMED_OUT {
            return DiagnosticQueryResponse::failure(
                &pending.correlation,
                QueryStatus::Cancelled,
                QueryReason::Deadline,
            );
        }
        if now >= pending.deadline {
            return DiagnosticQueryResponse::failure(
                &pending.correlation,
                QueryStatus::Cancelled,
                QueryReason::Deadline,
            );
        }
        let Some(record) = pending.record.upgrade() else {
            return stale(&pending.correlation);
        };
        let response = if let Some(report) = record.report.as_deref() {
            let logical_work = u64::try_from(report.len())
                .ok()
                .and_then(|value| value.checked_add(1))
                .unwrap_or(u64::MAX);
            if logical_work > pending.work_limit {
                DiagnosticQueryResponse::failure(
                    &pending.correlation,
                    QueryStatus::OverBudget,
                    QueryReason::Work,
                )
            } else if protocol_v2::validate_json(report.as_bytes(), &record.sources).is_err() {
                DiagnosticQueryResponse::failure(
                    &pending.correlation,
                    QueryStatus::Unavailable,
                    QueryReason::Analysis,
                )
            } else {
                DiagnosticQueryResponse::success(&pending.correlation, report)
            }
        } else {
            DiagnosticQueryResponse::failure(
                &pending.correlation,
                QueryStatus::Unavailable,
                QueryReason::Analysis,
            )
        };
        if !self.is_active(pending.revision, pending.source_identity) {
            return stale(&pending.correlation);
        }
        response
    }

    fn release(&mut self, pending: &PendingDiagnosticQuery) {
        if self
            .in_flight
            .get(&pending.correlation.request_id)
            .is_some_and(|slot| Arc::ptr_eq(&slot.state, &pending.state))
        {
            self.in_flight.remove(&pending.correlation.request_id);
        }
    }

    fn expire_deadlines(&mut self, now: Instant) {
        self.in_flight.retain(|_, slot| {
            if now < slot.deadline {
                return true;
            }
            let _ =
                slot.state.compare_exchange(LIVE, TIMED_OUT, Ordering::AcqRel, Ordering::Acquire);
            false
        });
    }

    fn is_active(&self, revision: DiagnosticRevision, source_identity: SourceMapIdentity) -> bool {
        self.retained.back().is_some_and(|active| {
            active.sources.identity() == source_identity && active.description == revision
        })
    }
}

fn stale(correlation: &Correlation) -> DiagnosticQueryResponse {
    DiagnosticQueryResponse::failure(correlation, QueryStatus::Stale, QueryReason::Snapshot)
}
