use wit_parser::{Handle, Resolve, Type, TypeDefKind, TypeId, WorldId, decoding::DecodedWasm};

use super::{
    super::{ValidatedCommandComponent, emit_command_self_check, type_graph},
    fixtures,
};

fn baseline() -> (ValidatedCommandComponent, Resolve, WorldId) {
    let component =
        emit_command_self_check(&fixtures::program(), &fixtures::sources(), "add", &[20, 22], 42)
            .expect("authentic emitted baseline");
    let DecodedWasm::Component(resolve, world) =
        wit_parser::decoding::decode(component.bytes()).expect("decode")
    else {
        panic!("executable component required");
    };
    type_graph::compare(&component.world, &resolve, world)
        .expect("complete resource graph baseline");
    (component, resolve, world)
}

fn borrowed_resource(resolve: &Resolve) -> (TypeId, TypeId) {
    resolve
        .types
        .iter()
        .find_map(|(id, ty)| match ty.kind {
            TypeDefKind::Handle(Handle::Borrow(resource)) => Some((id, resource)),
            _ => None,
        })
        .expect("the pinned world contains borrowed resource handles")
}

#[test]
fn resource_graph_rejects_splitting_one_identity_across_a_borrowed_alias() {
    let (component, mut resolve, world) = baseline();
    let (handle, resource) = borrowed_resource(&resolve);
    let copy = resolve.types.alloc(resolve.types[resource].clone());
    resolve.types[handle].kind = TypeDefKind::Handle(Handle::Borrow(copy));
    assert!(type_graph::compare(&component.world, &resolve, world).is_err());
}

#[test]
fn resource_graph_rejects_collapsing_two_distinct_resources() {
    let (component, mut resolve, world) = baseline();
    let resources = resolve
        .types
        .iter()
        .filter_map(|(id, ty)| matches!(ty.kind, TypeDefKind::Resource).then_some(id))
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(resources.len(), 2, "independent distinct resource definitions");
    resolve.types[resources[0]].kind = TypeDefKind::Type(Type::Id(resources[1]));
    assert!(type_graph::compare(&component.world, &resolve, world).is_err());
}

#[test]
fn resource_graph_rejects_replacing_a_borrow_with_ownership() {
    let (component, mut resolve, world) = baseline();
    let (handle, resource) = borrowed_resource(&resolve);
    resolve.types[handle].kind = TypeDefKind::Handle(Handle::Own(resource));
    assert!(type_graph::compare(&component.world, &resolve, world).is_err());
}
