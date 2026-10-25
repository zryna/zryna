//! Literal nominal layouts and raw CFG constructors, without any source producer.
use super::super::{Failure, VerifiedLayouts, graph, raw};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap, Span, UntrustedSpan};

pub(super) const AT: UntrustedSpan = UntrustedSpan { file: 0, start: 0, end: 1 };

pub(super) fn key(tag: u8, module: u32, declaration: u32, arguments: &[&[u8]]) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(&module.to_le_bytes());
    bytes.extend_from_slice(&declaration.to_le_bytes());
    if tag == 0x13 || tag == 0x40 {
        bytes.extend_from_slice(&u32::try_from(arguments.len()).expect("arity").to_le_bytes());
        for argument in arguments {
            bytes.extend_from_slice(&u32::try_from(argument.len()).expect("key").to_le_bytes());
            bytes.extend_from_slice(argument);
        }
    }
    bytes
}

pub(super) fn layouts_graph(module: u32, spans: &[Span]) -> layout::raw::Graph {
    use layout::raw::{Declaration, ModuleId, NodeId, NominalKind, TypeKind, TypeNode, Variant};
    use zryna_layout::raw::TypeKind as Base;
    let variants = |payloads: &[Option<u32>]| {
        payloads
            .iter()
            .enumerate()
            .map(|(ordinal, payload)| Variant {
                ordinal: u32::try_from(ordinal).expect("ordinal"),
                payload: payload.map(NodeId),
            })
            .collect::<Vec<_>>()
    };
    let kinds = vec![
        TypeKind::Base(Base::Bool),
        TypeKind::Base(Base::I32),
        TypeKind::Base(Base::String),
        TypeKind::Result { okay: NodeId(2), error: NodeId(1) },
        TypeKind::Option { argument: NodeId(3) },
        TypeKind::Enum {
            module: ModuleId(module),
            declaration: 0,
            arguments: vec![NodeId(2), NodeId(1)],
            variants: variants(&[None, Some(2), Some(1), Some(4)]),
        },
        TypeKind::Enum {
            module: ModuleId(module),
            declaration: 1,
            arguments: vec![NodeId(2)],
            variants: variants(&[None, Some(1)]),
        },
        TypeKind::Enum {
            module: ModuleId(module),
            declaration: 2,
            arguments: vec![NodeId(1)],
            variants: variants(&[None, Some(2)]),
        },
        TypeKind::Base(Base::Enum {
            module: ModuleId(module),
            declaration: 3,
            variants: variants(&[None, Some(2)]),
        }),
    ];
    let types = kinds
        .into_iter()
        .enumerate()
        .map(|(id, kind)| TypeNode {
            id: NodeId(u32::try_from(id).expect("node")),
            span: match id {
                5 => Some(spans[0]),
                6 => Some(spans[1]),
                7 => Some(spans[2]),
                8 => Some(spans[3]),
                _ => None,
            },
            kind,
        })
        .collect();
    layout::raw::Graph {
        modules: vec![],
        declarations: [(2, 4), (1, 2), (1, 2), (0, 2)]
            .into_iter()
            .enumerate()
            .map(|(index, (parameters, members))| Declaration {
                module: ModuleId(module),
                index: u32::try_from(index).expect("declaration"),
                kind: NominalKind::Enum,
                parameters,
                members,
                span: spans[index],
            })
            .collect(),
        types,
        program_roots: vec![NodeId(4)],
    }
}

pub(super) struct Fixture {
    pub(super) sources: SourceMap,
    pub(super) graph: layout::raw::Graph,
    pub(super) linear: VerifiedLayouts,
    pub(super) linux: VerifiedLayouts,
}
impl Fixture {
    pub(super) fn new() -> Self {
        use layout::raw::{NodeId, TypeKind, TypeNode, Variant};
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "independent.zry".into(),
            text: "x".into(),
        }])
        .expect("raw span authority only, no syntax/source seal");
        let file = sources.verify_file_id(0).expect("file");
        let span = sources.span(file, 0, 1).expect("raw span");
        let mut graph = layouts_graph(0, &[span; 4]);
        graph.modules.push(layout::raw::Module {
            id: layout::raw::ModuleId(0),
            source_file: file,
            data_declarations: 4,
        });
        for kind in [
            TypeKind::Result { okay: NodeId(1), error: NodeId(2) },
            TypeKind::Option { argument: NodeId(9) },
            TypeKind::Enum {
                module: layout::raw::ModuleId(0),
                declaration: 0,
                arguments: vec![NodeId(1), NodeId(2)],
                variants: vec![
                    Variant { ordinal: 0, payload: None },
                    Variant { ordinal: 1, payload: Some(NodeId(1)) },
                    Variant { ordinal: 2, payload: Some(NodeId(2)) },
                    Variant { ordinal: 3, payload: Some(NodeId(10)) },
                ],
            },
        ] {
            let id = NodeId(u32::try_from(graph.types.len()).expect("raw node"));
            let nominal = matches!(kind, TypeKind::Enum { .. });
            graph.types.push(TypeNode { id, span: nominal.then_some(span), kind });
        }
        let linear = layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("independent nominal linear layout");
        let linux = layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("independent nominal Linux layout");
        Self { sources, graph, linear, linux }
    }
    pub(super) fn id(&self, tag: u8, declaration: u32, arguments: &[&[u8]]) -> u32 {
        let bytes = key(tag, 0, declaration, arguments);
        self.linear
            .types()
            .find(|ty| ty.key() == bytes)
            .expect("literal complete nominal key")
            .id()
            .index()
    }
    pub(super) fn choice(&self) -> u32 {
        self.id(0x13, 0, &[&[2], &[1]])
    }
    pub(super) fn other_choice(&self) -> u32 {
        self.id(0x13, 0, &[&[1], &[2]])
    }
    pub(super) fn marker(&self) -> u32 {
        self.id(0x13, 1, &[&[2]])
    }
    pub(super) fn fixed(&self) -> u32 {
        self.id(0x13, 2, &[&[1]])
    }
    pub(super) fn plain(&self) -> u32 {
        self.id(0x11, 3, &[])
    }
    pub(super) fn nested(&self) -> u32 {
        self.linear
            .types()
            .find(|ty| {
                ty.key()
                    == [
                        0x14, 1, 0, 0, 0, 15, 0, 0, 0, 0x15, 2, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0,
                        1,
                    ]
            })
            .expect("nested Option")
            .id()
            .index()
    }
    pub(super) fn claim(
        &self,
        blocks: Vec<graph::Block>,
        extensions: Vec<raw::Extension>,
    ) -> raw::Program {
        raw::Program {
            graph: graph::Program {
                modules: vec![graph::Module { id: 0, functions: 1 }],
                declarations: vec![graph::Declaration {
                    module: 0,
                    function: 0,
                    parameters: 0,
                    span: AT,
                }],
                type_keys: self.linear.types().map(|ty| ty.key().to_vec()).collect(),
                universe: *self.linear.universe_identity(),
                linear32: *self.linear.fingerprint(),
                linux_x86_64: *self.linux.fingerprint(),
                functions: vec![graph::Function {
                    key: key(0x41, 0, 0, &[]),
                    span: AT,
                    public_export: None,
                    parameters: blocks[0].parameters.iter().map(|d| d.ty).collect(),
                    result: graph::Type::Stored(1),
                    blocks,
                }],
            },
            extensions: vec![extensions],
            plans: vec![],
        }
    }
    pub(super) fn typed(&self, claim: &raw::Program) -> Result<(), Failure> {
        let inventory = crate::generic_v1::inventory::check(
            &claim.graph,
            &self.sources,
            &self.linear,
            &self.linux,
        )?;
        for layouts in [&self.linear, &self.linux] {
            super::super::super::typed::check(claim, &self.sources, layouts, &inventory)?;
        }
        Ok(())
    }
}

pub(super) fn d(id: u32, ty: graph::Type) -> graph::Definition {
    graph::Definition { id, ty }
}
pub(super) fn stored(id: u32, ty: u32) -> graph::Definition {
    d(id, graph::Type::Stored(ty))
}
pub(super) fn ins(id: u32, ty: graph::Type, operation: graph::Operation) -> graph::Instruction {
    graph::Instruction { result: d(id, ty), span: AT, operation }
}
pub(super) fn int(id: u32, value: i32) -> graph::Instruction {
    ins(id, graph::Type::Stored(1), graph::Operation::I32Literal(value))
}
pub(super) fn block(
    id: u32,
    parameters: Vec<graph::Definition>,
    instructions: Vec<graph::Instruction>,
    terminator: graph::Terminator,
) -> graph::Block {
    graph::Block { id, parameters, instructions, span: AT, terminator }
}
pub(super) fn arm(ordinal: u32, binding: Option<graph::Definition>, target: u32) -> graph::Arm {
    graph::Arm { ordinal, binding, edge: graph::Edge { target, arguments: vec![] } }
}
pub(super) fn rejection(result: Result<(), Failure>) {
    let Failure::Diagnostics(errors) = result.expect_err("nominal hostile claim must reject")
    else {
        panic!("diagnostic")
    };
    assert_eq!(errors[0].code, "ZRYNA-I7001");
}
