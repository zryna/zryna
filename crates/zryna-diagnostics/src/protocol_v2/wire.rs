//! Private closed transport DTOs; never compiler authority.

use serde::{Deserialize, Serialize};

use super::{EXHAUSTION_CODE, MAX_PATH_BYTES, MAX_TEXT_BYTES};
use crate::Severity;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Report {
    pub schema_version: u32,
    pub diagnostics: Vec<Record>,
}

impl Report {
    pub(super) fn exhausted() -> Self {
        Self {
            schema_version: 2,
            diagnostics: vec![Record {
                code: EXHAUSTION_CODE.to_owned(),
                severity: Severity::Error,
                location: Location::Global {},
                message: "structured diagnostic limit exceeded".to_owned(),
                guidance: "reduce the diagnostic input and retry; this report is incomplete"
                    .to_owned(),
            }],
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub code: String,
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    pub guidance: String,
}

type SortKey<'a> = (Option<&'a str>, Option<u32>, Option<u32>, Severity, &'a str, &'a str, &'a str);

impl Record {
    pub(super) fn exceeds_limits(&self) -> bool {
        self.message.len() > MAX_TEXT_BYTES
            || self.guidance.len() > MAX_TEXT_BYTES
            || self.key().0.is_some_and(|path| path.len() > MAX_PATH_BYTES)
    }

    pub(super) fn key(&self) -> SortKey<'_> {
        let (path, start, end) = match &self.location {
            Location::Global {} => (None, None, None),
            Location::WorkspacePath { path } => (Some(path.as_str()), None, None),
            Location::Source { path, byte_start, byte_end } => {
                (Some(path.as_str()), Some(*byte_start), Some(*byte_end))
            }
        };
        (path, start, end, self.severity, &self.code, &self.message, &self.guidance)
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Location {
    Global {},
    WorkspacePath { path: String },
    Source { path: String, byte_start: u32, byte_end: u32 },
}
