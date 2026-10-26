//! Actual complete original sources and packets; no caller-constructed term or type maps.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::owned_v2;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{
        layouts::verify_layouts,
        owned_v5::{discover, produce_claim},
    },
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

pub(super) struct Evidence {
    pub(super) source: Vec<u8>,
    pub(super) native: Vec<u8>,
    pub(super) typescript: Vec<u8>,
    pub(super) runtime_expected: Vec<u8>,
    pub(super) resource_expected: Vec<u8>,
}
fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-clone-compact-dag")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(directory().join(name)).expect("frozen complete input")
}
fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&read(name)).expect("frozen JSON")
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for byte in bytes {
        write!(output, "{byte:02x}").expect("String formatting");
    }
    output
}
fn load(case: &str) -> Evidence {
    let freeze = read("SOURCE_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze)),
        "0aa6e4d817201b1ed64a01e3d08fc9469066b9d55a1684e01e915cf137b71f18"
    );
    let freeze: serde_json::Value = serde_json::from_slice(&freeze).expect("pre-provider freeze");
    let row = freeze["cases"]
        .as_array()
        .expect("three cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("frozen case");
    let source = read(row["source"].as_str().expect("source path"));
    let runtime_expected = read(row["expected"].as_str().expect("runtime expectation path"));
    let resource_expected =
        read(row["resourceExpected"].as_str().expect("resource expectation path"));
    for (bytes, hash) in [
        (&source, &row["sourceSha256"]),
        (&runtime_expected, &row["expectedSha256"]),
        (&resource_expected, &row["resourceExpectedSha256"]),
    ] {
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            hash.as_str().expect("pre-provider hash")
        );
    }
    let native = read(&format!("{case}-native.json"));
    let typescript = read(&format!("{case}-typescript.json"));
    let identities = json("packet-identities.json");
    for (provider, bytes) in [("native", &native), ("typescript", &typescript)] {
        let row = identities["records"]
            .as_array()
            .expect("six fresh packets")
            .iter()
            .find(|row| row["case"] == case && row["provider"] == provider)
            .expect("honest actual provider identity");
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), row["packetSha256"]);
    }
    Evidence { source, native, typescript, runtime_expected, resource_expected }
}
pub(super) fn with_program(
    case: &str,
    inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>, &[u8], &serde_json::Value, &Evidence),
) {
    let evidence = load(case);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(evidence.source.clone()).expect("exact complete literal"),
    }])
    .expect("source map");
    let lexed = zryna_frontend::native_lexer::lex(&sources).expect("actual native lexer");
    let raw = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
        .expect("actual native parser");
    let syntax = v5::verify_snapshot(raw, &sources).expect("actual native source authentication");
    for packet in [&evidence.native, &evidence.typescript] {
        let candidate =
            v5::verify_snapshot(v5::decode_snapshot(packet).expect("closed v5 packet"), &sources)
                .expect("complete source authentication");
        assert_eq!(syntax.files(), candidate.files());
    }
    let entry = sources.verify_file_id(0).expect("entry module");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original semantic input"),
    )
    .expect("all original declarations");
    let bodies = check_body_types(&declarations).expect("full all-payload source Clone capability");
    let context = discover(&bodies).expect("original ownership before complete v5 discovery");
    let linear =
        verify_layouts(context.instances(), StorageTarget::Linear32V1).expect("linear layout");
    let linux =
        verify_layouts(context.instances(), StorageTarget::LinuxX8664V1).expect("Linux layout");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("runtime declarations"),
        &linear,
        &linux,
    )
    .expect("independent runtime seal");
    let claim = produce_claim(&context, &linear, &linux).expect("genuine source-produced v5 claim");
    let wire = owned_v2::wire::v5::encode(&claim).expect("genuine v5 wire");
    let program = owned_v2::verify_v5(
        owned_v2::wire::v5::decode(&wire).expect("opaque v5 carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("GENUINE v5 source/IR seal required before crediting ANY backend budget rejection");
    let expected: serde_json::Value =
        serde_json::from_slice(&evidence.resource_expected).expect("frozen resources");
    check_sealed_shape(&program, &expected);
    inspect(&program, &wire, &expected, &evidence);
}
fn check_sealed_shape(program: &owned_v2::VerifiedOwnedProgram<'_>, expected: &serde_json::Value) {
    let clones = program.struct_clones().collect::<Vec<_>>();
    assert_eq!(clones.len(), 1);
    assert_eq!(
        clones[0].string_occurrences(),
        expected["possibleStringOccurrences"].as_u64().expect("independent all-variant census")
    );
    for (target, word) in [(StorageTarget::Linear32V1, 4), (StorageTarget::LinuxX8664V1, 8)] {
        for (role, width) in
            [("closedEnumKey", "enumPrivateLanes"), ("closedWrapKey", "wrapperPrivateLanes")]
        {
            let ty = program
                .layouts(target)
                .types()
                .find(|ty| {
                    hex(ty.key())
                        == expected[role].as_str().expect("independent closed nominal key")
                })
                .expect("actual source demanded nominal instance");
            assert_eq!(ty.size(), expected[width].as_u64().expect("small closed width") * word);
            assert_eq!(ty.key().len(), if role == "closedEnumKey" { 9 } else { 26 });
        }
    }
}
