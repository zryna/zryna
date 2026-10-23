//! Actual CFG/place/terminator kernels; no source proof or execution seal is constructed.
use super::{Function, check, raw};
use crate::generic_v1::execution_v1::{
    places,
    type_world::{Builder, Kind, TypeWorld},
    typed::cfg::Graph,
};
use zryna_source::UntrustedSpan;
fn value(id: u32, ty: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }
}
fn function(source: u32, payload: u32) -> raw::Function {
    let span = UntrustedSpan { file: 0, start: 0, end: 0 };
    raw::Function {
        key: vec![],
        original_span: span,
        executable_export: None,
        parameters: vec![raw::Parameter::Value(value(0, source))],
        result_type: raw::ValueType::Unit,
        places: vec![
            raw::Place { id: 0, ty: source, span, kind: raw::PlaceKind::Parameter(0) },
            raw::Place { id: 1, ty: payload, span, kind: raw::PlaceKind::Temporary(1) },
        ],
        loans: vec![],
        cleanup_plans: vec![],
        blocks: vec![
            raw::Block {
                id: 0,
                value_parameters: vec![],
                instructions: vec![],
                terminator: raw::Terminator::ClosedEnumMatch {
                    ty: source,
                    scrutinee: raw::PlaceOrLoan::Place(0),
                    mode: raw::MatchMode::Value,
                    arms: vec![
                        raw::EnumArm {
                            ordinal: 0,
                            binding: None,
                            edge: raw::Edge { target: 1, arguments: vec![] },
                        },
                        raw::EnumArm {
                            ordinal: 1,
                            binding: Some(raw::MatchBinding::Value(value(1, payload))),
                            edge: raw::Edge { target: 2, arguments: vec![] },
                        },
                    ],
                },
            },
            raw::Block {
                id: 1,
                value_parameters: vec![],
                instructions: vec![],
                terminator: raw::Terminator::Return { value: None, cleanup: 0 },
            },
            raw::Block {
                id: 2,
                value_parameters: vec![value(1, payload)],
                instructions: vec![],
                terminator: raw::Terminator::Return { value: None, cleanup: 0 },
            },
        ],
    }
}
fn checked(f: &raw::Function, world: &TypeWorld) -> Function {
    let graph =
        Graph::build(f, world.len(), &mut 0).expect("actual dense CFG and binding-edge checks");
    let values = graph.types().collect::<Vec<_>>();
    let places =
        places::check(f, &values, world, &mut 0).expect("actual exact place/type/owner checks");
    Function {
        graph,
        values,
        places,
        callees: vec![],
        sites: vec![],
        may_fail: false,
        upgrades: vec![],
    }
}
#[test]
fn enum_match_rejects_coherent_wrong_active_payload_and_equal_shaped_foreign_source() {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("valid checked test fixture");
    let integer = builder.declare(Kind::I32, &[], &mut 0).expect("valid checked test fixture");
    let first = builder
        .declare(Kind::Enum { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
        .expect("valid checked test fixture");
    let other = builder
        .declare(Kind::Enum { module: 0, declaration: 1, arity: 0 }, &[], &mut 0)
        .expect("valid checked test fixture");
    for ty in [first, other] {
        builder.set_members(ty, &[None, Some(string)], &mut 0).expect("valid checked test fixture");
    }
    let world = builder.seal(&mut 0).expect("valid checked test fixture");
    let original = function(first, string);
    check(&original, &checked(&original, &world), &world, 0, &mut 0)
        .expect("exact active String binding");
    let wrong_payload = function(first, integer);
    check(&wrong_payload, &checked(&wrong_payload, &world), &world, 0, &mut 0)
        .expect_err("CFG/places pass, enum owner rejects i32 active payload");
    let mut wrong_source = function(other, string);
    let raw::Terminator::ClosedEnumMatch { ty, .. } = &mut wrong_source.blocks[0].terminator else {
        unreachable!()
    };
    *ty = first;
    check(&wrong_source, &checked(&wrong_source, &world), &world, 0, &mut 0)
        .expect_err("equal-shaped enum B is not exact enum A");
    check(&original, &checked(&original, &world), &world, 0, &mut 0).expect("pristine exact retry");
}
