//! Exact invoked WASI signatures, independently authored for the test-only lower boundary.

use wasm_encoder::{
    Alias, ComponentBuilder, ComponentExportKind, ComponentOuterAliasKind, ComponentTypeRef,
    ComponentValType, InstanceType, PrimitiveValType, TypeBounds,
};

pub(super) fn environment(component: &mut ComponentBuilder) -> u32 {
    let mut interface = InstanceType::new();
    interface.ty().defined_type().tuple([ComponentValType::Primitive(PrimitiveValType::String); 2]);
    interface.ty().defined_type().list(ComponentValType::Type(0));
    function(&mut interface, "get-environment", 1);
    let arguments = interface.type_count();
    interface.ty().defined_type().list(ComponentValType::Primitive(PrimitiveValType::String));
    function(&mut interface, "get-arguments", arguments);
    let cwd = interface.type_count();
    interface.ty().defined_type().option(ComponentValType::Primitive(PrimitiveValType::String));
    function(&mut interface, "initial-cwd", cwd);
    let ty = component.type_instance(None, &interface);
    let instance = component.import("wasi:cli/environment@0.2.12", ComponentTypeRef::Instance(ty));
    component.alias_export(instance, "get-environment", ComponentExportKind::Func)
}

pub(super) fn filesystem(component: &mut ComponentBuilder) -> u32 {
    let mut types = InstanceType::new();
    types.export("descriptor", ComponentTypeRef::Type(TypeBounds::SubResource));
    let ty = component.type_instance(None, &types);
    let instance = component.import("wasi:filesystem/types@0.2.12", ComponentTypeRef::Instance(ty));
    let descriptor = component.alias_export(instance, "descriptor", ComponentExportKind::Type);
    let mut preopens = InstanceType::new();
    preopens.alias(Alias::Outer {
        kind: ComponentOuterAliasKind::Type,
        count: 1,
        index: descriptor,
    });
    preopens.export("descriptor", ComponentTypeRef::Type(TypeBounds::Eq(0)));
    preopens.ty().defined_type().own(1);
    preopens
        .ty()
        .defined_type()
        .tuple([ComponentValType::Type(2), ComponentValType::Primitive(PrimitiveValType::String)]);
    preopens.ty().defined_type().list(ComponentValType::Type(3));
    function(&mut preopens, "get-directories", 4);
    let ty = component.type_instance(None, &preopens);
    let instance =
        component.import("wasi:filesystem/preopens@0.2.12", ComponentTypeRef::Instance(ty));
    component.alias_export(instance, "get-directories", ComponentExportKind::Func)
}

fn function(interface: &mut InstanceType, name: &str, result: u32) {
    let index = interface.type_count();
    let mut function = interface.ty().function();
    function.params([] as [(&str, ComponentValType); 0]);
    function.result(Some(ComponentValType::Type(result)));
    interface.export(name, ComponentTypeRef::Func(index));
}
