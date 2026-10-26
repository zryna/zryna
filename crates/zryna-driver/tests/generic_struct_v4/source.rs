//! Native candidate DTOs and TypeScript packets authenticate exact original/derived bytes.
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

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Construction,
    DeclarationOrder,
    Reordered,
}
impl Case {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Construction => "construction",
            Self::DeclarationOrder => "declaration-order",
            Self::Reordered => "reordered",
        }
    }
    pub(super) const fn owned(self) -> bool {
        !matches!(self, Self::Construction)
    }
    fn original(self) -> &'static str {
        match self {
            Self::Construction => {
                include_str!("../../../../tests/m7-generic-struct-v4/construction.zry")
            }
            Self::DeclarationOrder => {
                include_str!("../../../../tests/m7-generic-struct-v4/fault-declaration-order.zry")
            }
            Self::Reordered => {
                include_str!("../../../../tests/m7-generic-struct-v4/fault-reordered-spelling.zry")
            }
        }
    }
}

fn authenticate(text: String, packet: &[u8]) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("exact source map");
    let frozen =
        v5::verify_snapshot(v5::decode_snapshot(packet).expect("untrusted TS packet"), &sources)
            .expect("independent TS source authentication");
    let lexed = zryna_frontend::native_lexer::lex(&sources).expect("actual native source lexer");
    let raw = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
        .expect("actual native parser candidate");
    let syntax = v5::verify_snapshot(raw, &sources).expect("actual native source authentication");
    assert_eq!(syntax.files(), frozen.files(), "complete native/TS original source facts");
    (sources, syntax)
}

pub(super) fn with_program(
    case: Case,
    inspect: impl FnOnce(&owned_v2::VerifiedOwnedProgram<'_>, &[u8]),
) {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generic_struct_v4/fixtures");
    let original = if matches!(case, Case::Construction) {
        include_bytes!("../../../../tests/m7-generic-struct-v4/construction-reference.json")
            .to_vec()
    } else {
        std::fs::read(directory.join(format!("{}-original.json", case.name())))
            .expect("fixed original TS packet")
    };
    let (original_sources, original_syntax) = authenticate(case.original().to_owned(), &original);
    owned_v2::check_original_bodies_v4(&original_syntax, &original_sources)
        .expect("opaque original ownership before adding export");
    let text = format!("{}\nexport function entry(): i32 {{ return score(); }}\n", case.original());
    assert!(text.starts_with(case.original()), "frozen source is an exact unchanged prefix");
    let packet = std::fs::read(directory.join(format!("{}-exported.json", case.name())))
        .expect("fixed explicit-export TS packet");
    let (sources, syntax) = authenticate(text, &packet);
    let entry = sources.verify_file_id(0).expect("entry module");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("opaque original body types");
    let instances = discover(&bodies).expect("v4 original ownership and demanded instances");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("original linear layout");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("original Linux layout");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("runtime declarations"),
        &linear,
        &linux,
    )
    .expect("independent runtime authority");
    let claim = produce_claim(&instances, &linear, &linux).expect("untrusted v4 claims");
    let wire = owned_v2::wire::v4::encode(&claim).expect("bounded v4 message");
    let program = owned_v2::verify_v4(
        owned_v2::wire::v4::decode(&wire).expect("distinct v4 carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("source/type/ownership/runtime v4 issuer");
    assert_eq!(program.scalar_abi().exports().len(), 1, "only explicit scalar wrapper exported");
    inspect(&program, &wire);
}
