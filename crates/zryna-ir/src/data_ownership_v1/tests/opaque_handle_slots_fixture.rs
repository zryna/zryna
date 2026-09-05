use super::*;
use zryna_layout::{TypeCategory, VerifiedLayouts};

pub(super) struct Fixture {
    pub(super) sources: SourceMap,
    pub(super) linear: VerifiedLayouts,
    pub(super) linux: VerifiedLayouts,
    pub(super) types: [raw::TypeId; 6],
}

impl Fixture {
    pub(super) fn new() -> Self {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: "export function id(value: i32): i32 { return value; }".into(),
        }])
        .expect("source map");
        let file = sources.verify_file_id(0).expect("file");
        let field = |ordinal, ty| raw_layout::Field { ordinal, ty: raw_layout::NodeId(ty) };
        let kinds = vec![
            raw_layout::TypeKind::Bool,
            raw_layout::TypeKind::I32,
            raw_layout::TypeKind::String,
            raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(2) },
            raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(2) },
            raw_layout::TypeKind::Struct {
                module: raw_layout::ModuleId(0),
                declaration: 0,
                fields: vec![field(0, 3), field(1, 4), field(2, 3)],
            },
            raw_layout::TypeKind::Enum {
                module: raw_layout::ModuleId(0),
                declaration: 1,
                variants: vec![
                    raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(5)) },
                    raw_layout::Variant { ordinal: 1, payload: None },
                ],
            },
            raw_layout::TypeKind::FixedArray { element: raw_layout::NodeId(6), length: 1 },
            raw_layout::TypeKind::Vec { element: raw_layout::NodeId(7) },
        ];
        let graph = raw_layout::Graph {
            modules: vec![raw_layout::Module {
                id: raw_layout::ModuleId(0),
                source_file: file,
                data_declarations: 2,
            }],
            types: kinds
                .into_iter()
                .enumerate()
                .map(|(id, kind)| raw_layout::TypeNode {
                    id: raw_layout::NodeId(u32::try_from(id).expect("small graph")),
                    span: match id {
                        5 => Some(sources.span(file, 0, 6).expect("struct span")),
                        6 => Some(sources.span(file, 7, 13).expect("enum span")),
                        _ => None,
                    },
                    kind,
                })
                .collect(),
            program_roots: vec![raw_layout::NodeId(8)],
        };
        let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("linear layout");
        let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("native layout");
        let types = [
            TypeCategory::Shared,
            TypeCategory::Weak,
            TypeCategory::Struct,
            TypeCategory::Enum,
            TypeCategory::FixedArray,
            TypeCategory::Vec,
        ]
        .map(|category| {
            raw::TypeId(
                linear.types().find(|ty| ty.category() == category).expect("type").id().index(),
            )
        });
        Self { sources, linear, linux, types }
    }

    pub(super) fn seed(&self) -> raw::Program {
        let mut raw = program(&self.sources, &self.linear, &self.linux);
        let [shared, weak, structure, enumeration, array, vector] = self.types;
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
        function.entry_export = None;
        function.parameters = vec![value(0, shared), value(1, weak), value(2, vector)];
        function.result = vector;
        let value_types =
            [shared, weak, vector, shared, weak, structure, enumeration, array, vector, vector];
        function.places = value_types
            .into_iter()
            .enumerate()
            .map(|(index, ty)| {
                let id = u32::try_from(index).expect("small owner arena");
                raw::Place {
                    id: raw::PlaceId(id),
                    ty,
                    span,
                    kind: if id < 3 {
                        raw::PlaceKind::Parameter(id)
                    } else {
                        raw::PlaceKind::Temporary(raw::ValueId(id))
                    },
                }
            })
            .collect();
        function.blocks[0].instructions = self.instructions(span);
        function.blocks[0].terminators[0].kind =
            raw::Terminator::Return { value: raw::ValueId(9), cleanup: raw::CleanupPlanId(3) };
        function.cleanup_plans = [vec![2, 1, 0], vec![3, 2, 1, 0], vec![7, 2, 1], vec![1]]
            .into_iter()
            .enumerate()
            .map(|(id, roots)| raw::CleanupPlan {
                id: raw::CleanupPlanId(u32::try_from(id).expect("four sites")),
                span,
                actions: roots
                    .into_iter()
                    .map(|id| raw::DropAction::DropPlace(raw::PlaceId(id)))
                    .collect(),
            })
            .collect();
        raw
    }

    fn instructions(&self, span: zryna_source::Span) -> Vec<raw::Instruction> {
        let [shared, weak, structure, enumeration, array, vector] = self.types;
        let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
        let instructions = vec![
            (
                Some(value(3, shared)),
                raw::InstructionKind::SharedClone {
                    place: raw::PlaceId(0),
                    cleanup: raw::CleanupPlanId(0),
                },
            ),
            (
                Some(value(4, weak)),
                raw::InstructionKind::WeakClone {
                    place: raw::PlaceId(1),
                    cleanup: raw::CleanupPlanId(1),
                },
            ),
            (
                Some(value(5, structure)),
                raw::InstructionKind::StructConstruct {
                    fields: vec![raw::ValueId(3), raw::ValueId(4), raw::ValueId(0)],
                    cleanup: None,
                },
            ),
            (
                Some(value(6, enumeration)),
                raw::InstructionKind::EnumConstruct {
                    variant: 0,
                    payload: Some(raw::ValueId(5)),
                    cleanup: None,
                },
            ),
            (
                Some(value(7, array)),
                raw::InstructionKind::FixedArrayConstruct {
                    elements: vec![raw::ValueId(6)],
                    cleanup: None,
                },
            ),
            (
                Some(value(8, vector)),
                raw::InstructionKind::VecConstruct {
                    elements: vec![raw::ValueId(7)],
                    cleanup: raw::CleanupPlanId(2),
                },
            ),
            (
                None,
                raw::InstructionKind::ReplacePlace {
                    place: raw::PlaceId(2),
                    value: raw::ValueId(8),
                },
            ),
            (
                Some(value(9, vector)),
                raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(2) },
            ),
        ];
        instructions
            .into_iter()
            .map(|(result, kind)| raw::Instruction { result, span, kind })
            .collect()
    }

    pub(super) fn verify(
        &self,
        raw: raw::Program,
    ) -> Result<crate::data_ownership_v1::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
        verify(
            raw,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            self.linear.clone(),
            self.linux.clone(),
        )
    }
}
