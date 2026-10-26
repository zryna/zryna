use super::super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use zryna_source::UntrustedSpan;
#[derive(Clone, Copy)]
pub(super) enum Case {
    String,
    Clone,
    Loan,
}
fn span() -> UntrustedSpan {
    UntrustedSpan { file: 0, start: 20, end: 30 }
}
fn value(id: u32, ty: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }
}
pub(super) fn inputs(p: &raw::Program, world: &TypeWorld) -> typed::Typed {
    typed::container_test_support::phase_inputs(p, world)
}
pub(super) fn program(case: Case) -> (raw::Program, TypeWorld) {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("source String fact");
    let world = builder.seal(&mut 0).expect("private checked capability graph");
    let parameter = !matches!(case, Case::String);
    let result = u32::from(parameter);
    let parameters = if parameter { vec![raw::Parameter::Value(value(0, string))] } else { vec![] };
    let mut places = if parameter {
        vec![raw::Place { id: 0, ty: string, span: span(), kind: raw::PlaceKind::Parameter(0) }]
    } else {
        vec![]
    };
    let mut loans = vec![];
    let (operation, cleanup_plans, cleanup) = match case {
        Case::String => string_operation(string, &mut places),
        Case::Clone => clone_operation(string, &mut places),
        Case::Loan => loan_operation(string, &mut loans),
    };
    let mut instructions = vec![raw::Instruction {
        site_span: span(),
        result: if matches!(case, Case::Loan) { None } else { Some(value(result, string)) },
        operation,
    }];
    if matches!(case, Case::Loan) {
        instructions.push(raw::Instruction {
            site_span: span(),
            result: None,
            operation: raw::Operation::EndLoan { loan: 0 },
        });
    }
    let f = raw::Function {
        key: vec![],
        original_span: UntrustedSpan { file: 0, start: 0, end: 100 },
        executable_export: None,
        parameters,
        result_type: raw::ValueType::Unit,
        places,
        loans,
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions,
            terminator: raw::Terminator::Return { value: None, cleanup },
        }],
        cleanup_plans,
    };
    (
        raw::Program {
            language_contract: raw::CONTRACT.into(),
            modules: vec![],
            declarations: vec![],
            type_keys: world.keys().map(<[u8]>::to_vec).collect(),
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            entry_module: 0,
            functions: vec![f],
        },
        world,
    )
}

fn string_operation(
    string: u32,
    places: &mut Vec<raw::Place>,
) -> (raw::Operation, Vec<raw::CleanupPlan>, u32) {
    places.push(raw::Place { id: 0, ty: string, span: span(), kind: raw::PlaceKind::Temporary(0) });
    (
        raw::Operation::StringFromUtf8 { bytes: b"L".to_vec(), failure: 0 },
        vec![
            raw::CleanupPlan { id: 0, span: span(), actions: vec![] },
            raw::CleanupPlan {
                id: 1,
                span: span(),
                actions: vec![raw::CleanupAction::DropPlace(0)],
            },
        ],
        1,
    )
}

fn clone_operation(
    string: u32,
    places: &mut Vec<raw::Place>,
) -> (raw::Operation, Vec<raw::CleanupPlan>, u32) {
    places.push(raw::Place { id: 1, ty: string, span: span(), kind: raw::PlaceKind::Temporary(1) });
    (
        raw::Operation::Clone {
            source: raw::PlaceOrLoan::Place(0),
            outer_failure: Some(0),
            prefix_failure: Some(1),
        },
        vec![
            raw::CleanupPlan {
                id: 0,
                span: span(),
                actions: vec![raw::CleanupAction::DropPlace(0)],
            },
            raw::CleanupPlan {
                id: 1,
                span: span(),
                actions: vec![
                    raw::CleanupAction::DropInitializedPrefix {
                        destination: 1,
                        preparation_site: raw::PreparationSite {
                            function: 0,
                            block: 0,
                            instruction: 0,
                        },
                    },
                    raw::CleanupAction::DropPlace(0),
                ],
            },
            raw::CleanupPlan {
                id: 2,
                span: span(),
                actions: vec![raw::CleanupAction::DropPlace(1), raw::CleanupAction::DropPlace(0)],
            },
        ],
        2,
    )
}

fn loan_operation(
    string: u32,
    loans: &mut Vec<raw::Loan>,
) -> (raw::Operation, Vec<raw::CleanupPlan>, u32) {
    loans.push(raw::Loan {
        id: 0,
        referent: string,
        access: raw::Access::Shared,
        region: raw::Region { span: span(), role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Place(0),
    });
    (
        raw::Operation::BeginLoan { loan: 0, failure: None },
        vec![raw::CleanupPlan {
            id: 0,
            span: span(),
            actions: vec![raw::CleanupAction::DropPlace(0)],
        }],
        0,
    )
}
