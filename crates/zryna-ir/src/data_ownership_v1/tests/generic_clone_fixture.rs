use super::*;
use zryna_layout::{TypeCategory, VerifiedLayouts};

pub(super) struct Fixture {
    pub(super) sources: SourceMap,
    pub(super) linear: VerifiedLayouts,
    pub(super) linux: VerifiedLayouts,
    pub(super) root: raw::TypeId,
    pub(super) string: raw::TypeId,
    pub(super) integer: raw::TypeId,
}

#[derive(Clone, Copy)]
pub(super) enum GraphKind {
    Ordinary,
    Recursive,
    Shared,
    Weak,
    RecursiveShared,
    ZeroStrideVec,
}

pub(super) fn graph(sources: &SourceMap, mode: GraphKind) -> raw_layout::Graph {
    let file = sources.verify_file_id(0).expect("file");
    let field = |ordinal, ty| raw_layout::Field { ordinal, ty: raw_layout::NodeId(ty) };
    let mut kinds = vec![
        raw_layout::TypeKind::Bool,
        raw_layout::TypeKind::I32,
        raw_layout::TypeKind::String,
        raw_layout::TypeKind::Struct {
            module: raw_layout::ModuleId(0),
            declaration: 0,
            fields: vec![field(0, 2), field(1, 1)],
        },
        raw_layout::TypeKind::Enum {
            module: raw_layout::ModuleId(0),
            declaration: 1,
            variants: vec![
                raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(2)) },
                raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(3)) },
            ],
        },
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(4) },
        raw_layout::TypeKind::FixedArray { element: raw_layout::NodeId(5), length: 2 },
        raw_layout::TypeKind::Struct {
            module: raw_layout::ModuleId(0),
            declaration: 2,
            fields: vec![field(0, 6), field(1, 4)],
        },
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(7) },
    ];
    if matches!(mode, GraphKind::Recursive | GraphKind::RecursiveShared) {
        let raw_layout::TypeKind::Struct { fields, .. } = &mut kinds[3] else {
            panic!("inner");
        };
        fields.push(field(2, 8));
    }
    if matches!(mode, GraphKind::Shared | GraphKind::Weak | GraphKind::RecursiveShared) {
        kinds.push(if matches!(mode, GraphKind::Weak) {
            raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(2) }
        } else {
            raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(2) }
        });
        let raw_layout::TypeKind::Struct { fields, .. } = &mut kinds[3] else {
            panic!("inner");
        };
        fields[0].ty = raw_layout::NodeId(9);
    }
    if matches!(mode, GraphKind::ZeroStrideVec) {
        kinds.push(raw_layout::TypeKind::FixedArray { element: raw_layout::NodeId(1), length: 0 });
        kinds[5] = raw_layout::TypeKind::Vec { element: raw_layout::NodeId(9) };
    }
    raw_layout::Graph {
        modules: vec![raw_layout::Module {
            id: raw_layout::ModuleId(0),
            source_file: file,
            data_declarations: 3,
        }],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(id, kind)| raw_layout::TypeNode {
                id: raw_layout::NodeId(u32::try_from(id).expect("small graph")),
                span: match id {
                    3 => Some(sources.span(file, 0, 6).expect("inner span")),
                    4 => Some(sources.span(file, 7, 13).expect("enum span")),
                    7 => Some(sources.span(file, 14, 20).expect("outer span")),
                    _ => None,
                },
                kind,
            })
            .collect(),
        program_roots: vec![raw_layout::NodeId(8)],
    }
}

impl Fixture {
    pub(super) fn new(category: TypeCategory) -> Self {
        Self::with_graph(category, GraphKind::Ordinary)
    }

    pub(super) fn with_graph(category: TypeCategory, mode: GraphKind) -> Self {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: "export function id(value: i32): i32 { return value; }".into(),
        }])
        .expect("source");
        let graph = graph(&sources, mode);
        let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("linear layout");
        let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("native layout");
        let outer =
            linear.types().find(|ty| ty.nominal_identity() == Some((0, 2))).expect("outer").id();
        let root = linear
            .types()
            .find(|ty| {
                ty.category() == category
                    && match category {
                        TypeCategory::Struct => ty.id() == outer,
                        TypeCategory::Vec => ty.referenced_type() == Some(outer),
                        _ => true,
                    }
            })
            .expect("root")
            .id();
        let scalar = |category| {
            raw::TypeId(
                linear.types().find(|ty| ty.category() == category).expect("scalar").id().index(),
            )
        };
        let string = scalar(TypeCategory::String);
        let integer = scalar(TypeCategory::I32);
        Self { root: raw::TypeId(root.index()), string, integer, sources, linear, linux }
    }

    pub(super) fn seed(&self) -> raw::Program {
        let mut raw = program(&self.sources, &self.linear, &self.linux);
        raw.modules[0].data_declarations = 3;
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
        function.entry_export = None;
        function.parameters =
            vec![value(0, self.root), value(1, self.string), value(2, self.integer)];
        function.result = self.root;
        function.places = vec![
            raw::Place {
                id: raw::PlaceId(0),
                ty: self.root,
                span,
                kind: raw::PlaceKind::Parameter(0),
            },
            raw::Place {
                id: raw::PlaceId(1),
                ty: self.string,
                span,
                kind: raw::PlaceKind::Parameter(1),
            },
            raw::Place {
                id: raw::PlaceId(2),
                ty: self.root,
                span,
                kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
            },
        ];
        function.blocks[0].instructions = vec![raw::Instruction {
            result: Some(value(3, self.root)),
            span,
            kind: raw::InstructionKind::GenericClonePlace {
                place: raw::PlaceId(0),
                cleanup: raw::CleanupPlanId(0),
                prefix_cleanup: raw::CleanupPlanId(1),
            },
        }];
        function.blocks[0].terminators[0].kind =
            raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(2) };
        let pending = vec![
            raw::DropAction::DropPlace(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ];
        let mut prefix = vec![raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(2))];
        prefix.extend(pending.clone());
        function.cleanup_plans = [pending.clone(), prefix, pending]
            .into_iter()
            .enumerate()
            .map(|(id, actions)| raw::CleanupPlan {
                id: raw::CleanupPlanId(u32::try_from(id).expect("three plans")),
                span,
                actions,
            })
            .collect();
        raw
    }

    pub(super) fn verify(&self, raw: raw::Program) -> crate::data_ownership_v1::VerifiedProgram {
        verify(
            raw,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            self.linear.clone(),
            self.linux.clone(),
        )
        .expect("independent generic clone baseline")
    }

    pub(super) fn rejects(&self, raw: raw::Program, code: &str) {
        self.rejects_case(raw, code, "generic clone hostile input");
    }

    pub(super) fn rejects_case(&self, raw: raw::Program, code: &str, case: &str) {
        let check = |raw| {
            verify(
                raw,
                &self.sources,
                self.sources.verify_file_id(0).expect("entry"),
                self.linear.clone(),
                self.linux.clone(),
            )
            .expect_err("hostile generic clone")
        };
        let first = check(raw.clone());
        assert!(
            first.iter().any(|error| error.code() == code),
            "{case}: expected {code}: {first:?}"
        );
        assert_eq!(first, check(raw));
    }
}
