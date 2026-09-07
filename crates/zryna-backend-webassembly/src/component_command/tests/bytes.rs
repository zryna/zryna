use wasm_encoder::{ComponentSection, ComponentTypeSection, ComponentValType, PrimitiveValType};

use super::{
    super::{audit, emit_command_self_check},
    fixtures,
};

#[test]
fn real_component_accepts_exact_byte_ceiling_and_rejects_first_extra_before_parsing() {
    let component =
        emit_command_self_check(&fixtures::program(), &fixtures::sources(), "add", &[20, 22], 42)
            .expect("authentic component baseline");
    let mut names = (0..4096).map(|index| format!("field{index}")).collect::<Vec<_>>();
    let encode = |names: &[String]| {
        let mut section = ComponentTypeSection::new();
        section.ty().defined_type().record(
            names
                .iter()
                .map(|name| (name.as_str(), ComponentValType::Primitive(PrimitiveValType::Bool))),
        );
        let mut bytes = component.bytes().to_vec();
        section.append_to_component(&mut bytes);
        bytes
    };
    let initial = encode(&names);
    let mut remaining = 1024 * 1024 - initial.len();
    let name_cost = |length: usize| length + if length < 128 { 1 } else { 2 };
    for name in &mut names {
        let old = name.len();
        let length = (old..=256)
            .rev()
            .find(|length| name_cost(*length) - name_cost(old) <= remaining)
            .expect("a bounded field name length");
        remaining -= name_cost(length) - name_cost(old);
        name.extend(std::iter::repeat_n('x', length - old));
    }
    assert_eq!(remaining, 0, "bounded unused record fields fill the exact byte envelope");
    let mut exact = encode(&names);
    assert_eq!(exact.len(), 1024 * 1024);
    audit::audit(&exact, &component.core, &component.invocation, &component.world)
        .expect("exact byte ceiling with unchanged authenticated public shape");
    exact.push(0);
    let failure = audit::audit(&exact, &component.core, &component.invocation, &component.world)
        .expect_err("first extra component byte");
    assert_eq!(failure.code(), "ZRYNA-W4015");
    assert!(failure.message().contains("exceeds 1048576 bytes"));
}
