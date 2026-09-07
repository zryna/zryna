use super::*;
use serde_json::{Value, json};
use zryna_source::SourceFileInput;

const GOLDEN: &str = include_str!("fixtures/golden.json");
const TERMINAL: &str = include_str!("fixtures/exhausted.json");
const HOSTILE: &str = include_str!("fixtures/hostile.json");

fn sources() -> SourceMap {
    let input: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/sources.json")).expect("source fixture");
    SourceMap::build(
        input
            .iter()
            .map(|source| SourceFileInput {
                path: source["path"].as_str().expect("path").to_owned(),
                text: source["text"].as_str().expect("text").to_owned(),
            })
            .collect(),
    )
    .expect("source map")
}

fn error() -> Diagnostic {
    Diagnostic::error("ZRYNA-T1001", None, "message", "guidance")
}

fn value() -> Value {
    serde_json::from_str(GOLDEN).expect("golden")
}

fn validate(value: &Value, sources: &SourceMap) -> Result<(), ProtocolError> {
    validate_json(&serde_json::to_vec(value).expect("serialize fixture"), sources)
}

#[test]
fn shared_golden_preserves_source_ownership_and_v1() {
    let sources = sources();
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let file = sources.file_id(&path).expect("file");
    let mut input = vec![
        Diagnostic::warning("ZRYNA-G1001", None, "global", "review"),
        Diagnostic::error(
            "ZRYNA-A1001",
            Some("Cargo.toml".to_owned()),
            "workspace",
            "fix workspace",
        ),
        Diagnostic::error_at(
            "ZRYNA-T1001",
            sources.span(file, 4, 4).expect("span"),
            "bad\nvalue",
            "choose an i32",
        ),
        Diagnostic::warning_at("ZRYNA-T1002", sources.span(file, 6, 8).expect("span"), "é", "😀"),
    ];
    let old_text = crate::render_text(&input, &sources).expect("text");
    let old_json = crate::render_json(&input, &sources).expect("v1");
    for _ in 0..input.len() {
        assert_eq!(render_json(&input, &sources).expect("render"), GOLDEN.trim_end());
        input.rotate_left(1);
    }
    assert_eq!(crate::render_text(&input, &sources).expect("text"), old_text);
    assert_eq!(crate::render_json(&input, &sources).expect("v1"), old_json);
    assert_eq!(validate_json(GOLDEN.as_bytes(), &sources), Ok(()));
    assert_eq!(validate_json(TERMINAL.as_bytes(), &sources), Ok(()));
    assert_eq!(render_json(&input, &self::sources()), Err(ProtocolError::Source));
    assert_eq!(
        validate_json(GOLDEN.as_bytes(), &SourceMap::build(vec![]).expect("empty")),
        Err(ProtocolError::Source)
    );
}

#[test]
fn independent_hostile_fixture_rejections_are_stable() {
    let sources = sources();
    let fixtures: Vec<Value> = serde_json::from_str(HOSTILE).expect("hostile fixture");
    for fixture in fixtures {
        let mut document = value();
        let pointer = fixture["pointer"].as_str().expect("pointer");
        let (parent, key) = pointer.rsplit_once('/').expect("field pointer");
        let object = document.pointer_mut(parent).expect("parent").as_object_mut().expect("object");
        if fixture["remove"] == true {
            object.remove(key);
        } else {
            object.insert(key.to_owned(), fixture["value"].clone());
        }
        let expected = fixture["error"].as_str().expect("error");
        for _ in 0..2 {
            assert_eq!(
                format!(
                    "{:?}",
                    validate(&document, &sources)
                        .expect_err(fixture["name"].as_str().expect("name"))
                ),
                expected,
                "{}",
                fixture["name"]
            );
        }
    }
}

#[test]
fn duplicate_fields_invalid_encoding_and_trailing_documents_reject() {
    let sources = sources();
    for input in [
        GOLDEN.replacen("\"schema_version\":2", "\"schema_version\":2,\"schema_version\":2", 1),
        GOLDEN.replacen(
            "\"severity\":\"warning\"",
            "\"severity\":\"warning\",\"severity\":\"warning\"",
            1,
        ),
        GOLDEN.replacen("\"kind\":\"global\"", "\"kind\":\"global\",\"kind\":\"global\"", 1),
        GOLDEN.replacen(
            "\"path\":\"Cargo.toml\"",
            "\"path\":\"Cargo.toml\",\"path\":\"Cargo.toml\"",
            1,
        ),
        GOLDEN.replacen("\"global\"", "\"\\ud800\"", 1),
        format!("{GOLDEN}{{}}"),
        GOLDEN[..GOLDEN.len() - 3].to_owned(),
    ] {
        assert_eq!(validate_json(input.as_bytes(), &sources), Err(ProtocolError::Shape));
    }
    let mut invalid_utf8 = GOLDEN.as_bytes().to_vec();
    invalid_utf8[0] = 0xff;
    assert_eq!(validate_json(&invalid_utf8, &sources), Err(ProtocolError::Shape));
}

#[test]
fn exact_and_first_extra_record_count_and_terminal_recovery() {
    let sources = sources();
    let mut input = vec![error(); MAX_DIAGNOSTICS];
    let exact = render_json(&input, &sources).expect("exact");
    assert!(!exact.contains(EXHAUSTION_CODE));
    assert_eq!(validate_json(exact.as_bytes(), &sources), Ok(()));
    input.push(error());
    assert_eq!(render_json(&input, &sources).expect("extra"), TERMINAL.trim_end());
    let document = json!({"schema_version":2,"diagnostics":vec![value()["diagnostics"][0].clone(); MAX_DIAGNOSTICS + 1]});
    assert_eq!(validate(&document, &sources), Err(ProtocolError::Limit));
    assert_eq!(render_json(&input[..MAX_DIAGNOSTICS], &sources).expect("recovery"), exact);
    let mut terminal: Value = serde_json::from_str(TERMINAL).expect("terminal");
    terminal["diagnostics"][0]["message"] = json!("forged");
    assert_eq!(validate(&terminal, &sources), Err(ProtocolError::Record));
    assert_eq!(
        render_json(&[Diagnostic::error(EXHAUSTION_CODE, None, "", "")], &sources),
        Err(ProtocolError::Record)
    );
}

#[test]
fn exact_and_first_extra_utf8_string_budgets() {
    let sources = sources();
    for field in ["message", "guidance", "path"] {
        let limit = if field == "path" { MAX_PATH_BYTES } else { MAX_TEXT_BYTES };
        for unit in ["a", "é", "😀"] {
            let exact = unit.repeat(limit / unit.len());
            let mut record = error();
            match field {
                "message" => record.message = exact.clone(),
                "guidance" => record.guidance = exact.clone(),
                _ => {
                    record = Diagnostic::error(
                        "ZRYNA-T1001",
                        Some(exact.clone()),
                        "message",
                        "guidance",
                    );
                }
            }
            let rendered = render_json(&[record], &sources).expect("exact");
            assert!(!rendered.contains(EXHAUSTION_CODE));
            assert_eq!(validate_json(rendered.as_bytes(), &sources), Ok(()));
            let mut document: Value = serde_json::from_str(&rendered).expect("record");
            let pointer = if field == "path" {
                "/diagnostics/0/location/path".to_owned()
            } else {
                format!("/diagnostics/0/{field}")
            };
            *document.pointer_mut(&pointer).expect("field") = json!(format!("{exact}a"));
            assert_eq!(validate(&document, &sources), Err(ProtocolError::Limit));
            let extra = format!("{exact}a");
            let record = match field {
                "message" => Diagnostic::error("ZRYNA-T1001", None, extra, "guidance"),
                "guidance" => Diagnostic::error("ZRYNA-T1001", None, "message", extra),
                _ => Diagnostic::error("ZRYNA-T1001", Some(extra), "message", "guidance"),
            };
            assert_eq!(render_json(&[record], &sources).expect("extra"), TERMINAL.trim_end());
        }
    }
}

#[test]
fn exact_and_first_extra_encoded_document_bytes() {
    let sources = sources();
    let mut input = vec![Diagnostic::error("ZRYNA-T1001", None, "a".repeat(3800), ""); 16];
    let initial = render_json(&input, &sources).expect("initial");
    let padding = MAX_DOCUMENT_BYTES - initial.len();
    assert!(padding < MAX_TEXT_BYTES);
    input.last_mut().expect("last").guidance = "x".repeat(padding);
    let exact = render_json(&input, &sources).expect("exact");
    assert_eq!(exact.len(), MAX_DOCUMENT_BYTES);
    assert_eq!(validate_json(exact.as_bytes(), &sources), Ok(()));
    input.last_mut().expect("last").guidance.push('x');
    assert_eq!(render_json(&input, &sources).expect("extra"), TERMINAL.trim_end());
    assert_eq!(validate_json(format!("{exact} ").as_bytes(), &sources), Err(ProtocolError::Limit));
    let escaped = vec![Diagnostic::error("ZRYNA-T1001", None, "\0".repeat(MAX_TEXT_BYTES), ""); 3];
    assert_eq!(render_json(&escaped, &sources).expect("escaped"), TERMINAL.trim_end());
}

#[test]
fn ordering_uses_utf8_and_all_content_ties_without_deduplication() {
    let sources = sources();
    let mut input = vec![error(), error(), Diagnostic::warning("ZRYNA-T1001", None, "", "")];
    input[0].message = "\u{10000}".to_owned();
    input[1].message = "\u{e000}".to_owned();
    input.push(input[1].clone());
    let encoded = render_json(&input, &sources).expect("render");
    let mut document: Value = serde_json::from_str(&encoded).expect("document");
    assert_eq!(document["diagnostics"][0]["message"], "\u{e000}");
    assert_eq!(document["diagnostics"].as_array().expect("array").len(), 4);
    assert_eq!(validate(&document, &sources), Ok(()));
    document["diagnostics"].as_array_mut().expect("array").swap(0, 2);
    assert_eq!(validate(&document, &sources), Err(ProtocolError::Order));
    input.reverse();
    assert_eq!(render_json(&input, &sources).expect("reverse"), encoded);
}
