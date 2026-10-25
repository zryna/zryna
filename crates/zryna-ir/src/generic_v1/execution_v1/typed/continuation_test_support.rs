//! Private owning-phase fixtures; these never construct source proof or execution authority.
use super::raw;
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use zryna_source::UntrustedSpan;

pub(in crate::generic_v1::execution_v1) struct Types {
    pub(in crate::generic_v1::execution_v1) world: TypeWorld,
    pub(in crate::generic_v1::execution_v1) string: u32,
    pub(in crate::generic_v1::execution_v1) integer: u32,
    pub(in crate::generic_v1::execution_v1) vector: u32,
    pub(in crate::generic_v1::execution_v1) weak: u32,
    pub(in crate::generic_v1::execution_v1) shared: u32,
    pub(in crate::generic_v1::execution_v1) foreign_shared: u32,
    pub(in crate::generic_v1::execution_v1) nominal_weak: u32,
}
impl Types {
    pub(in crate::generic_v1::execution_v1) fn new() -> Self {
        let mut b = Builder::new();
        let string = b.declare(Kind::String, &[], &mut 0).expect("String");
        let integer = b.declare(Kind::I32, &[], &mut 0).expect("i32");
        let vector = b.declare(Kind::Vec, &[string], &mut 0).expect("Vec<String>");
        let weak = b.declare(Kind::Weak, &[string], &mut 0).expect("Weak<String>");
        let shared = b.declare(Kind::Shared, &[string], &mut 0).expect("Shared<String>");
        let a = b
            .declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
            .expect("same-shaped nominal A");
        b.set_members(a, &[Some(integer)], &mut 0).expect("A payload");
        let nominal_weak = b.declare(Kind::Weak, &[a], &mut 0).expect("Weak<A>");
        let other = b
            .declare(Kind::Struct { module: 0, declaration: 1, arity: 0 }, &[], &mut 0)
            .expect("same-shaped nominal B");
        b.set_members(other, &[Some(integer)], &mut 0).expect("B same-shaped payload");
        let foreign_shared = b.declare(Kind::Shared, &[other], &mut 0).expect("Shared<B>");
        Self {
            world: b.seal(&mut 0).expect("private original world"),
            string,
            integer,
            vector,
            weak,
            shared,
            foreign_shared,
            nominal_weak,
        }
    }
}
pub(in crate::generic_v1::execution_v1) fn span() -> UntrustedSpan {
    UntrustedSpan { file: 0, start: 20, end: 30 }
}
pub(in crate::generic_v1::execution_v1) fn value(id: u32, ty: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }
}
pub(in crate::generic_v1::execution_v1) fn instruction(
    operation: raw::Operation,
) -> raw::Instruction {
    raw::Instruction { site_span: span(), result: None, operation }
}
fn place(id: u32, ty: u32, kind: raw::PlaceKind) -> raw::Place {
    raw::Place { id, ty, span: span(), kind }
}
fn cleanup(id: u32, actions: Vec<raw::CleanupAction>) -> raw::CleanupPlan {
    raw::CleanupPlan { id, span: span(), actions }
}
fn program(w: &Types, f: raw::Function) -> raw::Program {
    raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: w.world.keys().map(<[u8]>::to_vec).collect(),
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![f],
    }
}
fn function() -> raw::Function {
    raw::Function {
        key: vec![],
        original_span: UntrustedSpan { file: 0, start: 0, end: 100 },
        executable_export: None,
        parameters: vec![],
        result_type: raw::ValueType::Unit,
        places: vec![],
        loans: vec![],
        blocks: vec![],
        cleanup_plans: vec![],
    }
}
pub(in crate::generic_v1::execution_v1) fn push(
    w: &Types,
    access: Option<raw::Access>,
) -> raw::Program {
    let mut f = function();
    let (target, rhs, failure, exit) = if let Some(access) = access {
        f.parameters.push(raw::Parameter::Loan(raw::LoanDefinition {
            id: 0,
            referent: w.vector,
            access,
        }));
        f.parameters.push(raw::Parameter::Value(value(0, w.string)));
        f.places.push(place(0, w.string, raw::PlaceKind::Parameter(1)));
        f.loans.push(raw::Loan {
            id: 0,
            referent: w.vector,
            access,
            region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
            origin: raw::LoanOrigin::Formal(0),
        });
        (
            raw::PlaceOrLoan::Loan(0),
            0,
            vec![raw::CleanupAction::EndLoan(0), raw::CleanupAction::DropPlace(0)],
            vec![],
        )
    } else {
        f.parameters = vec![
            raw::Parameter::Value(value(0, w.vector)),
            raw::Parameter::Value(value(1, w.string)),
        ];
        f.places = vec![
            place(0, w.vector, raw::PlaceKind::Parameter(0)),
            place(1, w.string, raw::PlaceKind::Parameter(1)),
        ];
        (
            raw::PlaceOrLoan::Place(0),
            1,
            vec![raw::CleanupAction::DropPlace(1), raw::CleanupAction::DropPlace(0)],
            vec![raw::CleanupAction::DropPlace(0)],
        )
    };
    let mut instructions =
        vec![instruction(raw::Operation::VecPush { vector: target, value: rhs, failure: 0 })];
    if access.is_some() {
        instructions.push(instruction(raw::Operation::EndLoan { loan: 0 }));
    }
    f.blocks.push(raw::Block {
        id: 0,
        value_parameters: vec![],
        instructions,
        terminator: raw::Terminator::Return { value: None, cleanup: 1 },
    });
    f.cleanup_plans = vec![cleanup(0, failure), cleanup(1, exit)];
    program(w, f)
}
pub(in crate::generic_v1::execution_v1) fn upgrade(w: &Types) -> raw::Program {
    let mut f = function();
    f.parameters = vec![raw::Parameter::Value(value(0, w.weak))];
    f.places = vec![
        place(0, w.weak, raw::PlaceKind::Parameter(0)),
        place(1, w.shared, raw::PlaceKind::Temporary(1)),
    ];
    f.blocks = vec![
        raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::WeakUpgradeBranch {
                weak: raw::PlaceOrLoan::Place(0),
                success_binding: value(1, w.shared),
                success: raw::Edge { target: 1, arguments: vec![] },
                expired: raw::Edge { target: 2, arguments: vec![] },
                failure: 0,
            },
        },
        raw::Block {
            id: 1,
            value_parameters: vec![value(1, w.shared)],
            instructions: vec![],
            terminator: raw::Terminator::Return { value: None, cleanup: 1 },
        },
        raw::Block {
            id: 2,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::Return { value: None, cleanup: 2 },
        },
    ];
    f.cleanup_plans = vec![
        cleanup(0, vec![raw::CleanupAction::DropPlace(0)]),
        cleanup(1, vec![raw::CleanupAction::DropPlace(1), raw::CleanupAction::DropPlace(0)]),
        cleanup(2, vec![raw::CleanupAction::DropPlace(0)]),
    ];
    program(w, f)
}
