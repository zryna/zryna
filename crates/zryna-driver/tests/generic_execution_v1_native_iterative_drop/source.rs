//! Both genuine frontend routes, unchanged literals and an independent same-seal owner.
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1 as semantic, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn pinned(row: &serde_json::Value, key: &str) -> Vec<u8> {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(row[key].as_str().expect("frozen relative input")),
    )
    .expect("actual original input");
    assert_eq!(digest(&bytes), row[format!("{key}_sha256")]);
    assert_eq!(u64::try_from(bytes.len()).expect("bounded bytes"), row[format!("{key}_bytes")]);
    bytes
}
pub(super) fn with_program(
    case: &str,
    inspect: impl Fn(&str, &execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    let inputs: serde_json::Value = serde_json::from_str(include_str!("input-pins.json"))
        .expect("preoutput provider/source pins");
    let row = &inputs[case];
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(pinned(row, "source")).expect("source UTF8"),
    }])
    .expect("actual main source");
    let packet = pinned(row, "packet");
    let native = zryna_frontend::native_parser::v5::parse_v5_candidate(
        &sources,
        &zryna_frontend::native_lexer::lex(&sources).expect("native original lexer"),
    )
    .expect("actual native frontend");
    assert_eq!(
        serde_json::to_value(&native).expect("complete native DTO"),
        serde_json::from_slice::<serde_json::Value>(&packet).expect("actual TS DTO")
    );
    for (provider, candidate) in [
        ("native", native),
        ("typescript", zryna_syntax::v5::decode_snapshot(&packet).expect("strict TS DTO")),
    ] {
        let syntax = zryna_syntax::v5::verify_snapshot(candidate, &sources).expect("syntax owner");
        let entry = sources.verify_file_id(0).expect("entry");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, entry).expect("original semantic input"),
        )
        .expect("original declarations");
        let bodies = check_execution_body_types(&declarations).expect("original body checks");
        let instances = semantic::discover(&bodies).expect("source instances");
        let linear =
            verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
        let linux =
            verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("Linux");
        let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
            zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux)
                .expect("runtime claims"),
            &linear,
            &linux,
        )
        .expect("runtime authority");
        let claim = semantic::produce_claim(&instances, &linear, &linux).expect("source claim");
        let wire = execution_v1::wire::encode(&claim).expect("new carrier");
        let program = execution_v1::verify(
            execution_v1::wire::decode(&wire).expect("carrier"),
            &syntax,
            &sources,
            entry,
            &linear,
            &linux,
            &runtime,
        )
        .expect("independent whole owner");
        inspect(provider, &program, &wire);
    }
}
