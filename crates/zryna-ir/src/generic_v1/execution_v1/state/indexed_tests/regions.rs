//! Checked metadata regions only: no source proof, executable seal or live permission.
use super::super::{Context, access, raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use zryna_source::UntrustedSpan;
struct Types {
    world: TypeWorld,
    integer: u32,
    record: u32,
    vector: u32,
    holder: u32,
}
fn types() -> Types {
    let mut b = Builder::new();
    let integer = b.declare(Kind::I32, &[], &mut 0).expect("i32");
    let record = b
        .declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
        .expect("record");
    b.set_members(record, &[Some(integer), Some(integer)], &mut 0).expect("two fields");
    let vector = b.declare(Kind::Vec, &[record], &mut 0).expect("Vec<record>");
    let holder = b
        .declare(Kind::Struct { module: 0, declaration: 1, arity: 0 }, &[], &mut 0)
        .expect("holder");
    b.set_members(holder, &[Some(vector), Some(vector)], &mut 0)
        .expect("two independent containers");
    let world = b.seal(&mut 0).expect("independently checked private type kernel");
    Types { world, integer, record, vector, holder }
}
fn loans(t: &Types, span: UntrustedSpan) -> Vec<raw::Loan> {
    (0..4)
        .map(|id| raw::Loan {
            id,
            referent: if id % 2 == 0 { t.record } else { t.integer },
            access: raw::Access::Shared,
            region: raw::Region { span, role: raw::RegionRole::Lexical },
            origin: match id {
                0 => raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(1), index: 1 },
                1 => raw::LoanOrigin::StaticProjection {
                    parent: 0,
                    projection: raw::StaticProjection::StructField(0),
                },
                2 => raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(2), index: 1 },
                _ => raw::LoanOrigin::StaticProjection {
                    parent: 2,
                    projection: raw::StaticProjection::StructField(1),
                },
            },
        })
        .collect()
}
fn places(t: &Types, span: UntrustedSpan) -> Vec<raw::Place> {
    vec![
        raw::Place { id: 0, ty: t.holder, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: 1,
            ty: t.vector,
            span,
            kind: raw::PlaceKind::StructField { base: 0, ordinal: 0 },
        },
        raw::Place {
            id: 2,
            ty: t.vector,
            span,
            kind: raw::PlaceKind::StructField { base: 0, ordinal: 1 },
        },
    ]
}
fn program(t: &Types) -> raw::Program {
    let span = UntrustedSpan { file: 0, start: 0, end: 100 };
    let f = raw::Function {
        key: vec![],
        original_span: span,
        executable_export: None,
        parameters: vec![
            raw::Parameter::Value(raw::ValueDefinition {
                id: 0,
                ty: raw::ValueType::Stored(t.holder),
            }),
            raw::Parameter::Value(raw::ValueDefinition {
                id: 1,
                ty: raw::ValueType::Stored(t.integer),
            }),
        ],
        result_type: raw::ValueType::Unit,
        places: places(t, span),
        loans: loans(t, span),
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: 0,
            span,
            actions: vec![raw::CleanupAction::DropPlace(0)],
        }],
    };
    raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: t.world.keys().map(<[u8]>::to_vec).collect(),
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![f],
    }
}
#[test]
fn static_descendants_cannot_narrow_dynamic_region_but_outer_static_fields_stay_disjoint() {
    let t = types();
    let mut p = program(&t);
    {
        let checked = typed::container_test_support::phase_inputs(&p, &t.world);
        let c = Context { program: &p, typed: &checked, function: 0 };
        assert!(
            !access::overlaps(&c, raw::PlaceOrLoan::Loan(1), raw::PlaceOrLoan::Loan(3)),
            "separate outer containers remain disjoint"
        );
    }
    p.functions[0].loans[2].origin =
        raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(1), index: 1 };
    let checked = typed::container_test_support::phase_inputs(&p, &t.world);
    let c = Context { program: &p, typed: &checked, function: 0 };
    assert!(
        access::overlaps(&c, raw::PlaceOrLoan::Loan(1), raw::PlaceOrLoan::Loan(3)),
        "different descendant fields cannot narrow the complete dynamic container region"
    );
}
