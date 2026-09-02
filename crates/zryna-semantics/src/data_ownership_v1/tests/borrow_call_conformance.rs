use super::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

const CONTRACT: &str = include_str!("../../../../../tests/m3-contract-v1.json");

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_owned()
}

fn borrow_call_contract() -> Value {
    serde_json::from_str::<Value>(CONTRACT).expect("canonical M3 contract")["borrowCallConformance"]
        .clone()
}

fn records<'a>(section: &'a Value, name: &str) -> &'a [Value] {
    section[name].as_array().expect("validated contract array")
}

fn text<'a>(record: &'a Value, name: &str) -> &'a str {
    record[name].as_str().expect("validated contract text")
}

fn fixture_paths(record: &Value) -> (&str, &str) {
    (text(record, "source"), text(record, "snapshot"))
}

fn authenticated(record: &Value) -> (SourceMap, zryna_syntax::v4::ProjectSyntaxSnapshot) {
    let (source_path, snapshot_path) = fixture_paths(record);
    let source = std::fs::read_to_string(workspace_root().join(source_path))
        .unwrap_or_else(|error| panic!("{source_path}: {error}"));
    let snapshot = std::fs::read(workspace_root().join(snapshot_path))
        .unwrap_or_else(|error| panic!("{snapshot_path}: {error}"));
    let sources = sources_for(&source);
    let raw = decode_snapshot(&snapshot).expect("canonical protocol-v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful protocol-v4 snapshot");
    (sources, syntax)
}

fn lowered_verified_data(
    sources: &SourceMap,
    syntax: &zryna_syntax::v4::ProjectSyntaxSnapshot,
) -> String {
    let program = lower(pair_input(syntax, sources)).expect("accepted fixture");
    format!("{program:#?}")
}

type DiagnosticTuple = (String, String, String, Option<(String, u32, u32)>);

fn rejection(record: &Value) -> Vec<DiagnosticTuple> {
    let fixture_id = text(record, "id");
    let (sources, syntax) = authenticated(record);
    let Err(diagnostics) = lower(pair_input(&syntax, &sources)) else {
        panic!("{fixture_id} unexpectedly lowered");
    };
    diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code().to_owned(),
                diagnostic.message().to_owned(),
                diagnostic.guidance().to_owned(),
                diagnostic.primary_span().map(|span| {
                    (
                        sources
                            .source(span.file())
                            .expect("diagnostic source")
                            .path()
                            .as_str()
                            .to_owned(),
                        span.start(),
                        span.end(),
                    )
                }),
            )
        })
        .collect()
}

#[test]
fn accepted_borrow_call_fixture_snapshots_authenticate_and_lower() {
    let section = borrow_call_contract();
    for fixture in records(&section, "acceptedCases") {
        let (sources, syntax) = authenticated(fixture);
        lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|diagnostics| panic!("{}: {diagnostics:?}", text(fixture, "id")));
    }
}

#[test]
fn rejected_borrow_call_fixtures_freeze_diagnostics_spans_and_recovery() {
    let section = borrow_call_contract();
    let accepted = records(&section, "acceptedCases");
    let accepted_inputs = accepted
        .iter()
        .map(|fixture| (text(fixture, "id"), authenticated(fixture)))
        .collect::<Vec<_>>();
    let pristine = accepted_inputs
        .iter()
        .map(|(id, (sources, syntax))| (*id, lowered_verified_data(sources, syntax)))
        .collect::<Vec<_>>();
    for fixture in records(&section, "exclusions") {
        let fixture_id = text(fixture, "id");
        let first = rejection(fixture);
        let expected = records(fixture, "diagnostics");
        assert_eq!(first.len(), expected.len(), "{fixture_id}");
        for (observed, expected) in first.iter().zip(expected) {
            let span = &expected["span"];
            assert_eq!(
                observed,
                &(
                    text(expected, "code").to_owned(),
                    text(expected, "message").to_owned(),
                    text(expected, "guidance").to_owned(),
                    Some((
                        text(span, "path").to_owned(),
                        u32::try_from(span["start"].as_u64().expect("validated span start"))
                            .expect("validated u32 span start"),
                        u32::try_from(span["end"].as_u64().expect("validated span end"))
                            .expect("validated u32 span end"),
                    )),
                ),
                "{fixture_id}"
            );
        }

        let recovery = &fixture["recovery"];
        assert_eq!(text(recovery, "expectation"), "same-verified-program");
        let accepted_fixture = text(recovery, "acceptedFixture");
        let (_, (recovery_sources, recovery_syntax)) = accepted_inputs
            .iter()
            .find(|(id, _)| *id == accepted_fixture)
            .expect("registered accepted recovery fixture");
        let baseline = pristine
            .iter()
            .find_map(|(id, data)| (*id == accepted_fixture).then_some(data))
            .expect("pristine accepted recovery data");
        assert_eq!(
            &lowered_verified_data(recovery_sources, recovery_syntax),
            baseline,
            "{fixture_id} contaminated deterministic recovery"
        );
        assert_eq!(rejection(fixture), first, "{fixture_id} rejection replay drifted");
        assert_eq!(
            &lowered_verified_data(recovery_sources, recovery_syntax),
            baseline,
            "{fixture_id} replay contaminated deterministic recovery"
        );
    }
}
