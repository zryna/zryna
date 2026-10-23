//! Every native test crosses the original syntax/source/layout/runtime/owned issuers.
use zryna_ir::generic_v1::owned_v2;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{
        layouts::verify_layouts,
        owned_v2::{discover, produce_claim},
    },
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{RawProjectSyntaxSnapshot, decode_snapshot, verify_snapshot};

pub(super) fn with_program(cross: bool, inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>)) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-owned-nominal-enum");
    let values = std::fs::read_to_string(root.join("values.zry")).expect("fixed declarations");
    let main = std::fs::read_to_string(root.join("main.zry")).expect("fixed entry source");
    let text = if cross {
        format!(
            "{}\n{main}",
            std::fs::read_to_string(root.join("imports.txt"))
                .expect("fixed named imports")
                .trim_end()
        )
    } else {
        format!("{values}\n{main}")
    };
    let mut files = vec![SourceFileInput { path: "main.zry".into(), text }];
    if cross {
        files.push(SourceFileInput { path: "values.zry".into(), text: values });
    }
    let bytes = std::fs::read(root.join(format!("{}-reference.json", files.len())))
        .expect("independent frozen syntax DTO");
    with_sources(
        &SourceMap::build(files).expect("exact source map"),
        decode_snapshot(&bytes).expect("closed DTO"),
        inspect,
    );
}

pub(super) fn with_source(text: String, inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>)) {
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("bounded generated authoritative source");
    let lexed = zryna_frontend::native_lexer::lex(&sources).expect("original bounded lexer");
    let raw = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
        .expect("untrusted native syntax candidate");
    with_sources(&sources, raw, inspect);
}

fn with_sources(
    sources: &SourceMap,
    raw: RawProjectSyntaxSnapshot,
    inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>),
) {
    let syntax = verify_snapshot(raw, sources).expect("complete source authentication");
    let entry = sources.verify_file_id(0).expect("entry module");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, sources, entry).expect("source-bound semantic input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("original symbolic bodies");
    let instances = discover(&bodies).expect("complete demanded owned inventory");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("original linear layout issuer");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("original Linux layout issuer");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("runtime declaration claims"),
        &linear,
        &linux,
    )
    .expect("independent retained runtime issuer");
    let claim = produce_claim(&instances, &linear, &linux).expect("untrusted owned claims");
    let wire = owned_v2::wire::encode(&claim).expect("bounded owned wire");
    let program = owned_v2::verify(
        owned_v2::wire::decode(&wire).expect("closed owned wire"),
        &syntax,
        sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent source, contextual type and ownership seal");
    inspect(&program);
}
