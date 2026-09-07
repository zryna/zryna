use super::{Correlation, MAX_RESPONSE_BYTES};

/// Closed status vocabulary for the internal diagnostics-only query slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryStatus {
    /// The complete retained diagnostic report was published.
    Ok,
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
            Self::Stale => "stale",
            Self::Malformed => "malformed",
            Self::Unsupported => "unsupported",
            Self::Cancelled => "cancelled",
            Self::Unavailable => "unavailable",
            Self::OverBudget => "over_budget",
        }
    }
}

/// Closed reason vocabulary for diagnostics-session outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryReason {
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
    /// The method is outside this diagnostics-only slice.
    Method,
    /// Diagnostic parameters are not the exact empty object.
    Params,
    /// The request was explicitly cancelled.
    Request,
    /// The request exceeded its monotonic deadline.
    Deadline,
    /// The diagnostic pass is not ready or its retained authority is invalid.
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
pub(crate) struct DiagnosticQueryResponse {
    status: QueryStatus,
    reason: Option<QueryReason>,
    encoded: Option<String>,
}

impl DiagnosticQueryResponse {
    /// Returns the closed response status.
    #[must_use]
    pub(crate) const fn status(&self) -> QueryStatus {
        self.status
    }

    /// Returns the closed failure reason, absent only for success.
    #[must_use]
    pub(crate) const fn reason(&self) -> Option<QueryReason> {
        self.reason
    }

    /// Returns canonical JSON when correlation fields were safely decoded.
    #[must_use]
    pub(crate) fn encoded(&self) -> Option<&str> {
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
    let encoded =
        format!("{},\"status\":\"ok\",\"result\":{{\"report\":{report}}}}}", prefix(correlation)?);
    (encoded.len() <= MAX_RESPONSE_BYTES).then_some(encoded)
}
