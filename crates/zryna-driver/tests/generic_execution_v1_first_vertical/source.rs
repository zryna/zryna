//! Actual native parse + both frozen provider DTOs, then one genuine new-domain issuer.
use super::authority;
use zryna_ir::generic_v1::execution_v1 as execution;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};
pub(super) struct Evidence {
    pub(super) source: Vec<u8>,
    pub(super) native: Vec<u8>,
    pub(super) typescript: Vec<u8>,
}
pub(super) fn authenticate(case: &str) -> (SourceMap, VerifiedProjectSyntaxV5, Evidence) {
    let source = authority::read(&format!("literals/{case}.zry"));
    let native = authority::read(&format!("{case}-native.json"));
    let typescript = authority::read(&format!("{case}-typescript.json"));
    assert_eq!(authority::digest(&source), authority::case(case)["source_sha256"]);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(source.clone()).expect("literal UTF8"),
    }])
    .expect("exact complete literal map");
    let lexed = zryna_frontend::native_lexer::lex(&sources).expect("actual current native lexer");
    let candidate = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
        .expect("actual current native parser");
    let syntax =
        v5::verify_snapshot(candidate, &sources).expect("actual original source authority");
    for (provider, packet) in [("native", &native), ("typescript", &typescript)] {
        let frozen = v5::verify_snapshot(
            v5::decode_snapshot(packet).expect("complete strict provider DTO"),
            &sources,
        )
        .expect("frozen exact-source syntax");
        assert_eq!(
            syntax.files(),
            frozen.files(),
            "{case}/{provider}: complete original syntax parity"
        );
    }
    (sources, syntax, Evidence { source, native, typescript })
}
pub(super) fn with_program(
    case: &str,
    inspect: impl FnOnce(&execution::VerifiedExecution<'_>, &[u8], &Evidence),
) {
    assert!(authority::POSITIVE.contains(&case));
    let (sources, syntax, evidence) = authenticate(case);
    let entry = sources.verify_file_id(0).expect("literal main.zry entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("bound semantic input"),
    )
    .expect("all original declarations");
    let bodies = check_body_types(&declarations).expect("all original body capabilities");
    let context = execution_v1::discover(&bodies).expect("new original-before-generated bridge");
    let linear = verify_layouts(context.instances(), StorageTarget::Linear32V1)
        .expect("exact linear layout");
    let linux = verify_layouts(context.instances(), StorageTarget::LinuxX8664V1)
        .expect("exact Linux layout");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("runtime declarations"),
        &linear,
        &linux,
    )
    .expect("independent runtime authority");
    let claim = execution_v1::produce_claim(&context, &linear, &linux)
        .expect("one genuine new execution claim");
    let wire = execution::wire::encode(&claim).expect("new complete bounded wire");
    assert!(wire.starts_with(execution::wire::HEADER));
    assert!(
        zryna_ir::generic_v1::owned_v2::wire::v5::decode(&wire).is_err(),
        "old carrier remains distinct"
    );
    let program = execution::verify(
        execution::wire::decode(&wire).expect("new opaque decoded carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent new source/type/state/plan/runtime/scalar ABI issuer");
    assert_eq!(program.entry_file(), entry);
    assert_eq!(program.scalar_abi().exports().len(), 1);
    assert_eq!(
        program.scalar_abi().exports().next().expect("entry export").logical_name().as_str(),
        "entry"
    );
    assert_eq!(program.export_functions().len(), 1);
    inspect(&program, &wire, &evidence);
}
