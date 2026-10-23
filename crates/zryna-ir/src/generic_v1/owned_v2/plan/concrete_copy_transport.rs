//! Independent nested-family loop carriers and hostile owner/type claims, without source production.
use super::*;
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};

const AT: UntrustedSpan = UntrustedSpan { file: 0, start: 0, end: 1 };

fn definition(id: u32, ty: u32) -> graph::Definition {
    graph::Definition { id, ty: graph::Type::Stored(ty) }
}
fn instruction(id: u32, ty: graph::Type, operation: graph::Operation) -> graph::Instruction {
    graph::Instruction { result: graph::Definition { id, ty }, span: AT, operation }
}
fn edge(target: u32, arguments: &[u32]) -> graph::Edge {
    graph::Edge { target, arguments: arguments.to_vec() }
}

struct Fixture {
    claim: raw::Program,
    sources: SourceMap,
    linear: VerifiedLayouts,
    linux: VerifiedLayouts,
    other_nested: u32,
}
impl Fixture {
    #[allow(
        clippy::too_many_lines,
        reason = "Keep the independent literal CFG and exact cleanup witness visible together."
    )]
    fn new() -> Self {
        use layout::raw::{Graph, Module, ModuleId, NodeId, TypeKind, TypeNode};
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "raw.zry".into(), text: "x".into() }])
                .expect("raw span authority only; no source seal is claimed");
        let graph = Graph {
            modules: vec![Module {
                id: ModuleId(0),
                source_file: sources.verify_file_id(0).expect("file"),
                data_declarations: 0,
            }],
            declarations: vec![],
            types: vec![
                TypeKind::Base(zryna_layout::raw::TypeKind::Bool),
                TypeKind::Base(zryna_layout::raw::TypeKind::I32),
                TypeKind::Base(zryna_layout::raw::TypeKind::String),
                TypeKind::Result { okay: NodeId(0), error: NodeId(1) },
                TypeKind::Option { argument: NodeId(3) },
                TypeKind::Option { argument: NodeId(0) },
                TypeKind::Option { argument: NodeId(5) },
            ]
            .into_iter()
            .enumerate()
            .map(|(id, kind)| TypeNode {
                id: NodeId(u32::try_from(id).expect("small graph")),
                span: None,
                kind,
            })
            .collect(),
            program_roots: (0..7).map(NodeId).collect(),
        };
        let linear = layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("independent layout");
        let linux = layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("independent Linux layout");
        let result = linear
            .types()
            .find(|t| t.key().first() == Some(&0x15))
            .expect("Result<bool,i32>")
            .id()
            .index();
        let nested = linear
            .types()
            .find(|t| {
                t.key()
                    == [
                        0x14, 1, 0, 0, 0, 15, 0, 0, 0, 0x15, 2, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0,
                        1,
                    ]
            })
            .expect("literal Option<Result<bool,i32>> identity")
            .id()
            .index();
        let other_nested = linear
            .types()
            .find(|t| t.key() == [0x14, 1, 0, 0, 0, 10, 0, 0, 0, 0x14, 1, 0, 0, 0, 1, 0, 0, 0, 0])
            .expect("literal Option<Option<bool>> identity")
            .id()
            .index();
        assert_ne!(nested, other_nested);
        assert_eq!(
            linear.types().find(|t| t.id().index() == nested).expect("nested").drop_kind(),
            0
        );
        let stored = graph::Type::Stored;
        let blocks = vec![
            graph::Block {
                id: 0,
                span: AT,
                parameters: vec![definition(0, 2), definition(1, 0)],
                instructions: vec![
                    instruction(2, stored(0), graph::Operation::BoolLiteral(true)),
                    instruction(
                        3,
                        stored(result),
                        graph::Operation::ClosedEnumConstruct {
                            ty: result,
                            ordinal: 0,
                            payload: Some(2),
                        },
                    ),
                    instruction(
                        4,
                        stored(nested),
                        graph::Operation::ClosedEnumConstruct {
                            ty: nested,
                            ordinal: 1,
                            payload: Some(3),
                        },
                    ),
                    instruction(
                        5,
                        stored(nested),
                        graph::Operation::ClosedEnumConstruct {
                            ty: nested,
                            ordinal: 0,
                            payload: None,
                        },
                    ),
                ],
                terminator: graph::Terminator::Jump(edge(1, &[4, 5, 1])),
            },
            graph::Block {
                id: 1,
                span: AT,
                parameters: vec![definition(6, nested), definition(7, nested), definition(8, 0)],
                instructions: vec![],
                terminator: graph::Terminator::Branch {
                    condition: 8,
                    yes: edge(2, &[]),
                    no: edge(3, &[]),
                },
            },
            graph::Block {
                id: 2,
                span: AT,
                parameters: vec![],
                instructions: vec![
                    instruction(9, stored(2), graph::Operation::Unit),
                    instruction(10, graph::Type::Unit, graph::Operation::Unit),
                    instruction(11, stored(0), graph::Operation::BoolLiteral(false)),
                ],
                terminator: graph::Terminator::Jump(edge(1, &[7, 6, 11])),
            },
            graph::Block {
                id: 3,
                span: AT,
                parameters: vec![],
                instructions: vec![instruction(12, stored(1), graph::Operation::I32Literal(7))],
                terminator: graph::Terminator::Return(12),
            },
        ];
        let claim = raw::Program {
            graph: graph::Program {
                modules: vec![graph::Module { id: 0, functions: 1 }],
                declarations: vec![graph::Declaration {
                    module: 0,
                    function: 0,
                    parameters: 0,
                    span: AT,
                }],
                type_keys: linear.types().map(|t| t.key().to_vec()).collect(),
                universe: *linear.universe_identity(),
                linear32: *linear.fingerprint(),
                linux_x86_64: *linux.fingerprint(),
                functions: vec![graph::Function {
                    key: vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0],
                    span: AT,
                    public_export: None,
                    parameters: vec![stored(2), stored(0)],
                    result: stored(1),
                    blocks,
                }],
            },
            extensions: vec![vec![
                raw::Extension {
                    result: 9,
                    operation: raw::Operation::StringLiteral(b"iteration".into()),
                },
                raw::Extension { result: 10, operation: raw::Operation::Drop(9) },
            ]],
            plans: vec![],
        };
        Self { claim, sources, linear, linux, other_nested }
    }
    fn typed(&self, claim: &raw::Program) -> Result<(), Failure> {
        let inventory = crate::generic_v1::inventory::check(
            &claim.graph,
            &self.sources,
            &self.linear,
            &self.linux,
        )?;
        for layouts in [&self.linear, &self.linux] {
            super::super::typed::check(claim, &self.sources, layouts, &inventory)?;
        }
        Ok(())
    }
}

#[test]
fn independent_nested_copy_loop_transport_preserves_exact_owner_plan() {
    let f = Fixture::new();
    f.typed(&f.claim).expect("independent typed nested family headers");
    let expected = vec![raw::Plan {
        steps: vec![
            raw::Step { block: 2, position: 0, failure: true, end_loans: vec![], cleanup: vec![0] },
            raw::Step {
                block: 3,
                position: 1,
                failure: false,
                end_loans: vec![],
                cleanup: vec![0],
            },
        ],
    }];
    for layouts in [&f.linear, &f.linux] {
        assert_eq!(
            derive(&f.claim.graph, &f.claim.extensions, layouts).expect("literal nested loop plan"),
            expected
        );
    }
    let mut same_width_wrong_identity = f.claim.clone();
    same_width_wrong_identity.graph.functions[0].blocks[1].parameters[0].ty =
        graph::Type::Stored(f.other_nested);
    assert!(
        f.typed(&same_width_wrong_identity).is_err(),
        "equal physical width grants no nominal identity conversion"
    );
    let mut owned_copy = f.claim.clone();
    owned_copy.graph.functions[0].blocks[2].instructions[0].operation =
        graph::Operation::Copy { value: 0 };
    owned_copy.extensions[0].remove(0);
    assert!(f.typed(&owned_copy).is_err(), "sealed layout excludes copying String");
    let mut altered_cleanup = f.claim.clone();
    altered_cleanup.extensions[0][1].operation = raw::Operation::Drop(0);
    assert!(
        derive(&altered_cleanup.graph, &altered_cleanup.extensions, &f.linear).is_err(),
        "backedge must retain the original root and release the iteration local"
    );
    for attack in 0..3 {
        let mut hostile = f.claim.clone();
        let graph::Terminator::Jump(e) = &mut hostile.graph.functions[0].blocks[2].terminator
        else {
            panic!("backedge");
        };
        match attack {
            0 => {
                e.arguments.pop();
            }
            1 => e.arguments[0] = u32::MAX,
            _ => e.target = 0,
        }
        assert!(f.typed(&hostile).is_err());
    }
    f.typed(&f.claim).expect("pristine independent typed recovery");
    assert_eq!(
        derive(&f.claim.graph, &f.claim.extensions, &f.linear).expect("pristine plan recovery"),
        expected
    );
}
