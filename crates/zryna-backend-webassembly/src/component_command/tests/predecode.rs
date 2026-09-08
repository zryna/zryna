use std::convert::Infallible;

use wasm_encoder::{
    Component, ComponentSection, ComponentTypeRef, ComponentTypeSection, ComponentValType,
    InstanceType, PrimitiveValType, TypeBounds,
    reencode::{Error, Reencode, ReencodeComponent, RoundtripReencoder},
};

use super::{
    super::{ValidatedCommandComponent, audit, emit_command_self_check},
    fixtures,
};

struct ImportedTypeMutation {
    depth: u32,
    exports: u32,
    changed: bool,
}

impl Reencode for ImportedTypeMutation {
    type Error = Infallible;
}

impl ReencodeComponent for ImportedTypeMutation {
    fn component_instance_type(
        &mut self,
        declarations: Box<[wasmparser::InstanceTypeDeclaration<'_>]>,
    ) -> Result<InstanceType, Error<Self::Error>> {
        let wall_clock = declarations.iter().any(|declaration| {
            matches!(declaration,
            wasmparser::InstanceTypeDeclaration::Export { name, .. } if name.name == "datetime")
        });
        let mut instance = RoundtripReencoder.component_instance_type(declarations)?;
        if wall_clock && !self.changed {
            self.changed = true;
            let mut value = ComponentValType::Primitive(PrimitiveValType::Bool);
            for _ in 0..self.depth {
                let index = instance.type_count();
                instance.ty().defined_type().list(value);
                value = ComponentValType::Type(index);
            }
            let index = match value {
                ComponentValType::Type(index) => index,
                ComponentValType::Primitive(ty) => {
                    let index = instance.type_count();
                    instance.ty().defined_type().primitive(ty);
                    index
                }
            };
            for export in 0..self.exports {
                instance.export(
                    format!("bound-{export}"),
                    ComponentTypeRef::Type(TypeBounds::Eq(index)),
                );
            }
        }
        Ok(instance)
    }
}

fn baseline() -> ValidatedCommandComponent {
    emit_command_self_check(&fixtures::program(), &fixtures::sources(), "add", &[20, 22], 42)
        .expect("authenticated emitted component baseline")
}

fn check(
    component: &ValidatedCommandComponent,
    bytes: &[u8],
) -> Result<(), zryna_diagnostics::Diagnostic> {
    audit::audit(bytes, &component.core, &component.invocation, &component.world)
}

fn mutate(component: &ValidatedCommandComponent, depth: u32, exports: u32) -> Vec<u8> {
    let mut mutation = ImportedTypeMutation { depth, exports, changed: false };
    let mut output = Component::new();
    mutation
        .parse_component(&mut output, wasmparser::Parser::new(0), component.bytes())
        .expect("binary mutation");
    assert!(mutation.changed, "the actual command imports wall-clock.datetime");
    output.finish()
}

#[test]
fn real_component_reference_depth_rejects_first_extra_before_wit_decoding() {
    let component = baseline();
    check(&component, component.bytes()).expect("positive complete audit");
    // The exact-depth fixture reaches the WIT comparison, which rejects its extra public type.
    assert_eq!(
        check(&component, &mutate(&component, 32, 1)).expect_err("extra public type").code(),
        "ZRYNA-W4012"
    );
    assert_eq!(
        check(&component, &mutate(&component, 33, 1)).expect_err("depth before decode").code(),
        "ZRYNA-W4013"
    );
}

#[test]
fn real_component_identity_budget_accepts_exact_and_rejects_first_extra() {
    let component = baseline();
    let used =
        audit::identity_budget_used(component.bytes()).expect("accepted topology accounting");
    assert!(used < 4096);
    let extend = |count: usize| {
        let mut types = ComponentTypeSection::new();
        for _ in 0..count {
            types.ty().defined_type().primitive(PrimitiveValType::Bool);
        }
        let mut bytes = component.bytes().to_vec();
        types.append_to_component(&mut bytes);
        bytes
    };
    check(&component, &extend(4096 - used)).expect("exact conservative identity budget");
    assert_eq!(
        check(&component, &extend(4097 - used)).expect_err("first extra identity").code(),
        "ZRYNA-W4013"
    );
}

#[test]
fn real_component_public_type_identity_budget_accepts_exact_and_rejects_first_extra() {
    let component = baseline();
    let used =
        audit::identity_budget_used(component.bytes()).expect("accepted topology accounting");
    let remaining = 4096 - used;
    // The mutation adds one small underlying type plus one identity per public export. Both the
    // declarations and their imported instance use are conservatively charged before decoding.
    assert_eq!(remaining % 2, 0, "fixture must reach the exact prospective identity ceiling");
    let exact_exports = remaining / 2 - 1;
    let exact = mutate(&component, 0, exact_exports as u32);
    assert_eq!(audit::identity_budget_used(&exact).expect("exact public identity budget"), 4096);
    assert_eq!(
        check(&component, &exact).expect_err("extra public types after bounded decode").code(),
        "ZRYNA-W4012"
    );
    let first_extra = mutate(&component, 0, exact_exports as u32 + 1);
    assert_eq!(
        check(&component, &first_extra)
            .expect_err("first extra public identity before decode")
            .code(),
        "ZRYNA-W4013"
    );
}
