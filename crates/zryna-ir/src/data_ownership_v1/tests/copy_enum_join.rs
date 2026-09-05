use super::*;
use zryna_layout::{TypeCategory, VerifiedLayouts};

mod hostile;

struct Fixture {
    sources: SourceMap,
    linear: VerifiedLayouts,
    linux: VerifiedLayouts,
}

impl Fixture {
    fn new() -> Self {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: "enum Choice { A, B } function choose(value: Choice): i32 { return 0; }".into(),
        }])
        .expect("source map");
        let file = sources.verify_file_id(0).expect("file");
        let kinds = [
            raw_layout::TypeKind::Bool,
            raw_layout::TypeKind::I32,
            raw_layout::TypeKind::String,
            raw_layout::TypeKind::Enum {
                module: raw_layout::ModuleId(0),
                declaration: 0,
                variants: vec![
                    raw_layout::Variant { ordinal: 0, payload: None },
                    raw_layout::Variant { ordinal: 1, payload: None },
                ],
            },
        ];
        let graph = raw_layout::Graph {
            modules: vec![raw_layout::Module {
                id: raw_layout::ModuleId(0),
                source_file: file,
                data_declarations: 1,
            }],
            types: kinds
                .into_iter()
                .enumerate()
                .map(|(id, kind)| raw_layout::TypeNode {
                    id: raw_layout::NodeId(u32::try_from(id).expect("small graph")),
                    span: (id == 3).then(|| sources.span(file, 0, 20).expect("enum span")),
                    kind,
                })
                .collect(),
            program_roots: (0..4).map(raw_layout::NodeId).collect(),
        };
        let linear =
            zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear");
        let linux =
            zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1).expect("native");
        Self { sources, linear, linux }
    }

    fn seed(&self) -> raw::Program {
        let mut raw = program(&self.sources, &self.linear, &self.linux);
        let ty = |category| {
            raw::TypeId(
                self.linear
                    .types()
                    .find(|ty| ty.category() == category)
                    .expect("type")
                    .id()
                    .index(),
            )
        };
        let enumeration = ty(TypeCategory::Enum);
        assert_eq!(
            self.linear
                .types()
                .find(|ty| ty.category() == TypeCategory::Enum)
                .expect("enum")
                .drop_kind(),
            0
        );
        raw.modules[0].data_declarations = 1;
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
        function.entry_export = None;
        function.parameters = vec![
            value(0, enumeration),
            value(1, ty(TypeCategory::I32)),
            value(2, ty(TypeCategory::String)),
        ];
        function.result = ty(TypeCategory::I32);
        function.places = [
            (enumeration, raw::PlaceKind::Temporary(raw::ValueId(3))),
            (ty(TypeCategory::String), raw::PlaceKind::Parameter(2)),
            (enumeration, raw::PlaceKind::Parameter(0)),
        ]
        .into_iter()
        .enumerate()
        .map(|(id, (ty, kind))| raw::Place {
            id: raw::PlaceId(u32::try_from(id).expect("small places")),
            ty,
            span,
            kind,
        })
        .collect();
        function.blocks[0].instructions = vec![
            raw::Instruction {
                span,
                result: Some(value(3, enumeration)),
                kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(2) },
            },
            raw::Instruction {
                span,
                result: None,
                kind: raw::InstructionKind::InitializePlace {
                    place: raw::PlaceId(0),
                    value: raw::ValueId(3),
                },
            },
        ];
        function.blocks[0].terminators[0].kind = raw::Terminator::EnumMatch {
            place: raw::PlaceId(0),
            arms: (0..2)
                .map(|variant| raw::EnumArm {
                    variant,
                    edge: raw::Edge { target: raw::BlockId(variant + 1), arguments: vec![] },
                })
                .collect(),
        };
        for borrow in 0..2 {
            function.blocks.push(restored_arm(borrow, span));
        }
        function.blocks.push(raw::Block {
            id: raw::BlockId(3),
            parameters: vec![value(4, function.result)],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(4),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        });
        function.cleanup_plans = vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        }];
        raw
    }

    fn verify(
        &self,
        raw: raw::Program,
    ) -> Result<super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
        verify(
            raw,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            self.linear.clone(),
            self.linux.clone(),
        )
    }

    fn reject(&self, raw: raw::Program, code: &str, message: Option<&str>) {
        let first = self.verify(raw.clone()).expect_err("hostile Copy join");
        assert!(
            first.iter().any(|error| error.code() == code
                && message.is_none_or(|message| error.message() == message)),
            "{first:?}"
        );
        assert_eq!(first, self.verify(raw).expect_err("deterministic rejection"));
        self.verify(self.seed()).expect("authenticated recovery");
    }
}

#[test]
fn copy_enum_join_restores_private_temporary_with_original_once_evaluated_ssa() {
    let fixture = Fixture::new();
    for _ in 0..2 {
        let verified = fixture.verify(fixture.seed()).expect("equal restored join state");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        assert!(function.public_export().is_none());
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(
            blocks[0]
                .instructions()
                .map(super::super::VerifiedInstruction::kind)
                .collect::<Vec<_>>(),
            [VerifiedInstructionKind::CopyFromPlace, VerifiedInstructionKind::InitializePlace]
        );
        for (borrow, block) in blocks[1..3].iter().enumerate() {
            let instructions = block.instructions().collect::<Vec<_>>();
            assert_eq!(
                instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
                [
                    VerifiedInstructionKind::BeginBorrow,
                    VerifiedInstructionKind::BorrowWrite,
                    VerifiedInstructionKind::EndBorrow
                ]
            );
            assert_eq!(
                instructions[1]
                    .value_operands()
                    .map(super::super::ValueIdentity::index)
                    .collect::<Vec<_>>(),
                [3]
            );
            assert_eq!(instructions[0].place_operands().next().expect("private temp").index(), 0);
            assert_eq!(
                instructions
                    .iter()
                    .map(|instruction| instruction
                        .borrow()
                        .expect("same lexical authority")
                        .index())
                    .collect::<Vec<_>>(),
                [u32::try_from(borrow).expect("small borrow"); 3]
            );
            assert_eq!(
                instructions[0].borrow_access(),
                Some(super::super::VerifiedBorrowAccess::Exclusive)
            );
            assert!(
                instructions
                    .iter()
                    .all(|instruction| instruction.derived_drop_actions().len() == 0)
            );
            assert_eq!(borrow, block.id().index() as usize - 1);
        }
        assert_eq!(
            blocks[3]
                .terminator()
                .derived_drop_actions()
                .map(|drop| drop.root().index())
                .collect::<Vec<_>>(),
            [1],
            "Copy restoration neither consumes nor adds owned cleanup"
        );
    }
}

fn restored_arm(borrow: u32, span: zryna_source::Span) -> raw::Block {
    raw::Block {
        id: raw::BlockId(borrow + 1),
        parameters: vec![],
        instructions: vec![
            begin_borrow(borrow, 0, raw::BorrowAccess::Exclusive, span),
            raw::Instruction {
                span,
                result: None,
                kind: raw::InstructionKind::BorrowWrite {
                    borrow: raw::BorrowId(borrow),
                    value: raw::ValueId(3),
                },
            },
            end_borrow(borrow, span),
        ],
        terminators: vec![raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Jump(raw::Edge {
                target: raw::BlockId(3),
                arguments: vec![raw::ValueId(1)],
            }),
        }],
    }
}
