use super::{Correlation, MAX_RESPONSE_BYTES};

/// Closed status vocabulary for implemented internal query slices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryStatus {
    /// The complete retained diagnostic report or definition result was published.
    Ok,
    /// No supported symbol occupies the requested position.
    Absent,
    /// The snapshot/revision pair is not the active authority.
    Stale,
    /// The request does not have the closed v1 shape.
    Malformed,
    /// The requested version or method is not implemented by this slice.
    Unsupported,
    /// Explicit cancellation or the monotonic deadline stopped the request.
    Cancelled,
    /// The retained revision has no completed diagnostic view.
    Unavailable,
    /// A request, response, work, queue, or cache boundary was exceeded.
    OverBudget,
}

impl QueryStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Absent => "absent",
            Self::Stale => "stale",
            Self::Malformed => "malformed",
            Self::Unsupported => "unsupported",
            Self::Cancelled => "cancelled",
            Self::Unavailable => "unavailable",
            Self::OverBudget => "over_budget",
        }
    }
}

/// Closed reason vocabulary for internal query-session outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryReason {
    /// The encoded request exceeds 65,536 bytes.
    RequestBytes,
    /// The request nesting exceeds 64 levels.
    RequestDepth,
    /// The closed request record is malformed.
    Shape,
    /// The query-contract version is unsupported.
    Version,
    /// The snapshot/revision pair is stale or foreign.
    Snapshot,
    /// The method is outside the selected internal slice.
    Method,
    /// Method parameters do not have the selected closed shape.
    Params,
    /// A path, byte offset, or retained span is outside the source authority.
    Source,
    /// No supported semantic symbol occupies the requested byte.
    Symbol,
    /// The request was explicitly cancelled.
    Request,
    /// The request exceeded its monotonic deadline.
    Deadline,
    /// The required diagnostic or semantic view is not ready or has invalid authority.
    Analysis,
    /// The canonical logical work charge exceeds the requested limit.
    Work,
    /// The complete response exceeds 1,048,576 bytes.
    ResponseBytes,
    /// Session cache or queue admission was exhausted.
    Session,
}

impl QueryReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RequestBytes => "request_bytes",
            Self::RequestDepth => "request_depth",
            Self::Shape => "shape",
            Self::Version => "version",
            Self::Snapshot => "snapshot",
            Self::Method => "method",
            Self::Params => "params",
            Self::Source => "source",
            Self::Symbol => "symbol",
            Self::Request => "request",
            Self::Deadline => "deadline",
            Self::Analysis => "analysis",
            Self::Work => "work",
            Self::ResponseBytes => "response_bytes",
            Self::Session => "session",
        }
    }
}

/// One complete query response, or an uncorrelated admission rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticQueryResponse {
    status: QueryStatus,
    reason: Option<QueryReason>,
    encoded: Option<String>,
}

impl DiagnosticQueryResponse {
    /// Returns the closed response status.
    #[must_use]
    pub const fn status(&self) -> QueryStatus {
        self.status
    }

    /// Returns the closed failure reason, absent only for success.
    #[must_use]
    pub const fn reason(&self) -> Option<QueryReason> {
        self.reason
    }

    /// Returns canonical JSON when correlation fields were safely decoded.
    #[must_use]
    pub fn encoded(&self) -> Option<&str> {
        self.encoded.as_deref()
    }

    pub(super) fn uncorrelated(status: QueryStatus, reason: QueryReason) -> Self {
        Self { status, reason: Some(reason), encoded: None }
    }

    pub(super) fn failure(
        correlation: &Correlation,
        status: QueryStatus,
        reason: QueryReason,
    ) -> Self {
        Self { status, reason: Some(reason), encoded: encode_failure(correlation, status, reason) }
    }

    pub(super) fn success(correlation: &Correlation, report: &str) -> Self {
        match encode_success(correlation, report) {
            Some(encoded) => Self { status: QueryStatus::Ok, reason: None, encoded: Some(encoded) },
            None => Self::failure(correlation, QueryStatus::OverBudget, QueryReason::ResponseBytes),
        }
    }

    pub(super) fn definition_success(
        correlation: &Correlation,
        path: &str,
        byte_start: u32,
        byte_end: u32,
    ) -> Self {
        let Ok(path) = serde_json::to_string(path) else {
            return Self::failure(correlation, QueryStatus::Unavailable, QueryReason::Analysis);
        };
        let result = format!(
            "{{\"locations\":[{{\"path\":{path},\"byte_start\":{byte_start},\"byte_end\":{byte_end}}}]}}"
        );
        match encode_result(correlation, &result) {
            Some(encoded) => Self { status: QueryStatus::Ok, reason: None, encoded: Some(encoded) },
            None => Self::failure(correlation, QueryStatus::OverBudget, QueryReason::ResponseBytes),
        }
    }
}

fn prefix(correlation: &Correlation) -> Option<String> {
    let request_id = serde_json::to_string(&correlation.request_id).ok()?;
    let snapshot = serde_json::to_string(&correlation.snapshot).ok()?;
    Some(format!(
        "{{\"query_version\":{},\"request_id\":{request_id},\"snapshot\":{snapshot},\"revision\":{}",
        correlation.query_version, correlation.revision
    ))
}

fn encode_failure(
    correlation: &Correlation,
    status: QueryStatus,
    reason: QueryReason,
) -> Option<String> {
    Some(format!(
        "{},\"status\":\"{}\",\"reason\":\"{}\"}}",
        prefix(correlation)?,
        status.as_str(),
        reason.as_str()
    ))
}

pub(super) fn encode_success(correlation: &Correlation, report: &str) -> Option<String> {
    encode_result(correlation, &format!("{{\"report\":{report}}}"))
}

fn encode_result(correlation: &Correlation, result: &str) -> Option<String> {
    let encoded = format!("{},\"status\":\"ok\",\"result\":{result}}}", prefix(correlation)?);
    (encoded.len() <= MAX_RESPONSE_BYTES).then_some(encoded)
}
