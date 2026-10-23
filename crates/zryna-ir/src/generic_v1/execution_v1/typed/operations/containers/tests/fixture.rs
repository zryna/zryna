use super::{Builder, Kind, TypeWorld, raw};
pub(super) struct Fixture {
    pub(super) world: TypeWorld,
    pub(super) string: u32,
    pub(super) integer: u32,
    pub(super) vector: u32,
    pub(super) vector_integer: u32,
    pub(super) shared: u32,
    pub(super) weak: u32,
    pub(super) shared_nominal: u32,
    pub(super) weak_other: u32,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let mut b = Builder::new();
        let string = b.declare(Kind::String, &[], &mut 0).expect("String");
        let integer = b.declare(Kind::I32, &[], &mut 0).expect("i32");
        let vector = b.declare(Kind::Vec, &[string], &mut 0).expect("Vec<String>");
        let vector_integer = b.declare(Kind::Vec, &[integer], &mut 0).expect("Vec<i32>");
        let shared = b.declare(Kind::Shared, &[string], &mut 0).expect("Shared<String>");
        let weak = b.declare(Kind::Weak, &[string], &mut 0).expect("Weak<String>");
        let a = b
            .declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
            .expect("A");
        let other = b
            .declare(Kind::Struct { module: 0, declaration: 1, arity: 0 }, &[], &mut 0)
            .expect("B");
        for ty in [a, other] {
            b.set_members(ty, &[Some(integer)], &mut 0).expect("same-shape exact nominal member");
        }
        let shared_nominal = b.declare(Kind::Shared, &[a], &mut 0).expect("Shared<A>");
        let weak_other = b.declare(Kind::Weak, &[other], &mut 0).expect("Weak<B>");
        Self {
            world: b.seal(&mut 0).expect("private source capability world"),
            string,
            integer,
            vector,
            vector_integer,
            shared,
            weak,
            shared_nominal,
            weak_other,
        }
    }
}
pub(super) fn instruction(id: u32, ty: u32, operation: raw::Operation) -> raw::Instruction {
    raw::Instruction {
        site_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        result: Some(raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }),
        operation,
    }
}
pub(super) fn program(world: &TypeWorld, params: &[u32], i: &raw::Instruction) -> raw::Program {
    let span = i.site_span;
    let parameters = params
        .iter()
        .enumerate()
        .map(|(id, ty)| {
            raw::Parameter::Value(raw::ValueDefinition {
                id: u32::try_from(id).expect("small fixture"),
                ty: raw::ValueType::Stored(*ty),
            })
        })
        .collect();
    let mut places = params
        .iter()
        .enumerate()
        .map(|(id, ty)| {
            let id = u32::try_from(id).expect("small fixture");
            raw::Place { id, ty: *ty, span, kind: raw::PlaceKind::Parameter(id) }
        })
        .collect::<Vec<_>>();
    if let Some(raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }) = i.result.as_ref() {
        places.push(raw::Place {
            id: u32::try_from(places.len()).expect("small fixture"),
            ty: *ty,
            span,
            kind: raw::PlaceKind::Temporary(*id),
        });
    }
    raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: world.keys().map(<[u8]>::to_vec).collect(),
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![raw::Function {
            key: vec![],
            original_span: span,
            executable_export: None,
            parameters,
            result_type: i.result.as_ref().map_or(raw::ValueType::Unit, |v| v.ty),
            places,
            loans: vec![],
            cleanup_plans: vec![],
            blocks: vec![raw::Block {
                id: 0,
                value_parameters: vec![],
                instructions: vec![i.clone()],
                terminator: raw::Terminator::Return {
                    value: i.result.as_ref().map(|v| v.id),
                    cleanup: 0,
                },
            }],
        }],
    }
}
