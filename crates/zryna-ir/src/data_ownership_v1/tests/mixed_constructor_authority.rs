use super::*;
use crate::data_ownership_v1::ValueIdentity;
use zryna_diagnostics::Diagnostic;
use zryna_layout::{TypeCategory, VerifiedLayouts};

// Independent raw IR/layout fixture, not a source-producer or runtime-execution test.
fn mixed_authorities() -> (SourceMap, VerifiedLayouts, VerifiedLayouts) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source spans");
    let file = sources.verify_file_id(0).expect("file");
    let nominal = |declaration, fields| raw_layout::TypeKind::Struct {
        module: raw_layout::ModuleId(0),
        declaration,
        fields,
    };
    let field = |ordinal, ty| raw_layout::Field { ordinal, ty: raw_layout::NodeId(ty) };
    let kinds = vec![
        raw_layout::TypeKind::Bool,
        raw_layout::TypeKind::I32,
        raw_layout::TypeKind::String,
        nominal(0, vec![field(0, 2), field(1, 1)]),
        // Outer refers forward to Vec<Inner>; raw order is not a layout authority.
        nominal(1, vec![field(0, 5), field(1, 2)]),
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(3) },
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
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
                    3 => Some(sources.span(file, 0, 6).expect("Inner declaration span")),
                    4 => Some(sources.span(file, 7, 13).expect("Outer declaration span")),
                    _ => None,
                },
                kind,
            })
            .collect(),
        program_roots: vec![raw_layout::NodeId(4), raw_layout::NodeId(6)],
    };
    let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect("mixed linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("mixed native layouts");
    (sources, linear, linux)
}

#[derive(Clone, Copy)]
struct Types {
    string: raw::TypeId,
    integer: raw::TypeId,
    inner: raw::TypeId,
    outer: raw::TypeId,
    vector: raw::TypeId,
    string_vector: raw::TypeId,
}

fn exact_types(layouts: &VerifiedLayouts) -> Types {
    let category = |kind| layouts.types().find(|ty| ty.category() == kind).expect("leaf").id();
    let nominal = |ordinal| {
        layouts
            .types()
            .find(|ty| ty.nominal_identity() == Some((0, ordinal)))
            .expect("exact nominal")
            .id()
    };
    let string = category(TypeCategory::String);
    let inner = nominal(0);
    let vector = |element| {
        layouts
            .types()
            .find(|ty| ty.category() == TypeCategory::Vec && ty.referenced_type() == Some(element))
            .expect("exact Vec element")
            .id()
    };
    Types {
        string: raw::TypeId(string.index()),
        integer: raw::TypeId(category(TypeCategory::I32).index()),
        inner: raw::TypeId(inner.index()),
        outer: raw::TypeId(nominal(1).index()),
        vector: raw::TypeId(vector(inner).index()),
        string_vector: raw::TypeId(vector(string).index()),
    }
}

fn mixed_program(
    sources: &SourceMap,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
) -> raw::Program {
    let ty = exact_types(linear);
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.entry_export = None;
    function.parameters = vec![value(0, ty.string), value(1, ty.string), value(2, ty.integer)];
    function.result = ty.outer;
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: ty.string, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place { id: raw::PlaceId(1), ty: ty.string, span, kind: raw::PlaceKind::Parameter(1) },
        raw::Place {
            id: raw::PlaceId(2),
            ty: ty.inner,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: ty.vector,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: ty.outer,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(value(3, ty.inner)),
            span,
            kind: raw::InstructionKind::StructConstruct {
                fields: vec![raw::ValueId(0), raw::ValueId(2)],
                cleanup: None,
            },
        },
        raw::Instruction {
            result: Some(value(4, ty.vector)),
            span,
            kind: raw::InstructionKind::VecConstruct {
                elements: vec![raw::ValueId(3)],
                cleanup: raw::CleanupPlanId(0),
            },
        },
        raw::Instruction {
            result: Some(value(5, ty.outer)),
            span,
            kind: raw::InstructionKind::StructConstruct {
                fields: vec![raw::ValueId(4), raw::ValueId(1)],
                cleanup: None,
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(2)),
                raw::DropAction::DropPlace(raw::PlaceId(1)),
            ],
        },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions: vec![] },
    ];
    raw
}

#[test]
fn mixed_raw_constructor_seed_verifies_exact_types_transfers_and_replay() {
    let (sources, linear, linux) = mixed_authorities();
    let raw = mixed_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let mut previous = None;
    for _ in 0..2 {
        let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect("independent mixed raw seed");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StructConstruct,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::StructConstruct
            ]
        );
        let values =
            instructions.iter().map(|i| i.result().expect("result").index()).collect::<Vec<_>>();
        assert_eq!(values, vec![3, 4, 5]);
        let operands = instructions
            .iter()
            .map(|i| i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(operands, vec![vec![0, 2], vec![3], vec![4, 1]]);
        let ty = exact_types(&linear);
        assert_eq!(
            instructions.iter().map(|i| i.result_type().expect("type").index()).collect::<Vec<_>>(),
            vec![ty.inner.0, ty.vector.0, ty.outer.0]
        );
        assert_eq!(function.places().count(), 5);
        assert_eq!(
            instructions[1].derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(
            block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            vec![5]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        if let Some(prior) = previous.replace((values.clone(), operands.clone())) {
            assert_eq!((values, operands), prior);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Damage {
    Layout,
    ElementType,
    HeterogeneousOrder,
    DuplicateOwner,
    MissingCleanup,
    ReusedSite,
}

fn mutate(raw: &mut raw::Program, ty: Types, damage: Damage) -> &'static str {
    if matches!(damage, Damage::Layout) {
        raw.authorities.linear32_fingerprint[0] ^= 1;
        return "ZRYNA-I3003";
    }
    let function = &mut raw.modules[0].functions[0];
    match damage {
        Damage::ElementType => {
            function.blocks[0].instructions[1].result.as_mut().expect("Vec result").ty =
                ty.string_vector;
            function.places[3].ty = ty.string_vector;
            "ZRYNA-I3005"
        }
        Damage::HeterogeneousOrder => {
            let raw::InstructionKind::StructConstruct { fields, .. } =
                &mut function.blocks[0].instructions[2].kind
            else {
                panic!("Outer")
            };
            fields.swap(0, 1);
            "ZRYNA-I3005"
        }
        Damage::DuplicateOwner => {
            let raw::InstructionKind::VecConstruct { elements, .. } =
                &mut function.blocks[0].instructions[1].kind
            else {
                panic!("Vec")
            };
            elements.push(raw::ValueId(3));
            "ZRYNA-I3010"
        }
        Damage::MissingCleanup => {
            function.cleanup_plans[0].actions.remove(0);
            "ZRYNA-I3012"
        }
        Damage::ReusedSite => {
            let raw::Terminator::Return { cleanup, .. } =
                &mut function.blocks[0].terminators[0].kind
            else {
                panic!("return")
            };
            *cleanup = raw::CleanupPlanId(0);
            "ZRYNA-I3012"
        }
        Damage::Layout => unreachable!(),
    }
}

#[test]
fn mixed_raw_constructor_mutations_fail_at_exact_authority_phase() {
    let (sources, linear, linux) = mixed_authorities();
    let seed = mixed_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    verify(seed.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("valid control before mutations");
    for damage in [
        Damage::Layout,
        Damage::ElementType,
        Damage::HeterogeneousOrder,
        Damage::DuplicateOwner,
        Damage::MissingCleanup,
        Damage::ReusedSite,
    ] {
        let mut raw = seed.clone();
        let expected = mutate(&mut raw, exact_types(&linear), damage);
        let first = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect_err("isolated forgery");
        let second = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect_err("deterministic forgery");
        assert_eq!(first, second, "complete ordered diagnostic replay: {damage:?}");
        assert_eq!(
            first.first().expect("nonempty diagnostics").code(),
            expected,
            "first authority phase: {damage:?}"
        );
        assert_eq!(first[0], first_diagnostic(damage, seed.modules[0].functions[0].span));
    }
}

fn first_diagnostic(damage: Damage, span: zryna_source::Span) -> Diagnostic {
    if matches!(damage, Damage::Layout) {
        return Diagnostic::error(
            "ZRYNA-I3003",
            None,
            "DataOwnershipV1 layout or runtime authority does not match its sealed claim",
            "use both exact layouts issued from the final source-bound type universe",
        );
    }
    let (code, message, guidance) = match damage {
        Damage::ElementType | Damage::HeterogeneousOrder => (
            "ZRYNA-I3005",
            "instruction operands or result have an invalid sealed type",
            "use the exact operand and result types required by the operation",
        ),
        Damage::DuplicateOwner => (
            "ZRYNA-I3010",
            "partial non-Copy owner cannot enter a context without mask transfer",
            "emit only legal ownership state transitions",
        ),
        Damage::MissingCleanup => (
            "ZRYNA-I3012",
            "cleanup plan is incomplete, duplicated, or out of reverse-completion order",
            "drop every live non-Copy root exactly once in reverse completion order",
        ),
        Damage::ReusedSite => (
            "ZRYNA-I3012",
            "cleanup plan is not bound to exactly one operation or exit site",
            "create one dense cleanup plan for each exact prepare, call, return, or trap site",
        ),
        Damage::Layout => unreachable!(),
    };
    Diagnostic::error_at(code, span, message, guidance)
}
