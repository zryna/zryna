//! Literal specification fixtures cross plain semantic discovery; no owned-profile remapping.
use super::{InstantiationFailure, discover};
use crate::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, resolve_declarations,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, path::PathBuf};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};

type Inventory = (Vec<Vec<u8>>, Vec<Vec<u8>>, usize);

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).expect("literal specification file"))
        .expect("specification JSON")
}

fn catalogue_source(id: &str) -> String {
    json("spec/language/generic-review-v1-fixtures.json")["sourceCases"]
        .as_array()
        .expect("accepted catalogue")
        .iter()
        .find(|case| case["id"] == id)
        .expect("fixed source case")["source"]
        .as_str()
        .expect("literal source")
        .into()
}

fn plain_discovery(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<Inventory, InstantiationFailure> {
    let entry = sources.verify_file_id(0).expect("original entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(syntax, sources, entry).expect("authenticated source input"),
    )
    .expect("all original declaration/name checks precede discovery");
    let bodies =
        check_body_types(&declarations).expect("all original opaque body checks precede discovery");
    let instances = discover(&bodies)?;
    Ok((
        instances.type_keys().map(<[u8]>::to_vec).collect(),
        instances.function_keys().map(<[u8]>::to_vec).collect(),
        instances.edges().len(),
    ))
}

#[test]
fn unused_literal_expanding_definition_rejects_at_plain_discovery_for_both_producers() {
    let manifest = json("tests/m7-generic-catalogue-fixtures/manifest.json");
    let case = manifest["cases"]
        .as_array()
        .expect("fixed complete source fixtures")
        .iter()
        .find(|case| case["id"] == "expanding-declaration")
        .expect("original unrooted fixture");
    let source = catalogue_source("expanding-declaration");
    assert_eq!(source.len(), 85);
    assert_eq!(source, case["catalogueSource"].as_str().expect("exact catalogue source"));
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        case["sourceSha256"].as_str().expect("exact source SHA")
    );
    assert_eq!(
        source,
        std::fs::read_to_string(
            root().join("tests/m7-generic-catalogue-fixtures/expanding-declaration/main.zry")
        )
        .expect("exact accepted source, with no synthetic root")
    );
    assert_eq!(&source.as_bytes()[69..81], b"Nest<Vec<T>>");
    assert_eq!(case["code"], "ZRYNA-M7003");
    assert_eq!(case["primarySpan"], serde_json::json!({"file":0,"start":69,"end":81}));
    for provider in ["native", "typescript"] {
        let pin = &case["providers"][provider];
        let bytes = std::fs::read(
            root()
                .join("tests/m7-generic-catalogue-fixtures")
                .join(pin["path"].as_str().expect("frozen producer DTO")),
        )
        .expect("raw source facts");
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().expect("pinned DTO size"));
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            pin["sha256"].as_str().expect("frozen DTO SHA")
        );
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: source.clone(),
        }])
        .expect("same immutable source for each independently authenticated DTO");
        let syntax = verify_snapshot(decode_snapshot(&bytes).expect("closed raw schema"), &sources)
            .expect("complete original source authentication before semantic rejection");
        let InstantiationFailure::Diagnostics(errors) = plain_discovery(&syntax, &sources)
            .expect_err("unused declaration must reject without inventing Nest<i32>")
        else {
            panic!("semantic owning diagnostic required");
        };
        assert_eq!(errors.len(), 1, "{provider}: {errors:?}");
        assert_eq!(errors[0].code(), "ZRYNA-M7003", "{provider}: {errors:?}");
        let span = errors[0].primary_span().expect("original generated application head");
        assert_eq!((span.file().index(), span.start(), span.end()), (0, 69, 81));
    }
}

#[test]
fn unused_same_key_indirection_adds_no_closed_instance_or_dependency() {
    let source = catalogue_source("same-key-indirection");
    let (types, functions, edges) = super::tests::check(&[("main.zry", &source)])
        .expect("same-key Vec indirection is legal, including without a closed root");
    assert_eq!(types, [vec![0], vec![1], vec![2]]);
    assert!(functions.is_empty());
    assert_eq!(edges, 0, "definition checking must not invent a closed probe instance");
}

#[test]
fn finite_supplied_type_projection_keeps_the_two_literal_box_instances() {
    let literal = catalogue_source("finite-supplied-nesting");
    let (declaration, local) = literal.split_once(" const value:").expect("fixed catalogue split");
    let (annotation, _) = local.split_once(" =").expect("fixed literal annotated initializer");
    assert_eq!(annotation.trim(), "Box<Box<i32>>");
    // This is explicitly a type-only projection, preserving the catalogue declaration and
    // annotation. Struct-constructor source qualification remains a separate obligation.
    let source =
        format!("{declaration} function read(value:{}):i32 {{return 0;}}", annotation.trim());
    let (types, functions, _) = super::tests::check(&[("main.zry", &source)])
        .expect("a copied supplied Box<i32> subtree is not declaration-generated growth");
    assert_eq!(types.iter().filter(|key| key.first() == Some(&0x12)).count(), 2);
    let expected = json("spec/language/generic-instantiation-v1-fixtures.json")
        ["identities"]["finiteNestedBox"]["keyHex"].as_str().expect("fixed complete Box key").to_owned();
    assert!(types.iter().any(|key| {
        let mut encoded = String::new();
        for byte in key {
            write!(&mut encoded, "{byte:02x}").expect("hexadecimal key formatting");
        }
        encoded == expected
    }));
    assert!(functions.is_empty());
}

#[test]
fn binder_scoped_flip_can_unify_until_actual_closed_arguments_are_known() {
    const DECLARATION: &str = "interface Flip<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {next:Vec<Flip<E,T>>;}";
    let (types, functions, edges) = super::tests::check(&[("main.zry", DECLARATION)])
        .expect("open T and E may close to the same key; symbolic inequality proves no growth");
    assert_eq!(types, [vec![0], vec![1], vec![2]]);
    assert!(functions.is_empty());
    assert_eq!(edges, 0);
    let equal = format!("{DECLARATION} function read(value:Flip<i32,i32>):i32 {{return 0;}}");
    let (types, _, _) = super::tests::check(&[("main.zry", &equal)])
        .expect("same actual closed key deduplicates through Vec");
    assert_eq!(types.iter().filter(|key| key.first() == Some(&0x12)).count(), 1);
    let unequal = format!("{DECLARATION} function read(value:Flip<i32,bool>):i32 {{return 0;}}");
    let InstantiationFailure::Diagnostics(errors) = super::tests::check(&[("main.zry", &unequal)])
        .expect_err("the existing generated rule still rejects different actual closed arguments")
    else {
        panic!("closed expansion diagnostic");
    };
    assert_eq!(errors[0].code(), "ZRYNA-M7003");
}

mod additional;
