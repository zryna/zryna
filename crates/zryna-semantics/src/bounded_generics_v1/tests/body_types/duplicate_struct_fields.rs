//! Original body checking owns duplicate-field diagnostics at exact source spans.
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    resolve_declarations,
};
use serde_json::Value;
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};

#[test]
fn original_generic_duplicate_fields_report_exact_second_utf8_field_and_controls_pass() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-duplicate-struct-fields");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(root.join("manifest.json")).expect("literal fixture manifest"),
    )
    .expect("JSON");
    for case in manifest["cases"].as_array().expect("fixed six cases") {
        let files: Value = serde_json::from_slice(
            &std::fs::read(root.join(case["sources"].as_str().expect("source path")))
                .expect("complete source"),
        )
        .expect("JSON");
        let sources = SourceMap::build(
            files
                .as_array()
                .expect("files")
                .iter()
                .map(|file| SourceFileInput {
                    path: file["path"].as_str().expect("path").into(),
                    text: file["text"].as_str().expect("text").into(),
                })
                .collect(),
        )
        .expect("source authority");
        let syntax = verify_snapshot(
            decode_snapshot(
                &std::fs::read(root.join(case["expected"].as_str().expect("expected path")))
                    .expect("literal independent DTO"),
            )
            .expect("closed raw schema"),
            &sources,
        )
        .expect("all files pass full original syntax proof before semantic diagnostics");
        let declarations = resolve_declarations(
            SemanticInput::try_new(
                &syntax,
                &sources,
                sources.verify_file_id(0).expect("main entry"),
            )
            .expect("exact source input"),
        )
        .expect("original imports/names and generic declarations");
        if case["expectedCode"].is_null() {
            check_body_types(&declarations)
                .expect("same valid source form passes opaque-body checking");
            continue;
        }
        let Err(BodyTypeFailure::Diagnostics(errors)) = check_body_types(&declarations) else {
            panic!("owning body diagnostic, no typed context")
        };
        assert_eq!(errors.len(), 1, "{}: {errors:?}", case["id"]);
        assert_eq!(errors[0].code(), "ZRYNA-M7006");
        let span = errors[0].primary_span().expect("original second duplicate field");
        assert_eq!(span.file().index(), 0);
        assert_eq!(
            u64::from(span.start()),
            case["primarySpan"]["start"].as_u64().expect("fixed UTF8 start")
        );
        assert_eq!(
            u64::from(span.end()),
            case["primarySpan"]["end"].as_u64().expect("fixed UTF8 end")
        );
        let text = files[0]["text"].as_str().expect("original main source");
        assert_eq!(&text.as_bytes()[span.start() as usize..span.end() as usize], b"value");
    }
}
