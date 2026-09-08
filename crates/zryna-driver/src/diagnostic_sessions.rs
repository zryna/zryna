//! Revision-bound internal diagnostic sessions.
//!
//! This compiler-host boundary retains exact source-map and structured-diagnostics-v2 authority.
//! It is not a transport, semantic query service, CLI/LSP route, formatter, or executor.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    sync::{
        Arc, Weak,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use zryna_diagnostics::{Diagnostic, protocol_v2};
use zryna_source::{SourceMap, SourceMapIdentity};

mod request;
mod response;
mod retention;
mod scheduling;

pub(crate) use response::{DiagnosticQueryResponse, QueryReason, QueryStatus};
use retention::{checked_cache_charge, source_fingerprint_and_charge};

/// Maximum complete encoded request bytes, including whitespace.
pub(crate) const MAX_REQUEST_BYTES: usize = 65_536;
/// Maximum complete encoded response bytes.
pub(crate) const MAX_RESPONSE_BYTES: usize = 1_048_576;
/// Maximum JSON object/array nesting in one request.
pub(crate) const MAX_REQUEST_DEPTH: u32 = 64;
/// Maximum logical work units admitted by one request.
pub(crate) const MAX_QUERY_WORK: u64 = 100_000;
/// Maximum result locations or edits admitted by one request.
pub(crate) const MAX_QUERY_RESULTS: u64 = 10_000;
/// Maximum retained immutable revisions in one session.
pub(crate) const MAX_RETAINED_REVISIONS: usize = 2;
/// Maximum deterministically charged cache bytes in one session.
pub(crate) const MAX_SESSION_CACHE_BYTES: usize = 64 * 1_024 * 1_024;
/// Maximum queued plus running requests in one session.
pub(crate) const MAX_IN_FLIGHT_REQUESTS: usize = 32;
/// Maximum request lifetime, including cancellation cleanup.
pub(crate) const QUERY_DEADLINE: Duration = Duration::from_secs(30);
/// Maximum request or snapshot identifier bytes.
pub(crate) const MAX_ID_BYTES: usize = 128;
/// Largest exact JSON integer admitted for a session revision.
pub(crate) const MAX_REVISION: u64 = (1_u64 << 53) - 1;

const LIVE: u8 = 0;
const CANCELLED: u8 = 1;
const TIMED_OUT: u8 = 2;
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque compiler-host-issued identity for one retained source revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DiagnosticSnapshotHandle {
    session: u64,
    serial: u64,
}

impl fmt::Display for DiagnosticSnapshotHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "s{:016x}-{:013x}", self.session, self.serial)
    }
}

/// Immutable description of one admitted source revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DiagnosticRevision {
    handle: DiagnosticSnapshotHandle,
    revision: u64,
    source_fingerprint: [u8; 32],
}

impl DiagnosticRevision {
    /// Returns the opaque session-bound handle.
    #[must_use]
    pub(crate) const fn handle(&self) -> DiagnosticSnapshotHandle {
        self.handle
    }

    /// Returns the monotonic nonzero revision.
    #[must_use]
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the exact source-closure fingerprint specified by tooling snapshot v1.
    #[must_use]
    pub(crate) const fn source_fingerprint(&self) -> [u8; 32] {
        self.source_fingerprint
    }
}

/// Admission failures that cannot be represented as a correlated query response.
#[derive(Debug)]
pub(crate) enum DiagnosticSessionError {
    /// The process-local session identity space was exhausted.
    SessionIdentityExhausted,
    /// The exact JavaScript-safe revision space was exhausted.
    RevisionExhausted,
    /// The retained revision alone exceeds the 64 MiB cache charge.
    SessionCacheExhausted,
    /// Structured diagnostic v2 rejected producer input or source authority.
    Diagnostics(protocol_v2::ProtocolError),
    /// A previously verified source map could not be enumerated consistently.
    SourceInvariant,
}

impl fmt::Display for DiagnosticSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SessionIdentityExhausted => "diagnostic session identity space is exhausted",
            Self::RevisionExhausted => "diagnostic session revision space is exhausted",
            Self::SessionCacheExhausted => "diagnostic session cache limit exceeded",
            Self::Diagnostics(_) => "structured diagnostics rejected the retained source",
            Self::SourceInvariant => "retained source map could not be enumerated",
        })
    }
}

impl std::error::Error for DiagnosticSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Diagnostics(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct RevisionRecord {
    description: DiagnosticRevision,
    sources: SourceMap,
    report: Option<Arc<str>>,
    cache_bytes: usize,
}

#[derive(Clone, Debug)]
struct Correlation {
    query_version: u64,
    request_id: String,
    snapshot: String,
    revision: u64,
}

/// One admitted diagnostics request awaiting bounded completion.
#[derive(Debug)]
pub(crate) struct PendingDiagnosticQuery {
    session: u64,
    correlation: Correlation,
    revision: DiagnosticRevision,
    record: Weak<RevisionRecord>,
    source_identity: SourceMapIdentity,
    work_limit: u64,
    deadline: Instant,
    state: Arc<AtomicU8>,
}

#[derive(Debug)]
struct RequestSlot {
    state: Arc<AtomicU8>,
    deadline: Instant,
}

/// One compiler-host-owned collection of immutable diagnostic revisions.
#[derive(Debug)]
pub(crate) struct DiagnosticSession {
    session: u64,
    next_revision: u64,
    retained: VecDeque<Arc<RevisionRecord>>,
    cache_bytes: usize,
    in_flight: BTreeMap<String, RequestSlot>,
}

impl DiagnosticSession {
    /// Creates a fresh session whose handles cannot be reused by another live session.
    ///
    /// # Errors
    ///
    /// Returns an error if the process-local identity counter is exhausted.
    pub(crate) fn try_new() -> Result<Self, DiagnosticSessionError> {
        let session = NEXT_SESSION_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| current.checked_add(1))
            .map_err(|_| DiagnosticSessionError::SessionIdentityExhausted)?;
        Ok(Self {
            session,
            next_revision: 1,
            retained: VecDeque::new(),
            cache_bytes: 0,
            in_flight: BTreeMap::new(),
        })
    }

    /// Admits an immutable source revision with a complete v2 diagnostic pass.
    ///
    /// # Errors
    ///
    /// Rejects foreign spans, invalid diagnostic records, revision exhaustion, source invariant
    /// failure, or a first revision whose deterministic cache charge exceeds 64 MiB.
    pub(crate) fn admit_diagnostics(
        &mut self,
        sources: SourceMap,
        diagnostics: &[Diagnostic],
    ) -> Result<DiagnosticRevision, DiagnosticSessionError> {
        let report = protocol_v2::render_json(diagnostics, &sources)
            .map_err(DiagnosticSessionError::Diagnostics)?;
        self.admit(sources, Some(report.into()))
    }

    /// Admits source authority whose diagnostic pass is not ready yet.
    ///
    /// # Errors
    ///
    /// Rejects revision exhaustion, source invariant failure, or a cache charge over 64 MiB.
    pub(crate) fn admit_unready(
        &mut self,
        sources: SourceMap,
    ) -> Result<DiagnosticRevision, DiagnosticSessionError> {
        self.admit(sources, None)
    }

    /// Returns the currently active revision, when one has been admitted.
    #[must_use]
    pub(crate) fn active_revision(&self) -> Option<DiagnosticRevision> {
        self.retained.back().map(|record| record.description)
    }

    /// Returns the number of immutable revisions retained for bounded incremental reuse.
    #[must_use]
    pub(crate) fn retained_revisions(&self) -> usize {
        self.retained.len()
    }

    /// Returns the exact variable-byte cache charge for retained paths, source, and v2 reports.
    #[must_use]
    pub(crate) const fn cache_bytes(&self) -> usize {
        self.cache_bytes
    }

    fn admit(
        &mut self,
        sources: SourceMap,
        report: Option<Arc<str>>,
    ) -> Result<DiagnosticRevision, DiagnosticSessionError> {
        if self.next_revision > MAX_REVISION {
            return Err(DiagnosticSessionError::RevisionExhausted);
        }
        let (source_fingerprint, source_bytes) = source_fingerprint_and_charge(&sources)?;
        let cache_bytes =
            checked_cache_charge(source_bytes, report.as_ref().map_or(0, |value| value.len()))?;
        let description = DiagnosticRevision {
            handle: DiagnosticSnapshotHandle { session: self.session, serial: self.next_revision },
            revision: self.next_revision,
            source_fingerprint,
        };
        let record = Arc::new(RevisionRecord { description, sources, report, cache_bytes });
        let mut projected_cache = self
            .cache_bytes
            .checked_add(cache_bytes)
            .ok_or(DiagnosticSessionError::SessionCacheExhausted)?;
        let mut evictions = 0_usize;
        while self.retained.len() + 1 - evictions > MAX_RETAINED_REVISIONS
            || projected_cache > MAX_SESSION_CACHE_BYTES
        {
            let evicted =
                self.retained.get(evictions).ok_or(DiagnosticSessionError::SourceInvariant)?;
            projected_cache = projected_cache
                .checked_sub(evicted.cache_bytes)
                .ok_or(DiagnosticSessionError::SourceInvariant)?;
            evictions += 1;
        }
        self.mark_replaced();
        self.next_revision += 1;
        for _ in 0..evictions {
            let _ = self.retained.pop_front();
        }
        self.retained.push_back(record);
        self.cache_bytes = projected_cache;
        Ok(description)
    }

    fn mark_replaced(&mut self) {
        self.in_flight.clear();
    }
}

#[cfg(test)]
mod tests;
