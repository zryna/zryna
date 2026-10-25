//! Source-independent actual place/CFG/state inputs, never a whole-seal constructor.
use super::{raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use typed::continuation_test_support::{instruction, span, value};
pub(in crate::generic_v1::execution_v1::state) fn input(loan: bool) -> (raw::Program, TypeWorld) {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("String");
    let boolean = builder.declare(Kind::Bool, &[], &mut 0).expect("bool");
    let world = builder.seal(&mut 0).expect("actual private checked types");
    let mut p = typed::continuation_test_support::push(
        &typed::continuation_test_support::Types::new(),
        None,
    );
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters = vec![raw::Parameter::Value(value(0, string))];
    f.result_type = raw::ValueType::Unit;
    f.places = vec![
        raw::Place { id: 0, ty: string, span: span(), kind: raw::PlaceKind::Parameter(0) },
        raw::Place { id: 1, ty: string, span: span(), kind: raw::PlaceKind::Temporary(2) },
    ];
    f.loans = Vec::new();
    let mut body = vec![raw::Instruction {
        site_span: span(),
        result: Some(value(2, string)),
        operation: raw::Operation::StringFromUtf8 { bytes: b"body".to_vec(), failure: 0 },
    }];
    if loan {
        f.loans.push(raw::Loan {
            id: 0,
            referent: string,
            access: raw::Access::Shared,
            origin: raw::LoanOrigin::Place(1),
            region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
        });
        body.push(instruction(raw::Operation::BeginLoan { loan: 0, failure: None }));
        body.push(instruction(raw::Operation::EndLoan { loan: 0 }));
    }
    body.push(instruction(raw::Operation::DropPlace { place: 1 }));
    f.blocks = vec![
        raw::Block {
            id: 0,
            value_parameters: Vec::new(),
            instructions: vec![raw::Instruction {
                site_span: span(),
                result: Some(value(1, boolean)),
                operation: raw::Operation::BoolLiteral(false),
            }],
            terminator: raw::Terminator::Jump(raw::Edge { target: 1, arguments: Vec::new() }),
        },
        raw::Block {
            id: 1,
            value_parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: raw::Terminator::Branch {
                condition: 1,
                yes: raw::Edge { target: 2, arguments: Vec::new() },
                no: raw::Edge { target: 3, arguments: Vec::new() },
            },
        },
        raw::Block {
            id: 2,
            value_parameters: Vec::new(),
            instructions: body,
            terminator: raw::Terminator::Jump(raw::Edge { target: 1, arguments: Vec::new() }),
        },
        raw::Block {
            id: 3,
            value_parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        },
    ];
    f.cleanup_plans = vec![raw::CleanupPlan {
        id: 0,
        span: span(),
        actions: vec![raw::CleanupAction::DropPlace(0)],
    }];
    (p, world)
}
