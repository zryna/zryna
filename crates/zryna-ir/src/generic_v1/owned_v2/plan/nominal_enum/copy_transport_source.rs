//! Both complete source forms cross original bodies, independent layouts and the public seal.
use super::authorities::key;
use crate::generic_v1::{Failure, owned_v2};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};

struct Fixture {
    sources: SourceMap,
    syntax: VerifiedProjectSyntaxV5,
    graph: layout::raw::Graph,
    function_key: Vec<u8>,
}

impl Fixture {
    fn new(form: u8) -> Self {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-nominal-copy-transport");
        let values = std::fs::read_to_string(root.join("values.zry")).expect("frozen declarations");
        let main = std::fs::read_to_string(root.join("main.zry")).expect("frozen scalar wrappers");
        let files = if form == 1 {
            vec![SourceFileInput { path: "main.zry".into(), text: format!("{values}\n{main}") }]
        } else {
            vec![
                SourceFileInput {
                    path: "main.zry".into(),
                    text: format!("import {{transport}} from \"./values.zry\";\n{main}"),
                },
                SourceFileInput { path: "values.zry".into(), text: values },
            ]
        };
        let sources = SourceMap::build(files).expect("complete original source closure");
        let dto = std::fs::read(root.join(format!("{form}-reference.json")))
            .expect("independently authenticated packet DTO");
        let syntax =
            v5::verify_snapshot(v5::decode_snapshot(&dto).expect("untrusted packet DTO"), &sources)
                .expect("complete source/arena/token proof");
        let unit = syntax
            .files()
            .iter()
            .find(|unit| !unit.data_declarations.is_empty())
            .expect("original definitions");
        assert_eq!(unit.data_declarations.len(), 2);
        let graph = layouts_graph(&syntax, &sources, unit.id);
        let function = unit
            .functions
            .iter()
            .position(|function| function.name.text == "transport")
            .expect("original generic transport");
        let function_key =
            key(0x40, unit.id, u32::try_from(function).expect("function ordinal"), &[&[2]]);
        Self { sources, syntax, graph, function_key }
    }
    fn layouts(
        &self,
        graph: &layout::raw::Graph,
    ) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
        (
            layout::verify(graph, &self.sources, StorageTarget::Linear32V1)
                .expect("independent Linear32 claims"),
            layout::verify(graph, &self.sources, StorageTarget::LinuxX8664V1)
                .expect("independent Linux claims"),
        )
    }
    fn verify(
        &self,
        claim: &owned_v2::raw::Program,
        linear: &layout::VerifiedLayouts,
        linux: &layout::VerifiedLayouts,
    ) -> Result<usize, Failure> {
        let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
            zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux).expect("runtime claims"),
            linear,
            linux,
        )
        .expect("runtime declaration authority");
        let wire = owned_v2::wire::encode(claim)?;
        owned_v2::verify(
            owned_v2::wire::decode(&wire)?,
            &self.syntax,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            linear,
            linux,
            &runtime,
        )
        .map(|sealed| sealed.scalar_abi().exports().len())
    }
}

fn layouts_graph(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    module: u32,
) -> layout::raw::Graph {
    use layout::raw::{
        Declaration, Module, ModuleId, NodeId, NominalKind, TypeKind, TypeNode, Variant,
    };
    use zryna_layout::raw::TypeKind as Base;
    let unit = &syntax.files()[module as usize];
    let declarations = unit
        .data_declarations
        .iter()
        .enumerate()
        .map(|(index, data)| Declaration {
            module: ModuleId(module),
            index: u32::try_from(index).expect("declaration ordinal"),
            kind: NominalKind::Enum,
            parameters: 1,
            members: 2,
            span: sources.verify_span(data.span).expect("original declaration range"),
        })
        .collect::<Vec<_>>();
    let kinds = vec![
        TypeKind::Base(Base::Bool),
        TypeKind::Base(Base::I32),
        TypeKind::Base(Base::String),
        TypeKind::Result { okay: NodeId(0), error: NodeId(1) },
        TypeKind::Option { argument: NodeId(3) },
        TypeKind::Enum {
            module: ModuleId(module),
            declaration: 0,
            arguments: vec![NodeId(4)],
            variants: vec![
                Variant { ordinal: 0, payload: None },
                Variant { ordinal: 1, payload: Some(NodeId(4)) },
            ],
        },
        TypeKind::Enum {
            module: ModuleId(module),
            declaration: 1,
            arguments: vec![NodeId(2)],
            variants: vec![
                Variant { ordinal: 0, payload: None },
                Variant { ordinal: 1, payload: Some(NodeId(0)) },
            ],
        },
    ];
    let types = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| TypeNode {
            id: NodeId(u32::try_from(index).expect("literal type ordinal")),
            span: match index {
                5 => Some(declarations[0].span),
                6 => Some(declarations[1].span),
                _ => None,
            },
            kind,
        })
        .collect();
    layout::raw::Graph {
        modules: syntax
            .files()
            .iter()
            .map(|unit| Module {
                id: ModuleId(unit.id),
                source_file: sources.verify_file_id(unit.id).expect("source file"),
                data_declarations: u32::try_from(unit.data_declarations.len())
                    .expect("declarations"),
            })
            .collect(),
        declarations,
        types,
        program_roots: (0..7).map(NodeId).collect(),
    }
}

fn public_source_transport(form: u8) {
    let f = Fixture::new(form);
    owned_v2::check_original_bodies(&f.syntax, &f.sources)
        .expect("complete original opaque bodies admit concrete nominal Copy carriers");
    let (linear, linux) = f.layouts(&f.graph);
    for prefix in [0x13, 0x14, 0x15] {
        assert!(
            linear
                .types()
                .filter(|ty| ty.key().first() == Some(&prefix))
                .all(|ty| ty.drop_kind() == 0),
            "independent complete payload ownership"
        );
    }
    let claim = owned_v2::produce_claim(&f.syntax, &f.sources, &linear, &linux, &[&f.function_key])
        .expect("original branch/loop claim production");
    assert_eq!(
        f.verify(&claim, &linear, &linux).expect("public source-bound nominal Copy seal"),
        3
    );
    let pristine = owned_v2::wire::encode(&claim).expect("canonical pristine wire");
    let foreign = Fixture::new(form);
    assert!(
        foreign.verify(&claim, &linear, &linux).is_err(),
        "equal source text has foreign identity"
    );
    let mut hostile = claim.clone();
    let ty = linear
        .types()
        .find(|ty| ty.key().first() == Some(&0x13))
        .expect("nominal type")
        .id()
        .index();
    let block = hostile
        .graph
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .find(|block| {
            block.id != 0
                && block
                    .parameters
                    .iter()
                    .any(|parameter| parameter.ty == crate::generic_v1::raw::Type::Stored(ty))
        })
        .expect("actual source nominal phi");
    block
        .parameters
        .iter_mut()
        .find(|parameter| parameter.ty == crate::generic_v1::raw::Type::Stored(ty))
        .expect("nominal phi parameter")
        .ty = crate::generic_v1::raw::Type::Stored(2);
    assert!(
        f.verify(&hostile, &linear, &linux).is_err(),
        "phi cannot replace exact nominal identity with String"
    );
    let mut layout_attack = f.graph.clone();
    let layout::raw::TypeKind::Enum { variants, .. } = &mut layout_attack.types[5].kind else {
        panic!("literal Box layout");
    };
    variants[1].payload = Some(layout::raw::NodeId(2));
    let (wrong_linear, wrong_linux) = f.layouts(&layout_attack);
    assert!(
        owned_v2::produce_claim(
            &f.syntax,
            &f.sources,
            &wrong_linear,
            &wrong_linux,
            &[&f.function_key]
        )
        .is_err(),
        "physically valid inactive String payload cannot replace exact original substitution"
    );
    assert_eq!(
        f.verify(&claim, &linear, &linux).expect("pristine source seal after every hostile retry"),
        3
    );
    assert_eq!(owned_v2::wire::encode(&claim).expect("pristine wire"), pristine);
}

#[test]
fn joined_original_nominal_copy_transport_reaches_public_seal_and_hostile_replay() {
    public_source_transport(1);
}

#[test]
fn imported_original_nominal_copy_transport_reaches_public_seal_and_hostile_replay() {
    public_source_transport(2);
}
