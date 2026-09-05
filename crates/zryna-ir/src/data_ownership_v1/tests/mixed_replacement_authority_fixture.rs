use super::*;
use zryna_layout::{TypeCategory, VerifiedLayouts};

// Independent raw IR/layout evidence; no source producer or target execution is involved.
pub(super) fn authorities() -> (SourceMap, VerifiedLayouts, VerifiedLayouts) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source spans");
    let file = sources.verify_file_id(0).expect("file");
    let kinds = vec![
        raw_layout::TypeKind::Bool,
        raw_layout::TypeKind::I32,
        raw_layout::TypeKind::String,
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
        raw_layout::TypeKind::Enum {
            module: raw_layout::ModuleId(0),
            declaration: 0,
            variants: vec![
                raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(3)) },
                raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(3)) },
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
                span: (id == 4).then(|| sources.span(file, 0, 6).expect("Enum span")),
                kind,
            })
            .collect(),
        program_roots: vec![raw_layout::NodeId(4)],
    };
    let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect("linear authority");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native authority");
    (sources, linear, linux)
}

pub(super) fn seed(
    sources: &SourceMap,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
) -> raw::Program {
    let category = |kind| {
        raw::TypeId(
            linear.types().find(|ty| ty.category() == kind).expect("unique category").id().index(),
        )
    };
    let string = category(TypeCategory::String);
    let integer = category(TypeCategory::I32);
    let vector = category(TypeCategory::Vec);
    let choice = category(TypeCategory::Enum);
    let mut raw = program(sources, linear, linux);
    raw.modules[0].data_declarations = 1;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    let place = |id, ty, kind| raw::Place { id: raw::PlaceId(id), ty, span, kind };
    let temporary = |id| raw::PlaceKind::Temporary(raw::ValueId(id));
    function.entry_export = None;
    function.parameters = vec![value(0, string), value(1, string), value(2, integer)];
    function.result = integer;
    function.places = vec![
        place(0, string, raw::PlaceKind::Parameter(0)),
        place(1, string, raw::PlaceKind::Parameter(1)),
        place(2, vector, temporary(3)),
        place(3, choice, temporary(4)),
        place(4, choice, raw::PlaceKind::Local(0)),
        place(5, string, temporary(5)),
        place(6, vector, temporary(6)),
        place(7, choice, temporary(7)),
    ];
    let instruction = |result, kind| raw::Instruction { result, span, kind };
    let vector_value = |id, element, cleanup| {
        instruction(
            Some(value(id, vector)),
            raw::InstructionKind::VecConstruct {
                elements: vec![raw::ValueId(element)],
                cleanup: raw::CleanupPlanId(cleanup),
            },
        )
    };
    let enum_value = |id, variant, payload| {
        instruction(
            Some(value(id, choice)),
            raw::InstructionKind::EnumConstruct {
                variant,
                payload: Some(raw::ValueId(payload)),
                cleanup: None,
            },
        )
    };
    function.blocks[0].instructions = vec![
        vector_value(3, 0, 0),
        enum_value(4, 1, 3),
        instruction(
            None,
            raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(4),
                value: raw::ValueId(4),
            },
        ),
        instruction(
            Some(value(5, string)),
            raw::InstructionKind::StringFromUtf8 {
                bytes: b"survivor".to_vec(),
                cleanup: raw::CleanupPlanId(1),
            },
        ),
        vector_value(6, 1, 2),
        enum_value(7, 0, 6),
        instruction(
            None,
            raw::InstructionKind::ReplacePlace { place: raw::PlaceId(4), value: raw::ValueId(7) },
        ),
    ];
    let cleanup = |id, roots: &[u32]| raw::CleanupPlan {
        id: raw::CleanupPlanId(id),
        span,
        actions: roots.iter().map(|id| raw::DropAction::DropPlace(raw::PlaceId(*id))).collect(),
    };
    function.cleanup_plans =
        vec![cleanup(0, &[1, 0]), cleanup(1, &[4, 1]), cleanup(2, &[5, 4, 1]), cleanup(3, &[4, 5])];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(3) };
    raw
}
