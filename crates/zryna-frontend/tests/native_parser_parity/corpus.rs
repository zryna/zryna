use super::*;

fn frozen_cases() -> Vec<Value> {
    let mut cases = [
        include_str!("corpus.json"),
        include_str!("depth-corpus.json"),
        include_str!("project-corpus.json"),
        include_str!("call-depth-corpus.json"),
        include_str!("priority-corpus.json"),
        include_str!("priority-project-corpus.json"),
        include_str!("count-order-corpus.json"),
        include_str!("review-corpus.json"),
    ]
    .into_iter()
    .flat_map(|text| serde_json::from_str::<Vec<Value>>(text).expect("frozen corpus"))
    .collect::<Vec<_>>();
    // One frozen response serves the two independently retained diagnostic-limit sources.
    let response = cases
        .iter()
        .find(|case| case["name"] == "v2-parse-diagnostic-limit-256")
        .expect("frozen canonical diagnostic-limit case")
        .get("response")
        .expect("frozen canonical diagnostic-limit response")
        .clone();
    let duplicate = cases
        .iter_mut()
        .find(|case| case["name"] == "v2-parse-diagnostic-limit-257")
        .expect("frozen duplicate diagnostic-limit case");
    let reference = duplicate
        .as_object_mut()
        .expect("frozen diagnostic-limit object")
        .remove("response_ref")
        .expect("explicit frozen diagnostic-limit reference");
    assert_eq!(reference, "v2-parse-diagnostic-limit-256");
    assert!(duplicate.get("response").is_none(), "one frozen response representation");
    duplicate["response"] = response;
    let mut names = std::collections::BTreeSet::new();
    for case in &cases {
        assert!(case.get("response_ref").is_none(), "no unresolved frozen response reference");
        assert!(
            names.insert(case["name"].as_str().expect("stable case identity")),
            "duplicate corpus identity"
        );
    }
    assert_eq!(cases.len(), 428, "complete combined frozen inventory");
    cases
}

pub(super) fn pinned_provider_matches_frozen_diagnostic_corpus() {
    let cases = frozen_cases();
    assert_eq!(cases.len(), 428, "complete frozen diagnostic inventory");
    for version in 2..=4 {
        let selected = cases
            .iter()
            .filter(|case| case["version"] == version && case.get("text").is_some())
            .collect::<Vec<_>>();
        let texts = selected
            .iter()
            .map(|case| case["text"].as_str().expect("source").to_owned())
            .collect::<Vec<_>>();
        let responses = provider::responses(version, &texts);
        for (case, response) in selected.into_iter().zip(responses) {
            assert_eq!(response, case["response"], "{}: exact pinned receipt", case["name"]);
        }
    }
    for case in cases.iter().filter(|case| case.get("files").is_some()) {
        let version =
            u32::try_from(case["version"].as_u64().expect("version")).expect("version range");
        let files =
            case_sources(case).into_iter().map(|file| (file.path, file.text)).collect::<Vec<_>>();
        assert_eq!(
            provider::project_response(version, &files),
            case["response"],
            "{}: pinned multi-file receipt",
            case["name"]
        );
    }
    frozen_recovery_and_rejections_match_exact_provider_diagnostics();
}

#[test]
fn frozen_recovery_and_rejections_match_exact_provider_diagnostics() {
    let cases = frozen_cases();
    let mut failures = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().expect("case identity");
        let sources = SourceMap::build(case_sources(case)).expect("source authority");
        let lexed = lex(&sources).expect("bounded tokens");
        let version =
            u32::try_from(case["version"].as_u64().expect("protocol")).expect("version range");
        let native = candidate(version, &sources, &lexed);
        if native != case["response"] {
            failures.push(format!("{name}:\nnative {native}\nworker {}", case["response"]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn case_sources(case: &Value) -> Vec<SourceFileInput> {
    if let Some(files) = case.get("files") {
        files
            .as_array()
            .expect("case file inventory")
            .iter()
            .map(|file| SourceFileInput {
                path: file["path"].as_str().expect("file path").to_owned(),
                text: file["text"].as_str().expect("file source").to_owned(),
            })
            .collect()
    } else {
        vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: case["text"].as_str().expect("case source").to_owned(),
        }]
    }
}
