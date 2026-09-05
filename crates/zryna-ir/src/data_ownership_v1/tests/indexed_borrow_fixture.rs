use super::*;
use zryna_layout::{TypeCategory, VerifiedLayouts};

#[derive(Clone, Copy, Debug)]
pub(super) enum Container {
    Array,
    Vec,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Element {
    Bool,
    I32,
    String,
    Struct,
    Enum,
    Shared,
    Weak,
    Vec,
    Array,
}

pub(super) const ELEMENTS: [Element; 9] = [
    Element::Bool,
    Element::I32,
    Element::String,
    Element::Struct,
    Element::Enum,
    Element::Shared,
    Element::Weak,
    Element::Vec,
    Element::Array,
];

pub(super) struct Fixture {
    pub(super) sources: SourceMap,
    pub(super) linear: VerifiedLayouts,
    pub(super) linux: VerifiedLayouts,
    pub(super) root: raw::TypeId,
    pub(super) element: raw::TypeId,
    pub(super) integer: raw::TypeId,
    pub(super) boolean: raw::TypeId,
    pub(super) wrapper: raw::TypeId,
}

impl Fixture {
    pub(super) fn new(container: Container, element: Element) -> Self {
        Self::with_lengths(container, element, 2, 2)
    }

    fn type_kinds(
        container: Container,
        element: Element,
        length: u64,
        nested_length: u64,
    ) -> (Vec<raw_layout::TypeKind>, u32) {
        let mut kinds = vec![
            raw_layout::TypeKind::Bool,
            raw_layout::TypeKind::I32,
            raw_layout::TypeKind::String,
            raw_layout::TypeKind::Struct {
                module: raw_layout::ModuleId(0),
                declaration: 0,
                fields: vec![
                    raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(2) },
                    raw_layout::Field { ordinal: 1, ty: raw_layout::NodeId(1) },
                ],
            },
            raw_layout::TypeKind::Enum {
                module: raw_layout::ModuleId(0),
                declaration: 1,
                variants: vec![
                    raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(2)) },
                    raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(3)) },
                ],
            },
            raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(2) },
            raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(2) },
            raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
            raw_layout::TypeKind::FixedArray {
                element: raw_layout::NodeId(2),
                length: nested_length,
            },
        ];
        let node = match element {
            Element::Bool => 0,
            Element::I32 => 1,
            Element::String => 2,
            Element::Struct => 3,
            Element::Enum => 4,
            Element::Shared => 5,
            Element::Weak => 6,
            Element::Vec => 7,
            Element::Array => 8,
        };
        let root = match (container, element) {
            (Container::Vec, Element::String) => 7,
            (Container::Array, Element::String) if length == nested_length => 8,
            _ => {
                kinds.push(match container {
                    Container::Array => raw_layout::TypeKind::FixedArray {
                        element: raw_layout::NodeId(node),
                        length,
                    },
                    Container::Vec => {
                        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(node) }
                    }
                });
                9
            }
        };
        let wrapper_node = u32::try_from(kinds.len()).expect("small graph");
        kinds.push(raw_layout::TypeKind::Enum {
            module: raw_layout::ModuleId(0),
            declaration: 2,
            variants: vec![
                raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(root)) },
                raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(root)) },
            ],
        });
        (kinds, wrapper_node)
    }

    pub(super) fn with_lengths(
        container: Container,
        element: Element,
        length: u64,
        nested_length: u64,
    ) -> Self {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: "export function id(value: i32): i32 { return value; }".into(),
        }])
        .expect("source authority");
        let file = sources.verify_file_id(0).expect("file");
        let (kinds, wrapper_node) = Self::type_kinds(container, element, length, nested_length);
        let graph = raw_layout::Graph {
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
                        3 => Some(sources.span(file, 0, 6).expect("Struct span")),
                        4 => Some(sources.span(file, 7, 13).expect("Enum span")),
                        _ if id == wrapper_node as usize => {
                            Some(sources.span(file, 14, 20).expect("wrapper span"))
                        }
                        _ => None,
                    },
                    kind,
                })
                .collect(),
            program_roots: (2..=wrapper_node).map(raw_layout::NodeId).collect(),
        };
        let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("linear layout");
        let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("native layout");
        let category = |category| {
            linear.types().find(|ty| ty.category() == category).expect("unique category").id()
        };
        let element_id = match element {
            Element::Bool => category(TypeCategory::Bool),
            Element::I32 => category(TypeCategory::I32),
            Element::String => category(TypeCategory::String),
            Element::Struct => category(TypeCategory::Struct),
            Element::Enum => linear
                .types()
                .find(|ty| ty.nominal_identity() == Some((0, 1)))
                .expect("element Enum")
                .id(),
            Element::Shared => category(TypeCategory::Shared),
            Element::Weak => category(TypeCategory::Weak),
            Element::Vec | Element::Array => linear
                .types()
                .find(|ty| {
                    ty.category()
                        == if matches!(element, Element::Vec) {
                            TypeCategory::Vec
                        } else {
                            TypeCategory::FixedArray
                        }
                        && ty.referenced_type() == Some(category(TypeCategory::String))
                })
                .expect("nested String container")
                .id(),
        };
        let root = linear
            .types()
            .find(|ty| {
                ty.category()
                    == if matches!(container, Container::Vec) {
                        TypeCategory::Vec
                    } else {
                        TypeCategory::FixedArray
                    }
                    && ty.referenced_type() == Some(element_id)
            })
            .expect("exact container element")
            .id();
        let integer = raw::TypeId(category(TypeCategory::I32).index());
        let boolean = raw::TypeId(category(TypeCategory::Bool).index());
        let wrapper = raw::TypeId(
            linear
                .types()
                .find(|ty| ty.nominal_identity() == Some((0, 2)))
                .expect("wrapper Enum")
                .id()
                .index(),
        );
        Self {
            root: raw::TypeId(root.index()),
            element: raw::TypeId(element_id.index()),
            integer,
            boolean,
            wrapper,
            sources,
            linear,
            linux,
        }
    }

    pub(super) fn is_copy(&self, ty: raw::TypeId) -> bool {
        self.linear
            .types()
            .find(|record| record.id().index() == ty.0)
            .expect("sealed type")
            .drop_kind()
            == 0
    }

    pub(super) fn seed(&self, access: raw::BorrowAccess) -> raw::Program {
        let mut raw = program(&self.sources, &self.linear, &self.linux);
        raw.modules[0].data_declarations = 3;
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
        function.entry_export = None;
        function.parameters = vec![
            value(0, self.root),
            value(1, self.root),
            value(2, self.integer),
            value(3, self.integer),
            value(4, self.element),
        ];
        function.result = self.integer;
        function.places = vec![
            raw::Place {
                id: raw::PlaceId(0),
                ty: self.root,
                span,
                kind: raw::PlaceKind::Parameter(0),
            },
            raw::Place {
                id: raw::PlaceId(1),
                ty: self.root,
                span,
                kind: raw::PlaceKind::Parameter(1),
            },
            raw::Place {
                id: raw::PlaceId(2),
                ty: self.element,
                span,
                kind: raw::PlaceKind::Parameter(4),
            },
        ];
        function.blocks[0].instructions =
            vec![indexed(0, 0, 2, 0, access, span), end_borrow(0, span)];
        let actions = function
            .places
            .iter()
            .rev()
            .filter(|place| !self.is_copy(place.ty))
            .map(|place| raw::DropAction::DropPlace(place.id))
            .collect::<Vec<_>>();
        function.cleanup_plans = (0..2)
            .map(|id| raw::CleanupPlan {
                id: raw::CleanupPlanId(id),
                span,
                actions: actions.clone(),
            })
            .collect();
        function.blocks[0].terminators[0].kind =
            raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
        raw
    }

    pub(super) fn verify(&self, raw: raw::Program) -> super::super::VerifiedProgram {
        verify(
            raw,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            self.linear.clone(),
            self.linux.clone(),
        )
        .expect("independent valid indexed authority")
    }

    pub(super) fn rejects(&self, raw: raw::Program, code: &str) {
        let entry = self.sources.verify_file_id(0).expect("entry");
        let Err(first) =
            verify(raw.clone(), &self.sources, entry, self.linear.clone(), self.linux.clone())
        else {
            panic!("isolated hostile authority unexpectedly verified");
        };
        let Err(second) =
            verify(raw, &self.sources, entry, self.linear.clone(), self.linux.clone())
        else {
            panic!("repeated hostile authority unexpectedly verified");
        };
        assert_eq!(first, second, "complete diagnostic replay");
        assert!(first.iter().any(|diagnostic| diagnostic.code() == code), "{first:?}");
    }
}

pub(super) fn indexed(
    id: u32,
    place: u32,
    index: u32,
    cleanup: u32,
    access: raw::BorrowAccess,
    span: zryna_source::Span,
) -> raw::Instruction {
    raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::BeginIndexedBorrow {
            definition: raw::BorrowDefinition {
                id: raw::BorrowId(id),
                place: raw::PlaceId(place),
                access,
                span,
            },
            index: raw::ValueId(index),
            cleanup: raw::CleanupPlanId(cleanup),
        },
    }
}
