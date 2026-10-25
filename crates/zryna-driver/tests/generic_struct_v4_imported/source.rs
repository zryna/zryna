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

pub(super) struct Evidence {
    pub(super) source: Vec<u8>,
    pub(super) native: Vec<u8>,
    pub(super) typescript: Vec<u8>,
    pub(super) additional: Vec<(&'static str, Vec<u8>)>,
}
pub(super) fn authenticate(case: &str) -> (SourceMap, VerifiedProjectSyntaxV5, Evidence) {
    assert_eq!(case, "imported-alias-decoy");
    let source = oracle::read_fixture("imported-alias-decoy/main.zry");
    let additional = ["values.zry", "z-leaf.zry"]
        .into_iter()
        .map(|name| (name, oracle::read_fixture(&format!("imported-alias-decoy/{name}"))))
        .collect::<Vec<_>>();
    let native = oracle::read_fixture("imported-alias-decoy-native.json");
    let typescript = oracle::read_fixture("imported-alias-decoy-typescript.json");
    let mut inputs = vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(source.clone()).expect("literal UTF8"),
    }];
    inputs.extend(additional.iter().map(|(name, bytes)| SourceFileInput {
        path: (*name).into(),
        text: String::from_utf8(bytes.clone()).expect("literal UTF8"),
    }));
    let sources = SourceMap::build(inputs).expect("three exact complete source literals");
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
    (sources, syntax, Evidence { source, native, typescript, additional })
}
pub(super) fn with_program(
    case: &str,
    inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>, &[u8], &Evidence),
) {
    assert_eq!(case, "imported-alias-decoy");
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
    let expected = oracle::json("imported-alias-decoy-expected.json");
    for layouts in [&linear, &linux] {
        let keys = layouts.types().map(|ty| hex(ty.key())).collect::<Vec<_>>();
        for role in ["rootCarrierI32", "nestedBoxPayload", "leafPayload"] {
            assert!(
                keys.iter().any(|key| key
                    == expected["closedKeys"][role].as_str().expect("independent closed identity")),
                "{role}: declaring-module closed nominal key in both sealed layouts"
            );
        }
    }
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

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for byte in bytes {
        write!(output, "{byte:02x}").expect("String formatting");
    }
    output
}
