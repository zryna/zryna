use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    coordinates::{PositionEncoding, byte_range_to_positions},
    documents::Document,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    schema_version: u32,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    code: String,
    severity: Severity,
    location: Location,
    message: String,
    guidance: String,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Location {
    Global,
    WorkspacePath { path: String },
    Source { path: String, byte_start: u32, byte_end: u32 },
}

pub(crate) fn decode_report(value: &Value) -> Option<Value> {
    let report = value.get("result")?.get("report")?.clone();
    let decoded: Report = serde_json::from_value(report.clone()).ok()?;
    (decoded.schema_version == 2).then_some(report)
}

pub(crate) fn standard_diagnostics(
    report: &Value,
    documents: &BTreeMap<String, Document>,
    encoding: PositionEncoding,
) -> Option<Vec<(String, i64, Vec<Value>)>> {
    let report: Report = serde_json::from_value(report.clone()).ok()?;
    if report.schema_version != 2 {
        return None;
    }
    let paths = documents
        .iter()
        .map(|(uri, document)| (document.path.as_str(), (uri.as_str(), document)))
        .collect::<BTreeMap<_, _>>();
    let mut output = documents
        .iter()
        .map(|(uri, document)| (uri.clone(), document.version, Vec::new()))
        .collect::<Vec<_>>();
    let indices = output
        .iter()
        .enumerate()
        .map(|(index, (uri, _, _))| (uri.clone(), index))
        .collect::<BTreeMap<_, _>>();
    for diagnostic in report.diagnostics {
        let Location::Source { path, byte_start, byte_end } = &diagnostic.location else {
            continue;
        };
        let (uri, document) = paths.get(path.as_str())?;
        let range = byte_range_to_positions(&document.text, *byte_start, *byte_end, encoding)?;
        let index = *indices.get(*uri)?;
        output[index].2.push(json!({
            "range": range,
            "severity": match diagnostic.severity { Severity::Error => 1, Severity::Warning => 2 },
            "code": diagnostic.code,
            "source": "zryna",
            "message": diagnostic.message,
            "data": {"guidance":diagnostic.guidance,"structuredLocation":diagnostic.location}
        }));
    }
    Some(output)
}
