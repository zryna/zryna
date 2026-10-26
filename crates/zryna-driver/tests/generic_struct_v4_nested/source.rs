//! Every provider authenticates the same complete literal; no wrapper is appended.
use super::oracle;
use zryna_ir::generic_v1::owned_v2;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{
        layouts::verify_layouts,
        owned_v4::{discover, produce_claim},
    },
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};

pub(super) const POSITIVE_CASES: [&str; 8] = [
    "nested-copy",
    "nested-owned-call",
    "option-box-root",
    "box-option-root",
    "plain-and-reversed-arguments",
    "phantom-remains-copy",
    "fixed-member-remains-affine",
    "nested-prefix-faults",
];
pub(super) const ALL_CASES: [&str; 13] = [
    "nested-copy",
    "nested-owned-call",
    "option-box-root",
    "box-option-root",
    "plain-and-reversed-arguments",
    "phantom-remains-copy",
    "fixed-member-remains-affine",
    "nested-prefix-faults",
    "moved-whole-twice",
    "loaned-whole-consumed",
    "affine-field-projection",
    "opaque-copy-assumption",
    "exclusive-copy-observation",
];
pub(super) struct Evidence {
    pub(super) source: Vec<u8>,
    pub(super) native: Vec<u8>,
    pub(super) typescript: Vec<u8>,
}
pub(super) fn authenticate(case: &str) -> (SourceMap, VerifiedProjectSyntaxV5, Evidence) {
    assert!(ALL_CASES.contains(&case));
    let source = oracle::read_fixture(&format!("{case}.zry"));
    let native = oracle::read_fixture(&format!("{case}-native.json"));
    let typescript = oracle::read_fixture(&format!("{case}-typescript.json"));
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(source.clone()).expect("literal UTF8"),
    }])
    .expect("exact complete literal source map");
    let lexed = zryna_frontend::native_lexer::lex(&sources).expect("actual native lexer");
    let candidate = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
        .expect("actual native parser candidate");
    let syntax =
        v5::verify_snapshot(candidate, &sources).expect("actual native source authentication");
    for (provider, packet) in [("native", &native), ("typescript", &typescript)] {
        let frozen =
            v5::verify_snapshot(v5::decode_snapshot(packet).expect("closed v5 packet"), &sources)
                .expect("frozen provider exact-source authentication");
        assert_eq!(syntax.files(), frozen.files(), "{case}/{provider}: complete source facts");
    }
    (sources, syntax, Evidence { source, native, typescript })
}
pub(super) fn with_program(
    case: &str,
    inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>, &[u8], &Evidence),
) {
    assert!(POSITIVE_CASES.contains(&case));
    let (sources, syntax, evidence) = authenticate(case);
    owned_v2::check_original_bodies_v4(&syntax, &sources).expect("all opaque original ownership");
    let entry = sources.verify_file_id(0).expect("literal entry module");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("bound semantic input"),
    )
    .expect("all original declarations");
    let bodies = check_body_types(&declarations).expect("all original body types");
    let instances = discover(&bodies).expect("original ownership and demanded v4 instances");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("sealed original linear layout");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("sealed original Linux layout");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("runtime declarations"),
        &linear,
        &linux,
    )
    .expect("independent runtime authority");
    let claim = produce_claim(&instances, &linear, &linux).expect("one source-produced v4 claim");
    let wire = owned_v2::wire::v4::encode(&claim).expect("exact bounded v4 message");
    let program = owned_v2::verify_v4(
        owned_v2::wire::v4::decode(&wire).expect("distinct v4 carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent source/type/ownership/runtime v4 issuer");
    assert_eq!(program.scalar_abi().exports().len(), 1, "only literal scalar entry exported");
    inspect(&program, &wire, &evidence);
}
