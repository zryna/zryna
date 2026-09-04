use super::*;
use crate::data_ownership_v1::ValueIdentity;
use zryna_diagnostics::Diagnostic;
use zryna_layout::{TypeCategory, VerifiedLayouts};

// Raw IR authority only: spans are real, but this graph is not a parsed source program.
fn authorities() -> (SourceMap, VerifiedLayouts, VerifiedLayouts) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source spans");
    let file = sources.verify_file_id(0).expect("file");
    let kinds = vec![
        raw_layout::TypeKind::String,
        raw_layout::TypeKind::I32,
        raw_layout::TypeKind::Enum {
            module: raw_layout::ModuleId(0),
            declaration: 0,
            variants: vec![
                raw_layout::Variant { ordinal: 0, payload: None },
                raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(0)) },
            ],
        },
        raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
        raw_layout::TypeKind::Struct {
            module: raw_layout::ModuleId(0),
            declaration: 1,
            fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(3) }],
        },
        raw_layout::TypeKind::Bool,
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
                    2 => Some(sources.span(file, 0, 6).expect("Enum declaration")),
                    4 => Some(sources.span(file, 7, 13).expect("Outer declaration")),
                    _ => None,
                },
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

fn seed(sources: &SourceMap, linear: &VerifiedLayouts, linux: &VerifiedLayouts) -> raw::Program {
    let category = |kind| {
        raw::TypeId(
            linear.types().find(|t| t.category() == kind).expect("unique category").id().index(),
        )
    };
    let (string, integer, choice, vector, outer) = (
        category(TypeCategory::String),
        category(TypeCategory::I32),
        category(TypeCategory::Enum),
        category(TypeCategory::Vec),
        category(TypeCategory::Struct),
    );
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.entry_export = None;
    function.parameters = vec![value(0, string), value(1, integer), value(2, string)];
    function.result = outer;
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: string, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place { id: raw::PlaceId(1), ty: string, span, kind: raw::PlaceKind::Parameter(2) },
        raw::Place {
            id: raw::PlaceId(2),
            ty: choice,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: vector,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: outer,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(value(3, choice)),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 1,
                payload: Some(raw::ValueId(0)),
                cleanup: None,
            },
        },
        raw::Instruction {
            result: Some(value(4, vector)),
            span,
            kind: raw::InstructionKind::VecConstruct {
                elements: vec![raw::ValueId(3)],
                cleanup: raw::CleanupPlanId(0),
            },
        },
        raw::Instruction {
            result: Some(value(5, outer)),
            span,
            kind: raw::InstructionKind::StructConstruct {
                fields: vec![raw::ValueId(4)],
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
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
    ];
    program
}

#[test]
fn mixed_selected_enum_vec_outer_raw_control_seals_payload_and_cleanup_roles() {
    let (sources, linear, linux) = authorities();
    let raw = seed(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let mut previous = None;
    for _ in 0..2 {
        let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect("selected owned Enum inside Vec inside Outer");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::EnumConstruct,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::StructConstruct,
            ]
        );
        assert_eq!(instructions[0].variant(), Some(1));
        assert_eq!(function.places().count(), 5);
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.result().expect("result").index(),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            vec![(3, vec![0], vec![]), (4, vec![3], vec![2, 1]), (5, vec![4], vec![])]
        );
        let drop = instructions[1].derived_drop_actions().next().expect("selected Enum cleanup");
        assert_eq!(drop.active_variant(), Some(1));
        assert_eq!(
            block.terminator().derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            function.cleanup_plans().map(|p| p.site().role()).collect::<Vec<_>>(),
            vec![VerifiedCleanupRole::PrepareFailure, VerifiedCleanupRole::Return]
        );
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}

#[derive(Clone, Copy, Debug)]
enum Damage {
    PayloadType,
    PayloadOnEmpty,
    SwappedRoles,
    ReusedSite,
    MissingAction,
    DuplicateAction,
}

fn mutate(program: &mut raw::Program, damage: Damage) {
    let function = &mut program.modules[0].functions[0];
    match damage {
        Damage::PayloadType | Damage::PayloadOnEmpty => {
            let raw::InstructionKind::EnumConstruct { variant, payload, .. } =
                &mut function.blocks[0].instructions[0].kind
            else {
                panic!("Enum")
            };
            if matches!(damage, Damage::PayloadType) {
                *payload = Some(raw::ValueId(1));
            } else {
                *variant = 0;
            }
        }
        Damage::SwappedRoles | Damage::ReusedSite => {
            let raw::Terminator::Return { cleanup, .. } =
                &mut function.blocks[0].terminators[0].kind
            else {
                panic!("return")
            };
            *cleanup = raw::CleanupPlanId(0);
            if matches!(damage, Damage::SwappedRoles) {
                let raw::InstructionKind::VecConstruct { cleanup, .. } =
                    &mut function.blocks[0].instructions[1].kind
                else {
                    panic!("Vec")
                };
                *cleanup = raw::CleanupPlanId(1);
            }
        }
        Damage::MissingAction => {
            function.cleanup_plans[0].actions.remove(0);
        }
        Damage::DuplicateAction => {
            function.cleanup_plans[0].actions.push(raw::DropAction::DropPlace(raw::PlaceId(2)));
        }
    }
}

fn diagnostic(damage: Damage, at: zryna_source::Span) -> Diagnostic {
    let (code, message, help) = match damage {
        Damage::PayloadType | Damage::PayloadOnEmpty => (
            "ZRYNA-I3005",
            "instruction operands or result have an invalid sealed type",
            "use the exact operand and result types required by the operation",
        ),
        Damage::ReusedSite => (
            "ZRYNA-I3012",
            "cleanup plan is not bound to exactly one operation or exit site",
            "create one dense cleanup plan for each exact prepare, call, return, or trap site",
        ),
        Damage::DuplicateAction => (
            "ZRYNA-I3012",
            "cleanup plan has a noncanonical identity or foreign place",
            "use dense plans containing each local verified place at most once",
        ),
        Damage::SwappedRoles | Damage::MissingAction => (
            "ZRYNA-I3012",
            "cleanup plan is incomplete, duplicated, or out of reverse-completion order",
            "drop every live non-Copy root exactly once in reverse completion order",
        ),
    };
    Diagnostic::error_at(code, at, message, help)
}

#[test]
fn mixed_selected_enum_mutations_reject_exact_payload_and_cleanup_authority() {
    let (sources, linear, linux) = authorities();
    let seed = seed(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    verify(seed.clone(), &sources, entry, linear.clone(), linux.clone()).expect("valid control");
    for damage in [
        Damage::PayloadType,
        Damage::PayloadOnEmpty,
        Damage::SwappedRoles,
        Damage::ReusedSite,
        Damage::MissingAction,
        Damage::DuplicateAction,
    ] {
        let mut raw = seed.clone();
        mutate(&mut raw, damage);
        let first = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect_err("isolated malformed authority");
        let second = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect_err("deterministic malformed authority");
        assert_eq!(first, second, "full diagnostic replay: {damage:?}");
        assert_eq!(
            first.first(),
            Some(&diagnostic(damage, seed.modules[0].functions[0].span)),
            "first exact authority diagnostic: {damage:?}"
        );
    }
}
