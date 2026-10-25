use super::super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use zryna_source::UntrustedSpan;
#[derive(Clone, Copy)]
pub(super) enum Case {
    Vector,
    Shared,
    Downgrade,
}
pub(super) fn span() -> UntrustedSpan {
    UntrustedSpan { file: 0, start: 20, end: 30 }
}
pub(super) fn value(id: u32, ty: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }
}
fn world() -> (TypeWorld, u32, u32, u32, u32) {
    let mut b = Builder::new();
    let string = b.declare(Kind::String, &[], &mut 0).expect("String");
    let vector = b.declare(Kind::Vec, &[string], &mut 0).expect("Vec<String>");
    let shared = b.declare(Kind::Shared, &[string], &mut 0).expect("Shared<String>");
    let weak = b.declare(Kind::Weak, &[string], &mut 0).expect("Weak<String>");
    (b.seal(&mut 0).expect("private source capability world"), string, vector, shared, weak)
}
pub(super) fn program(case: Case) -> (raw::Program, TypeWorld) {
    let (world, string, vector, shared, weak) = world();
    let (parameter_types, result_type, operation) = match case {
        Case::Vector => (
            vec![string, string],
            vector,
            raw::Operation::VecConstruct { ty: vector, elements: vec![0, 1], failure: 0 },
        ),
        Case::Shared => (
            vec![string],
            shared,
            raw::Operation::SharedConstruct { ty: shared, value: 0, failure: 0 },
        ),
        Case::Downgrade => (
            vec![shared],
            weak,
            raw::Operation::WeakDowngrade { shared: raw::PlaceOrLoan::Place(0), failure: 0 },
        ),
    };
    let result = u32::try_from(parameter_types.len()).expect("small fixture");
    let parameters = parameter_types
        .iter()
        .enumerate()
        .map(|(id, ty)| {
            raw::Parameter::Value(value(u32::try_from(id).expect("small fixture"), *ty))
        })
        .collect();
    let mut places = parameter_types
        .iter()
        .enumerate()
        .map(|(id, ty)| {
            let id = u32::try_from(id).expect("small fixture");
            raw::Place { id, ty: *ty, span: span(), kind: raw::PlaceKind::Parameter(id) }
        })
        .collect::<Vec<_>>();
    places.push(raw::Place {
        id: result,
        ty: result_type,
        span: span(),
        kind: raw::PlaceKind::Temporary(result),
    });
    let actions = (0..result).rev().map(raw::CleanupAction::DropPlace).collect();
    let return_actions = if matches!(case, Case::Downgrade) {
        vec![raw::CleanupAction::DropPlace(0)]
    } else {
        vec![]
    };
    let f = raw::Function {
        key: vec![],
        original_span: UntrustedSpan { file: 0, start: 0, end: 100 },
        executable_export: None,
        parameters,
        result_type: raw::ValueType::Stored(result_type),
        places,
        loans: vec![],
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![raw::Instruction {
                site_span: span(),
                result: Some(value(result, result_type)),
                operation,
            }],
            terminator: raw::Terminator::Return { value: Some(result), cleanup: 1 },
        }],
        cleanup_plans: vec![
            raw::CleanupPlan { id: 0, span: span(), actions },
            raw::CleanupPlan { id: 1, span: span(), actions: return_actions },
        ],
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
pub(super) fn inputs(p: &raw::Program, world: &TypeWorld) -> typed::Typed {
    typed::container_test_support::phase_inputs(p, world)
}
