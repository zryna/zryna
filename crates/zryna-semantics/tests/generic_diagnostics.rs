//! Implemented private diagnostic cases checked against frozen catalogue oracles.
//! Includes literal unrooted by-value rejection at its original layout-rule owner.

use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::{Failure as IrFailure, owned_v2 as ir};
use zryna_layout::{StorageTarget, generic_v1::Failure as LayoutFailure};
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    instantiation::{self, InstantiationFailure, layouts, owned_v2},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap, Span, UntrustedSpan};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};

#[derive(Debug)]
struct Observation {
    phase: &'static str,
    code: Option<String>,
    span: Option<UntrustedSpan>,
    detail: String,
}

fn span(value: Span) -> UntrustedSpan {
    UntrustedSpan { file: value.file().index(), start: value.start(), end: value.end() }
}

fn diagnostic(phase: &'static str, errors: &[Diagnostic]) -> Observation {
    let first = errors.first().expect("rejection has a diagnostic");
    Observation {
        phase,
        code: Some(first.code().into()),
        span: first.primary_span().map(span),
        detail: format!("{errors:?}"),
    }
}

fn other(phase: &'static str, failure: impl std::fmt::Debug) -> Observation {
    Observation { phase, code: None, span: None, detail: format!("{failure:?}") }
}

fn ir_failure(phase: &'static str, failure: IrFailure) -> Observation {
    match failure {
        IrFailure::Diagnostics(errors) => diagnostic(phase, &errors),
        failure => other(phase, failure),
    }
}

fn layout_failure(phase: &'static str, failure: LayoutFailure) -> Observation {
    match failure {
        LayoutFailure::Diagnostics(errors) => diagnostic(phase, &errors),
        failure => other(phase, failure),
    }
}

fn observe(sources: &SourceMap, bytes: &[u8]) -> Observation {
    let raw = decode_snapshot(bytes).expect("frozen closed producer DTO decodes");
    let syntax = match verify_snapshot(raw, sources) {
        Ok(syntax) => syntax,
        Err(errors) => {
            let first = errors.first().expect("syntax rejection has a diagnostic");
            return Observation {
                phase: "syntax verification/declaration shape",
                code: Some(first.code.into()),
                span: first.span.map(span),
                detail: format!("{errors:?}"),
            };
        }
    };
    assert!(syntax.is_bound_to(sources));
    let entry = sources.verify_file_id(0).expect("original entry");
    let input = SemanticInput::try_new(&syntax, sources, entry).expect("source-bound syntax input");
    let declarations = match resolve_declarations(input) {
        Ok(declarations) => declarations,
        Err(errors) => return diagnostic("declaration resolution", &errors),
    };
    let bodies = match check_body_types(&declarations) {
        Ok(bodies) => bodies,
        Err(BodyTypeFailure::Diagnostics(errors)) => {
            return diagnostic("original body types", &errors);
        }
        Err(failure) => return other("original body types", failure),
    };
    // Check discovery before entering the public ownership wrapper. That wrapper independently
    // checks every original body and repeats discovery; no private constructor or shortcut is used.
    if let Err(failure) = instantiation::discover(&bodies) {
        return match failure {
            InstantiationFailure::Diagnostics(errors) => diagnostic("instantiation", &errors),
            failure => other("instantiation", failure),
        };
    }
    let owned = match owned_v2::discover(&bodies) {
        Ok(owned) => owned,
        Err(failure) => return ir_failure("original ownership/discovery", failure),
    };
    let linear = match layouts::verify_layouts(owned.instances(), StorageTarget::Linear32V1) {
        Ok(layouts) => layouts,
        Err(failure) => return layout_failure("linear32 layout", failure),
    };
    let linux = match layouts::verify_layouts(owned.instances(), StorageTarget::LinuxX8664V1) {
        Ok(layouts) => layouts,
        Err(failure) => return layout_failure("linux layout", failure),
    };
    let claim = match owned_v2::produce_claim(&owned, &linear, &linux) {
        Ok(claim) => claim,
        Err(failure) => return ir_failure("source-owned claim production", failure),
    };
    let contract = match zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux) {
        Ok(contract) => contract,
        Err(failure) => return other("runtime declaration production", failure),
    };
    let runtime =
        match zryna_ownership_runtime_abi::generic_v1::verify_v1(contract, &linear, &linux) {
            Ok(runtime) => runtime,
            Err(failure) => return other("runtime declaration verification", failure),
        };
    let encoded = match ir::wire::encode(&claim) {
        Ok(encoded) => encoded,
        Err(failure) => return ir_failure("owned wire encoding", failure),
    };
    let decoded = match ir::wire::decode(&encoded) {
        Ok(decoded) => decoded,
        Err(failure) => return ir_failure("owned wire decoding", failure),
    };
    match ir::verify(decoded, &syntax, sources, entry, &linear, &linux, &runtime) {
        Err(failure) => ir_failure("owned IR and scalar ABI verification", failure),
        Ok(_) => other("all available public authorities", "no rejection"),
    }
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root")
        .join("tests/m7-generic-catalogue-fixtures")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn run_case(id: &str) {
    let root = fixture_root();
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(root.join("manifest.json")).expect("frozen manifest"),
    )
    .expect("closed fixture manifest");
    assert_eq!(manifest["commit"], "e639c525c0b506a5606429a9f99c59ad3a9b7a5c");
    let catalogue_bytes = std::fs::read(
        root.parent()
            .and_then(std::path::Path::parent)
            .expect("repository root")
            .join("spec/language/generic-review-v1-fixtures.json"),
    )
    .expect("accepted source catalogue");
    assert_eq!(
        sha256(&catalogue_bytes),
        manifest["catalogueSha256"].as_str().expect("catalogue SHA")
    );
    let catalogue: Value =
        serde_json::from_slice(&catalogue_bytes).expect("accepted catalogue JSON");
    let oracle = catalogue["sourceCases"]
        .as_array()
        .expect("catalogue cases")
        .iter()
        .find(|entry| entry["id"] == id)
        .expect("original catalogue case");
    let entries = manifest["cases"].as_array().expect("cases");
    assert_eq!(entries.len(), 19);
    let case = entries.iter().find(|case| case["id"] == id).expect("frozen catalogue case");
    let source = std::fs::read(root.join(case["source"].as_str().expect("source path")))
        .expect("exact wrapped source");
    assert_eq!(sha256(&source), case["sourceSha256"].as_str().expect("source SHA"));
    let expected: UntrustedSpan = serde_json::from_value(case["primarySpan"].clone())
        .expect("frozen rebased UTF-8 primary span");
    assert_eq!(
        &source[expected.start as usize..expected.end as usize],
        case["primaryToken"].as_str().expect("catalogue primary spelling").as_bytes()
    );
    assert_eq!(case["code"], oracle["code"]);
    assert_eq!(case["primaryToken"], oracle["primaryToken"]);
    assert_eq!(case["primaryOccurrence"], oracle["primaryOccurrence"]);
    assert_eq!(case["catalogueSource"], oracle["source"]);
    let expected_code = oracle["code"].as_str().expect("catalogue rejection code");
    let mut failures = Vec::new();
    let mut payloads = Vec::new();
    for provider in ["native", "typescript"] {
        let witness = &case["providers"][provider];
        let bytes = std::fs::read(root.join(witness["path"].as_str().expect("provider payload")))
            .expect("unchanged producer result-value bytes");
        assert_eq!(bytes.len() as u64, witness["bytes"].as_u64().expect("payload bytes"));
        assert_eq!(sha256(&bytes), witness["sha256"].as_str().expect("payload SHA"));
        payloads.push(serde_json::from_slice::<Value>(&bytes).expect("raw DTO JSON"));
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: String::from_utf8(source.clone()).expect("UTF-8 fixture"),
        }])
        .expect("immutable source map");
        sources.verify_span(expected).expect("oracle range belongs to exact original source");
        let observed = observe(&sources, &bytes);
        println!(
            "catalogue {id}/{provider}: phase={}, observed={:?} {:?}; oracle={expected_code} {expected:?}; {}",
            observed.phase, observed.code, observed.span, observed.detail
        );
        if observed.code.as_deref() != Some(expected_code) || observed.span != Some(expected) {
            failures.push(format!("{provider}: {observed:?}"));
        }
    }
    assert_eq!(payloads[0], payloads[1], "both retained producer DTO structures agree");
    assert!(failures.is_empty(), "{id}: expected {expected_code} {expected:?}; {failures:?}");
}

macro_rules! catalogue_case {
    ($name:ident, $id:literal) => {
        #[test]
        fn $name() {
            run_case($id);
        }
    };
}

catalogue_case!(wrong_bound, "wrong-bound");
catalogue_case!(omitted_argument, "omitted-argument");
catalogue_case!(extra_argument, "extra-argument");
catalogue_case!(borrow_argument, "borrow-argument");
catalogue_case!(unit_argument, "unit-argument");
catalogue_case!(opaque_clone, "opaque-clone");
catalogue_case!(opaque_arithmetic, "opaque-arithmetic");
catalogue_case!(recursive_function, "recursive-function");
catalogue_case!(expanding_declaration, "expanding-declaration");
catalogue_case!(by_value_cycle, "by-value-cycle");
catalogue_case!(unknown_variant, "unknown-variant");
catalogue_case!(missing_arm, "missing-arm");
catalogue_case!(aggregate_export, "aggregate-export");
catalogue_case!(wrong_payload, "wrong-payload");
catalogue_case!(extra_payload, "extra-payload");
catalogue_case!(borrowed_payload_move, "borrowed-payload-move");
catalogue_case!(declaration_merge, "declaration-merge");
catalogue_case!(higher_kinded_use, "higher-kinded-use");
catalogue_case!(utf8_primary_span, "utf8-primary-span");

#[test]
fn duplicate_arm_independent_complete_dto_uses_full_second_quoted_key() {
    let root = fixture_root().join("duplicate-arm");
    let source = std::fs::read(root.join("main.zry")).expect("independent literal source");
    let bytes = std::fs::read(root.join("independent.json")).expect("independently authored DTO");
    let witness: Value = serde_json::from_slice(
        &std::fs::read(root.join("witness.json")).expect("frozen independent witness"),
    )
    .expect("witness JSON");
    assert_eq!(sha256(&source), witness["sourceSha256"].as_str().expect("source SHA"));
    assert_eq!(sha256(&bytes), witness["payloadSha256"].as_str().expect("payload SHA"));
    let expected = UntrustedSpan { file: 0, start: 81, end: 94 };
    assert_eq!(&source[81..94], b"\"Option.none\"");
    assert_eq!(witness["expectedCode"], "ZRYNA-M7004");
    assert_eq!(
        serde_json::to_value(expected).expect("oracle span"),
        witness["expectedPrimarySpan"]
    );
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(source).expect("UTF-8 source"),
    }])
    .expect("exact immutable source");
    let syntax = verify_snapshot(decode_snapshot(&bytes).expect("closed DTO"), &sources)
        .expect("complete independent source/arena must authenticate before semantic rejection");
    assert!(syntax.is_bound_to(&sources));
    let observed = observe(&sources, &bytes);
    println!("catalogue duplicate-arm/independent: {observed:?}; oracle=ZRYNA-M7004 {expected:?}");
    assert_eq!(observed.code.as_deref(), Some("ZRYNA-M7004"), "{observed:?}");
    assert_eq!(observed.span, Some(expected), "{observed:?}");
}
