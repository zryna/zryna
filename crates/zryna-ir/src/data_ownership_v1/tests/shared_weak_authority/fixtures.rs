use super::*;
use zryna_layout::{TypeCategory, VerifiedLayouts};
use zryna_source::Span;

#[derive(Clone, Copy, Debug)]
pub(super) enum Payload {
    Bool,
    I32,
    String,
    Struct,
    Enum,
    EmptyArray,
    Array,
    Vec,
    Shared,
    Weak,
    Nested,
    Recursive,
}

impl Payload {
    pub(super) const ALL: [Self; 12] = [
        Self::Bool,
        Self::I32,
        Self::String,
        Self::Struct,
        Self::Enum,
        Self::EmptyArray,
        Self::Array,
        Self::Vec,
        Self::Shared,
        Self::Weak,
        Self::Nested,
        Self::Recursive,
    ];
}

pub(super) struct Fixture {
    pub(super) sources: SourceMap,
    pub(super) linear: VerifiedLayouts,
    pub(super) linux: VerifiedLayouts,
    pub(super) payload: raw::TypeId,
    pub(super) shared: raw::TypeId,
    pub(super) weak: raw::TypeId,
    declarations: u32,
}

impl Fixture {
    pub(super) fn new(payload: Payload) -> Self {
        let (sources, _, _) = authorities();
        let file = sources.verify_file_id(0).expect("entry");
        let span = sources.span(file, 0, 6).expect("nominal span");
        let mut types = vec![];
        let boolean = add(&mut types, raw_layout::TypeKind::Bool, None);
        let integer = add(&mut types, raw_layout::TypeKind::I32, None);
        let string = add(&mut types, raw_layout::TypeKind::String, None);
        let mut declarations = 1;
        let payload =
            payload_node(payload, &mut types, span, &mut declarations, boolean, integer, string);
        let shared = add(&mut types, raw_layout::TypeKind::Shared { payload }, None);
        let weak = add(&mut types, raw_layout::TypeKind::Weak { payload }, None);
        let marker = add(
            &mut types,
            raw_layout::TypeKind::Struct {
                module: raw_layout::ModuleId(0),
                declaration: 0,
                fields: vec![raw_layout::Field { ordinal: 0, ty: payload }],
            },
            Some(span),
        );
        let graph = raw_layout::Graph {
            modules: vec![raw_layout::Module {
                id: raw_layout::ModuleId(0),
                source_file: file,
                data_declarations: declarations,
            }],
            types,
            program_roots: vec![marker, shared, weak],
        };
        let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("independently authenticated linear layout");
        let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("independently authenticated native layout");
        let payload = linear
            .types()
            .find(|ty| ty.nominal_identity() == Some((0, 0)))
            .expect("marker")
            .fields()[0]
            .ty();
        let handle = |category| {
            raw::TypeId(
                linear
                    .types()
                    .find(|ty| ty.category() == category && ty.referenced_type() == Some(payload))
                    .expect("exact handle type")
                    .id()
                    .index(),
            )
        };
        let shared = handle(TypeCategory::Shared);
        let weak = handle(TypeCategory::Weak);
        Self {
            sources,
            linear,
            linux,
            payload: raw::TypeId(payload.index()),
            shared,
            weak,
            declarations,
        }
    }

    pub(super) fn verify(
        &self,
        program: raw::Program,
    ) -> Result<super::super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
        verify(
            program,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            self.linear.clone(),
            self.linux.clone(),
        )
    }

    // Parameters authenticate complete incoming values, not executed allocations or
    // particular Vec lengths/enum variants. Every operation and edge reaches verify.
    pub(super) fn program(&self) -> raw::Program {
        let mut raw = program(&self.sources, &self.linear, &self.linux);
        raw.modules[0].data_declarations = self.declarations;
        let f = &mut raw.modules[0].functions[0];
        let span = f.span;
        f.entry_export = None;
        f.parameters = vec![definition(0, self.payload, span), definition(1, raw::TypeId(1), span)];
        f.places = vec![place(0, self.payload, raw::PlaceKind::Parameter(0), span)];
        for (id, value, ty) in [
            (1, 2, self.shared),
            (2, 3, self.shared),
            (3, 4, self.weak),
            (4, 5, self.weak),
            (5, 6, self.shared),
        ] {
            f.places.push(place(id, ty, raw::PlaceKind::Temporary(raw::ValueId(value)), span));
        }
        f.blocks[0].instructions = vec![
            instruction(
                2,
                self.shared,
                raw::InstructionKind::SharedConstruct {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(0),
                },
                span,
            ),
            instruction(
                3,
                self.shared,
                raw::InstructionKind::SharedClone {
                    place: raw::PlaceId(1),
                    cleanup: raw::CleanupPlanId(1),
                },
                span,
            ),
            instruction(
                4,
                self.weak,
                raw::InstructionKind::WeakDowngrade {
                    place: raw::PlaceId(2),
                    cleanup: raw::CleanupPlanId(2),
                },
                span,
            ),
            instruction(
                5,
                self.weak,
                raw::InstructionKind::WeakClone {
                    place: raw::PlaceId(3),
                    cleanup: raw::CleanupPlanId(3),
                },
                span,
            ),
        ];
        f.blocks[0].terminators[0].kind = raw::Terminator::WeakUpgradeBranch {
            weak: raw::PlaceId(4),
            success: edge(1, vec![raw::ValueId(1)]),
            expired: edge(2, vec![raw::ValueId(1)]),
            cleanup: raw::CleanupPlanId(4),
        };
        f.blocks.push(raw::Block {
            id: raw::BlockId(1),
            parameters: vec![definition(6, self.shared, span), definition(7, raw::TypeId(1), span)],
            instructions: vec![],
            terminators: vec![returning(7, 5, span)],
        });
        f.blocks.push(raw::Block {
            id: raw::BlockId(2),
            parameters: vec![definition(8, raw::TypeId(1), span)],
            instructions: vec![],
            terminators: vec![returning(8, 6, span)],
        });
        let owns_payload = self
            .linear
            .types()
            .find(|ty| ty.id().index() == self.payload.0)
            .expect("payload")
            .drop_kind()
            != 0;
        f.cleanup_plans = vec![
            cleanup(0, if owns_payload { vec![0] } else { vec![] }, span),
            cleanup(1, vec![1], span),
            cleanup(2, vec![2, 1], span),
            cleanup(3, vec![3, 2, 1], span),
            cleanup(4, vec![4, 3, 2, 1], span),
            cleanup(5, vec![5, 4, 3, 2, 1], span),
            cleanup(6, vec![4, 3, 2, 1], span),
        ];
        raw
    }
}

fn add(
    types: &mut Vec<raw_layout::TypeNode>,
    kind: raw_layout::TypeKind,
    span: Option<Span>,
) -> raw_layout::NodeId {
    if let Some(node) = types.iter().find(|node| node.kind == kind) {
        return node.id;
    }
    let id = raw_layout::NodeId(u32::try_from(types.len()).expect("bounded fixture types"));
    types.push(raw_layout::TypeNode { id, span, kind });
    id
}

#[allow(clippy::too_many_arguments)]
fn payload_node(
    payload: Payload,
    types: &mut Vec<raw_layout::TypeNode>,
    span: Span,
    declarations: &mut u32,
    boolean: raw_layout::NodeId,
    integer: raw_layout::NodeId,
    string: raw_layout::NodeId,
) -> raw_layout::NodeId {
    match payload {
        Payload::Bool => boolean,
        Payload::I32 => integer,
        Payload::String => string,
        Payload::Array | Payload::EmptyArray => add(
            types,
            raw_layout::TypeKind::FixedArray {
                element: string,
                length: if matches!(payload, Payload::Array) { 2 } else { 0 },
            },
            None,
        ),
        Payload::Vec => add(types, raw_layout::TypeKind::Vec { element: string }, None),
        Payload::Shared => add(types, raw_layout::TypeKind::Shared { payload: string }, None),
        Payload::Weak => add(types, raw_layout::TypeKind::Weak { payload: string }, None),
        Payload::Enum => {
            *declarations += 1;
            add(
                types,
                raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    variants: vec![
                        raw_layout::Variant { ordinal: 0, payload: None },
                        raw_layout::Variant { ordinal: 1, payload: Some(string) },
                        raw_layout::Variant { ordinal: 2, payload: Some(integer) },
                    ],
                },
                Some(span),
            )
        }
        Payload::Struct | Payload::Nested => {
            *declarations += 1;
            let mut fields = vec![boolean, integer, string];
            if matches!(payload, Payload::Nested) {
                let shared = add(types, raw_layout::TypeKind::Shared { payload: string }, None);
                let weak = add(types, raw_layout::TypeKind::Weak { payload: string }, None);
                let array = add(
                    types,
                    raw_layout::TypeKind::FixedArray { element: shared, length: 2 },
                    None,
                );
                fields.push(add(types, raw_layout::TypeKind::Vec { element: array }, None));
                fields.push(weak);
            }
            add(
                types,
                raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    fields: fields
                        .into_iter()
                        .enumerate()
                        .map(|(ordinal, ty)| raw_layout::Field {
                            ordinal: u32::try_from(ordinal).expect("bounded field"),
                            ty,
                        })
                        .collect(),
                },
                Some(span),
            )
        }
        Payload::Recursive => {
            *declarations += 1;
            let root = raw_layout::NodeId(u32::try_from(types.len()).expect("bounded types"));
            let shared = raw_layout::NodeId(root.0 + 1);
            let node = add(
                types,
                raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    variants: vec![
                        raw_layout::Variant { ordinal: 0, payload: None },
                        raw_layout::Variant { ordinal: 1, payload: Some(shared) },
                    ],
                },
                Some(span),
            );
            assert_eq!(node, root);
            assert_eq!(add(types, raw_layout::TypeKind::Shared { payload: root }, None), shared);
            root
        }
    }
}

pub(super) fn definition(id: u32, ty: raw::TypeId, span: Span) -> raw::ValueDefinition {
    raw::ValueDefinition { id: raw::ValueId(id), ty, span }
}

pub(super) fn place(id: u32, ty: raw::TypeId, kind: raw::PlaceKind, span: Span) -> raw::Place {
    raw::Place { id: raw::PlaceId(id), ty, span, kind }
}

pub(super) fn instruction(
    id: u32,
    ty: raw::TypeId,
    kind: raw::InstructionKind,
    span: Span,
) -> raw::Instruction {
    raw::Instruction { result: Some(definition(id, ty, span)), span, kind }
}

pub(super) fn cleanup(id: u32, places: Vec<u32>, span: Span) -> raw::CleanupPlan {
    raw::CleanupPlan {
        id: raw::CleanupPlanId(id),
        span,
        actions: places
            .into_iter()
            .map(|id| raw::DropAction::DropPlace(raw::PlaceId(id)))
            .collect(),
    }
}

pub(super) fn edge(target: u32, arguments: Vec<raw::ValueId>) -> raw::Edge {
    raw::Edge { target: raw::BlockId(target), arguments }
}

pub(super) fn returning(value: u32, cleanup: u32, span: Span) -> raw::SpannedTerminator {
    raw::SpannedTerminator {
        span,
        kind: raw::Terminator::Return {
            value: raw::ValueId(value),
            cleanup: raw::CleanupPlanId(cleanup),
        },
    }
}
