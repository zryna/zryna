//! Exact diagnostic and recovery parity at the untrusted candidate boundary.

use serde_json::{Value, json};
#[path = "m7_fuzz_witness/parser.rs"]
mod parser_witness;
#[path = "m7_fuzz_witness/support.rs"]
mod witness;
use zryna_frontend::{
    native_lexer::lex,
    native_parser::{
        parse_v2_recovering_candidate, v3::parse_v3_straight_line_candidate, v4::parse_v4_candidate,
    },
    syntax_v2, syntax_v3, syntax_v4,
};
use zryna_source::{SourceFileInput, SourceMap};

#[path = "native_parser_parity/corpus.rs"]
mod corpus;
#[path = "native_parser_parity/projects.rs"]
mod projects;
#[path = "native_parser_parity/provider.rs"]
mod provider;
#[path = "native_parser_parity/resources.rs"]
mod resources;

#[test]
fn frozen_project_resources_reject_first_extra_atomically() {
    let receipts: Vec<Value> =
        serde_json::from_str(include_str!("native_parser_parity/project-rejections.json"))
            .expect("frozen project receipts");
    assert_eq!(receipts.len(), 12, "complete project first-extra inventory");
    for receipt in &receipts {
        let version =
            u32::try_from(receipt["version"].as_u64().expect("version")).expect("version range");
        let (_, files) = projects::cases(version)
            .into_iter()
            .find(|(name, _)| receipt["name"] == *name)
            .expect("project boundary source");
        let sources = SourceMap::build(
            files.into_iter().map(|(path, text)| SourceFileInput { path, text }).collect(),
        )
        .expect("bounded project authority");
        let lexed = lex(&sources).expect("reachable project lexical budget");
        assert_eq!(
            candidate(version, &sources, &lexed),
            receipt["response"],
            "v{version}-{}: project first extra",
            receipt["name"]
        );
    }
}

#[test]
#[ignore = "requires pinned worker and complete multi-file production inventories"]
fn pinned_provider_matches_project_resource_boundaries() {
    let mut failures = Vec::new();
    let mut count = 0;
    for version in 2..=4 {
        for (name, files) in projects::cases(version) {
            count += 1;
            eprintln!("checking v{version}-{name}");
            let worker = provider::project_response(version, &files);
            if worker.get("error").is_some() {
                eprintln!("receipt v{version}-{name}: {worker}");
            }
            let sources = SourceMap::build(
                files.into_iter().map(|(path, text)| SourceFileInput { path, text }).collect(),
            )
            .expect("bounded project");
            let lexed = lex(&sources).expect("reachable project lexical budget");
            let native = candidate(version, &sources, &lexed);
            if native != worker {
                failures.push(format!(
                    "v{version}-{name}: native {} worker {}",
                    native.get("error").unwrap_or(&Value::Null),
                    worker.get("error").unwrap_or(&Value::Null)
                ));
            }
        }
    }
    assert_eq!(count, 24, "complete reachable project boundary inventory");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn frozen_production_resources_reject_first_extra_atomically() {
    let receipts: Value =
        serde_json::from_str(include_str!("native_parser_parity/resource-rejections.json"))
            .expect("frozen production resource receipts");
    let receipts = receipts.as_array().expect("resource receipts");
    assert_eq!(receipts.len(), 29, "complete first-extra inventory");
    for receipt in receipts {
        let version =
            u32::try_from(receipt["version"].as_u64().expect("version")).expect("bounded version");
        let (_, text) = resources::cases(version)
            .into_iter()
            .find(|(name, _)| receipt["name"] == *name)
            .expect("resource source");
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text }])
                .expect("bounded source");
        let lexed = lex(&sources).expect("production lexical budget");
        assert_eq!(
            candidate(version, &sources, &lexed),
            receipt["response"],
            "v{version}-{}: exact fatal rejection",
            receipt["name"]
        );
    }
}

#[test]
fn frozen_locals_preserve_statement_limit_priority() {
    let receipts: Vec<Value> =
        serde_json::from_str(include_str!("native_parser_parity/resource-rejections.json"))
            .expect("pinned resource receipts");
    let mut count = 0;
    for receipt in receipts.iter().filter(|receipt| receipt["name"] == "locals-4097") {
        count += 1;
        let version =
            u32::try_from(receipt["version"].as_u64().expect("version")).expect("version range");
        let (_, text) = resources::cases(version)
            .into_iter()
            .find(|(name, _)| name == "locals-4097")
            .expect("local boundary source");
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text }])
                .expect("source authority");
        let lexed = lex(&sources).expect("lexical budget");
        assert_eq!(
            candidate(version, &sources, &lexed),
            receipt["response"],
            "v{version}: statement slot precedes local allocation"
        );
    }
    assert_eq!(count, 2, "both structured protocols");
}

#[test]
#[ignore = "requires pinned worker and production-limit snapshots"]
fn pinned_provider_matches_production_resource_boundaries() {
    let mut failures = Vec::new();
    for version in 2..=4 {
        for (name, text) in resources::cases(version) {
            eprintln!("checking v{version}-{name}");
            let worker = provider::responses(version, std::slice::from_ref(&text)).remove(0);
            let sources =
                SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text }])
                    .expect("bounded source");
            let lexed = lex(&sources).expect("production lexical budget");
            let native = candidate(version, &sources, &lexed);
            if native != worker {
                let describe = |response: &Value| {
                    response.get("error").cloned().unwrap_or_else(|| json!({"candidate":true}))
                };
                failures.push(format!(
                    "v{version}-{name}: native {} worker {}",
                    describe(&native),
                    describe(&worker)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "requires the exact pinned TypeScript provider"]
fn pinned_provider_matches_frozen_diagnostic_corpus() {
    corpus::pinned_provider_matches_frozen_diagnostic_corpus();
}

fn rejection(diagnostic: &zryna_diagnostics::Diagnostic) -> Value {
    let span = diagnostic.primary_span();
    json!({"error": {
        "code": diagnostic.code(),
        "message": diagnostic.message(),
        "span": span.map(|span| json!({
            "file": span.file().index(), "start": span.start(), "end": span.end(),
        })),
    }})
}

fn candidate(
    version: u32,
    sources: &SourceMap,
    lexed: &zryna_frontend::native_lexer::LexedProject,
) -> Value {
    match version {
        2 => match parse_v2_recovering_candidate(sources, lexed) {
            Ok(raw) => {
                let value = serde_json::to_value(&raw).expect("v2 JSON");
                syntax_v2::verify_snapshot(raw, sources).expect("v2 verifier");
                json!({"result": value})
            }
            Err(error) => rejection(error.diagnostic()),
        },
        3 => match parse_v3_straight_line_candidate(sources, lexed) {
            Ok(raw) => {
                let value = serde_json::to_value(&raw).expect("v3 JSON");
                syntax_v3::verify_snapshot(raw, sources).expect("v3 verifier");
                json!({"result": value})
            }
            Err(error) => rejection(error.diagnostic()),
        },
        4 => match parse_v4_candidate(sources, lexed) {
            Ok(raw) => {
                let value = serde_json::to_value(&raw).expect("v4 JSON");
                syntax_v4::verify_snapshot(raw, sources).expect("v4 verifier");
                json!({"result": value})
            }
            Err(error) => rejection(error.diagnostic()),
        },
        _ => unreachable!(),
    }
}

#[path = "native_parser_parity/generated_property.rs"]
mod generated_property;

#[test]
fn arbitrary_bounded_token_grammar_is_deterministic_and_never_forges_a_snapshot() {
    generated_property::run();
}
