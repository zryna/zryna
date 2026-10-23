//! Complete original Struct field occurrences reach semantic diagnostic ownership.
use super::production;
use serde_json::Value;
use zryna_frontend::{WorkerFailure, native_lexer, native_parser::v5::parse_v5_candidate};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{RawProjectSyntaxSnapshot, decode_snapshot, verify_snapshot};

fn cases() -> Vec<(String, SourceMap, RawProjectSyntaxSnapshot)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-duplicate-struct-fields");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(root.join("manifest.json")).expect("independent frozen manifest"),
    )
    .expect("JSON");
    manifest["cases"]
        .as_array()
        .expect("six literal originals")
        .iter()
        .map(|case| {
            let inputs: Value = serde_json::from_slice(
                &std::fs::read(root.join(case["sources"].as_str().expect("source path")))
                    .expect("original source file"),
            )
            .expect("JSON");
            let sources = SourceMap::build(
                inputs
                    .as_array()
                    .expect("source inventory")
                    .iter()
                    .map(|file| SourceFileInput {
                        path: file["path"].as_str().expect("portable path").into(),
                        text: file["text"].as_str().expect("complete source").into(),
                    })
                    .collect(),
            )
            .expect("authoritative source map");
            let expected = decode_snapshot(
                &std::fs::read(root.join(case["expected"].as_str().expect("fixed expected path")))
                    .expect("independently authored complete DTO"),
            )
            .expect("closed v5 schema");
            (case["id"].as_str().expect("case ID").to_owned(), sources, expected)
        })
        .collect()
}

#[test]
fn native_v5_retains_every_duplicate_and_valid_struct_initializer() {
    for (id, sources, expected) in cases() {
        let raw =
            parse_v5_candidate(&sources, &native_lexer::lex(&sources).expect("original lexer"))
                .expect("duplicate field is semantic input, not provider rejection");
        assert_eq!(raw, expected, "{id}: no field or value may be erased/reordered");
        let syntax = verify_snapshot(raw, &sources).expect("complete original source proof");
        assert!(syntax.is_bound_to(&sources));
        assert!(syntax.advisories().is_empty());
    }
}

#[test]
fn actual_typescript_v5_retains_every_duplicate_and_valid_struct_initializer() {
    for (id, sources, expected) in cases() {
        let syntax = production()
            .analyze_verified_v5(&sources)
            .expect("actual worker must preserve duplicate original fields");
        assert_eq!(
            syntax.files(),
            expected.files,
            "{id}: complete independently fixed source facts"
        );
        assert!(syntax.advisories().is_empty());
    }
}

#[test]
fn independently_literal_duplicate_dtos_authenticate_but_omitted_fields_do_not() {
    for (id, sources, expected) in cases() {
        verify_snapshot(expected.clone(), &sources)
            .expect("fixed fixture is actual original syntax");
        if id.ends_with("-valid") {
            continue;
        }
        let mut omitted = expected;
        let expression = omitted.files[0].functions[0]
            .body
            .expressions
            .iter_mut()
            .find(|expression| {
                matches!(
                    expression.kind,
                    zryna_syntax::v5::RawExpressionKind::StructConstruction { .. }
                )
            })
            .expect("original constructor");
        let zryna_syntax::v5::RawExpressionKind::StructConstruction { fields, .. } =
            &mut expression.kind
        else {
            unreachable!()
        };
        assert_eq!(fields.len(), 2);
        fields.remove(1);
        let errors = verify_snapshot(omitted, &sources)
            .expect_err("raw repair cannot erase original duplicate tokens");
        assert!(errors.iter().all(|error| error.code == "ZRYNA-Y5001"));
    }
}

#[test]
fn duplicate_preservation_keeps_malformed_sources_fail_closed() {
    let (_, source_map, _) = cases().remove(0);
    let original =
        source_map.source(source_map.verify_file_id(0).expect("entry")).expect("source").text();
    for text in [original.replace("value: 8", "value:"), original.trim_end_matches('}').to_owned()]
    {
        let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
            .expect("source");
        assert_eq!(
            parse_v5_candidate(&sources, &native_lexer::lex(&sources).expect("lexer"))
                .expect_err("no partial DTO")
                .diagnostic()
                .code(),
            "ZRYNA-F2002"
        );
        let error = production()
            .analyze_verified_v5(&sources)
            .expect_err("actual malformed worker source rejects");
        assert_eq!(error.failure(), WorkerFailure::ProviderRejected);
        assert!(error.diagnostics().is_empty(), "provider code never becomes semantic authority");
    }
}

#[test]
fn repeated_field_occurrences_keep_exact_and_first_extra_original_initializer_limits() {
    for count in [1024, 1025] {
        let text = format!(
            "interface Box<T extends ZrynaValue> extends ZrynaStruct {{value:T;}} function score():i32 {{const box:Box<i32> =Box<i32>({{{}}});return 0;}}",
            vec!["value:7"; count].join(",")
        );
        let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
            .expect("exact limit source");
        let native =
            parse_v5_candidate(&sources, &native_lexer::lex(&sources).expect("original lexing"));
        let typescript = production().analyze_verified_v5(&sources);
        if count == 1024 {
            let syntax =
                verify_snapshot(native.expect("all1024 original fields retained"), &sources)
                    .expect("full original source proof");
            let peer = typescript.expect("actual TS exact initializer ceiling");
            assert_eq!(syntax.files(), peer.files());
            let fields = syntax.files()[0].functions[0]
                .body
                .expressions
                .iter()
                .find_map(|expression| {
                    if let zryna_syntax::v5::RawExpressionKind::StructConstruction {
                        fields, ..
                    } = &expression.kind
                    {
                        Some(fields)
                    } else {
                        None
                    }
                })
                .expect("source constructor");
            assert_eq!(fields.len(), 1024);
        } else {
            assert_eq!(
                native
                    .expect_err("first extra native field rejects before retaining candidates")
                    .diagnostic()
                    .code(),
                "ZRYNA-F1002"
            );
            let error = typescript.expect_err("first extra TS field is still a provider rejection");
            assert_eq!(error.failure(), WorkerFailure::ProviderRejected);
            assert!(error.diagnostics().is_empty());
        }
    }
}
