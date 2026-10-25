//! Exact unused accepted sources and both original provider packets; no supplied closed probe.
use super::{NEST, NODE, one, plain_errors, v4_errors};
use crate::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, resolve_declarations,
};
use sha2::{Digest, Sha256};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};

fn literal(id: &str, inspect: impl Fn(&super::BodyTypeContext<'_, '_>)) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-catalogue-fixtures");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("manifest.json")).expect("unchanged fixed manifest"),
    )
    .expect("manifest JSON");
    let case = manifest["cases"]
        .as_array()
        .expect("fixed cases")
        .iter()
        .find(|case| case["id"] == id)
        .expect("exact original case");
    let text = std::fs::read_to_string(root.join(case["source"].as_str().expect("source path")))
        .expect("literal complete unrooted source");
    assert_eq!(text, if id == "by-value-cycle" { NODE } else { NEST });
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        case["sourceSha256"].as_str().expect("frozen source SHA")
    );
    for provider in ["native", "typescript"] {
        let pin = &case["providers"][provider];
        let bytes =
            std::fs::read(root.join(pin["path"].as_str().expect("original provider packet")))
                .expect("untrusted exact DTO");
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().expect("frozen bytes"));
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            pin["sha256"].as_str().expect("frozen packet SHA")
        );
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.clone() }])
                .expect("exact source authority");
        let syntax = verify_snapshot(decode_snapshot(&bytes).expect("closed raw DTO"), &sources)
            .expect("full original source authentication before owning rejection");
        assert!(
            syntax.files()[0].functions.is_empty(),
            "no fabricated Node<i32> parameter/function"
        );
        assert_eq!(syntax.files()[0].data_declarations.len(), 1);
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, sources.verify_file_id(0).expect("entry"))
                .expect("original input"),
        )
        .expect("original declaration resolution");
        let bodies = check_body_types(&declarations).expect("opaque original member types");
        inspect(&bodies);
    }
}

#[test]
fn literal_unrooted_by_value_cycle_is_plain_layout_rejection_at_full_original_head() {
    literal("by-value-cycle", |bodies| {
        one(&plain_errors(bodies), "ZRYNA-L3002", 65, 72);
    });
}
#[test]
fn literal_unrooted_by_value_cycle_reaches_the_same_layout_owner_before_v4_ownership() {
    literal("by-value-cycle", |bodies| {
        one(&v4_errors(bodies), "ZRYNA-L3002", 65, 72);
    });
}
#[test]
fn literal_unrooted_expansion_retains_plain_m7003_at_the_fixed_full_head() {
    literal("expanding-declaration", |bodies| {
        one(&plain_errors(bodies), "ZRYNA-M7003", 69, 81);
    });
}
#[test]
fn literal_unrooted_expansion_reaches_m7003_before_private_v4_source_exclusion() {
    literal("expanding-declaration", |bodies| {
        one(&v4_errors(bodies), "ZRYNA-M7003", 69, 81);
    });
}
