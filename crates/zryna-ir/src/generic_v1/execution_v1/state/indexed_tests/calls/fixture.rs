//! Private independently checked state inputs, never source or executable authority.
use crate::generic_v1::execution_v1::{
    plan, raw,
    type_world::{Builder, Kind, TypeWorld},
    typed,
};
use typed::borrowed_match_test_support::{instruction, span, value};
pub(super) fn fixture(dynamic: bool, access: raw::Access) -> (TypeWorld, raw::Program) {
    let mut b = Builder::new();
    let integer = b.declare(Kind::I32, &[], &mut 0).expect("i32");
    let enumeration =
        b.declare(Kind::Enum { module: 0, declaration: 0, arity: 0 }, &[], &mut 0).expect("enum");
    b.set_members(enumeration, &[Some(integer), None], &mut 0).expect("two variants");
    let array = b.declare(Kind::FixedArray { length: 1 }, &[enumeration], &mut 0).expect("array");
    let world = b.seal(&mut 0).expect("actual type-world kernel");
    let f = raw::Function {
        key: vec![],
        original_span: span(0, 100),
        executable_export: None,
        parameters: vec![
            raw::Parameter::Value(value(0, array)),
            raw::Parameter::Value(value(1, integer)),
        ],
        result_type: raw::ValueType::Unit,
        places: vec![
            raw::Place { id: 0, ty: array, span: span(0, 100), kind: raw::PlaceKind::Parameter(0) },
            raw::Place {
                id: 1,
                ty: enumeration,
                span: span(0, 100),
                kind: raw::PlaceKind::FixedArrayConstant { base: 0, index: 0 },
            },
            raw::Place {
                id: 2,
                ty: integer,
                span: span(0, 100),
                kind: raw::PlaceKind::EnumPayload { base: 1, variant: 0 },
            },
        ],
        loans: vec![raw::Loan {
            id: 0,
            referent: enumeration,
            access,
            region: raw::Region { span: span(20, 40), role: raw::RegionRole::Lexical },
            origin: if dynamic {
                raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(0), index: 1 }
            } else {
                raw::LoanOrigin::Place(1)
            },
        }],
        blocks: blocks(enumeration, integer, dynamic),
        cleanup_plans: vec![
            raw::CleanupPlan { id: 0, span: span(25, 26), actions: vec![] },
            raw::CleanupPlan {
                id: 1,
                span: span(25, 26),
                actions: vec![raw::CleanupAction::EndLoan(0)],
            },
        ],
    };
    let p = raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: world.keys().map(<[u8]>::to_vec).collect(),
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![f, callee(enumeration, access)],
    };
    (world, p)
}
fn callee(enumeration: u32, access: raw::Access) -> raw::Function {
    let mut instructions = Vec::new();
    if access == raw::Access::Exclusive {
        instructions.push(instruction(
            raw::Operation::ClosedEnumConstruct { ty: enumeration, ordinal: 1, payload: None },
            Some(value(0, enumeration)),
        ));
        instructions.push(instruction(raw::Operation::ReplaceLoan { loan: 0, value: 0 }, None));
    }
    instructions.push(instruction(raw::Operation::EndLoan { loan: 0 }, None));
    raw::Function {
        key: vec![1],
        original_span: span(0, 100),
        executable_export: None,
        parameters: vec![raw::Parameter::Loan(raw::LoanDefinition {
            id: 0,
            referent: enumeration,
            access,
        })],
        result_type: raw::ValueType::Unit,
        places: vec![],
        loans: vec![raw::Loan {
            id: 0,
            referent: enumeration,
            access,
            origin: raw::LoanOrigin::Formal(0),
            region: raw::Region { span: span(0, 100), role: raw::RegionRole::Lexical },
        }],
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions,
            terminator: raw::Terminator::Trap {
                identity: raw::TrapIdentity::Allocation,
                cleanup: 0,
            },
        }],
        cleanup_plans: vec![raw::CleanupPlan { id: 0, span: span(25, 26), actions: vec![] }],
    }
}

fn blocks(enumeration: u32, integer: u32, dynamic: bool) -> Vec<raw::Block> {
    vec![
        raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::ClosedEnumMatch {
                ty: enumeration,
                scrutinee: raw::PlaceOrLoan::Place(1),
                mode: raw::MatchMode::Value,
                arms: vec![
                    raw::EnumArm {
                        ordinal: 0,
                        binding: None,
                        edge: raw::Edge { target: 1, arguments: vec![] },
                    },
                    raw::EnumArm {
                        ordinal: 1,
                        binding: None,
                        edge: raw::Edge { target: 2, arguments: vec![] },
                    },
                ],
            },
        },
        raw::Block {
            id: 1,
            value_parameters: vec![],
            instructions: vec![
                instruction(
                    raw::Operation::BeginLoan { loan: 0, failure: dynamic.then_some(0) },
                    None,
                ),
                instruction(
                    raw::Operation::SourceCall {
                        module: 0,
                        function: 1,
                        arguments: vec![raw::CallArgument::Loan(0)],
                        failure: Some(1),
                    },
                    None,
                ),
                instruction(raw::Operation::EndLoan { loan: 0 }, None),
                instruction(
                    raw::Operation::ReadPlace { place: 2, mode: raw::ReadMode::Copy },
                    Some(value(2, integer)),
                ),
            ],
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        },
        raw::Block {
            id: 2,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        },
    ]
}
pub(super) fn derive(
    world: &TypeWorld,
    p: &raw::Program,
) -> Result<plan::Plan, crate::generic_v1::Failure> {
    let typed = typed::container_test_support::phase_inputs(p, world);
    plan::derive(p, &typed, &mut 0)
}
