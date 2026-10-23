//! Owning graph controls: source capability is separate from argument validity and storage.
use super::*;

#[test]
fn acyclic_forward_closed_arguments_preserve_exact_container_capabilities() {
    let mut builder = Builder::closed(3).expect("complete closed inventory");
    let mut spent = 0;
    let wrapper = builder.declare(nominal(8, 1), &[2], &mut spent).expect("forward Vec argument");
    let scalar = builder.declare(Kind::I32, &[], &mut spent).expect("i32");
    let vector = builder.declare(Kind::Vec, &[scalar], &mut spent).expect("Vec<i32>");
    assert_eq!(vector, 2);
    builder.set_members(wrapper, &[Some(vector)], &mut spent).expect("real stored Vec member");
    let world = builder.seal(&mut spent).expect("finite forward argument graph");
    assert_eq!(world.fact(wrapper), Some(Fact { copy: false, clone: true }));
    assert_eq!(world.fact(vector), Some(Fact { copy: false, clone: true }));
    assert_eq!(world.fact(scalar), Some(Fact { copy: true, clone: true }));
}

fn nominal(declaration: u32, arity: u32) -> Kind {
    Kind::Struct { module: 0, declaration, arity }
}
fn graph_with_indirection() -> (Builder, u32, usize) {
    let mut spent = 0;
    let mut builder = Builder::new();
    let root = builder.declare(nominal(0, 0), &[], &mut spent).expect("Node");
    let vector = builder.declare(Kind::Vec, &[root], &mut spent).expect("Vec<Node>");
    builder.set_members(root, &[Some(vector)], &mut spent).expect("actual member");
    (builder, root, spent)
}
#[test]
fn same_key_vec_cycle_is_finite_and_cloneable_but_never_copy() {
    let (builder, root, mut spent) = graph_with_indirection();
    let world = builder.seal(&mut spent).expect("finite indirect SCC");
    let fact = world.fact(root).expect("same identity");
    assert!(!fact.copy);
    assert!(fact.clone);
    assert_eq!(world.len(), 2);
}
#[test]
fn by_value_cycle_rejects_instead_of_using_clone_fixed_point_as_layout_authority() {
    let mut builder = Builder::new();
    let mut spent = 0;
    let a = builder.declare(nominal(0, 0), &[], &mut spent).expect("a");
    let b = builder.declare(nominal(1, 0), &[], &mut spent).expect("b");
    builder.set_members(a, &[Some(b)], &mut spent).expect("a to b");
    builder.set_members(b, &[Some(a)], &mut spent).expect("b to a");
    assert!(builder.seal(&mut spent).is_err());
}
#[test]
fn opaque_owner_identity_phantom_validity_and_handle_clone_are_independent() {
    let mut builder = Builder::new();
    let mut spent = 0;
    let first = builder
        .declare(
            Kind::Opaque { owner: Frame { kind: 0, module: 0, index: 7 }, slot: 0 },
            &[],
            &mut spent,
        )
        .expect("function T");
    let second = builder
        .declare(
            Kind::Opaque { owner: Frame { kind: 1, module: 0, index: 7 }, slot: 0 },
            &[],
            &mut spent,
        )
        .expect("data T");
    assert_ne!(first, second);
    let scalar = builder.declare(Kind::I32, &[], &mut spent).expect("i32");
    let phantom =
        builder.declare(nominal(0, 1), &[first], &mut spent).expect("valid opaque argument");
    builder.set_members(phantom, &[Some(scalar)], &mut spent).expect("actual i32 only");
    let stored = builder.declare(nominal(1, 1), &[first], &mut spent).expect("Box<T>");
    builder.set_members(stored, &[Some(first)], &mut spent).expect("actual T");
    let shared = builder.declare(Kind::Shared, &[first], &mut spent).expect("Shared<T>");
    let weak = builder.declare(Kind::Weak, &[second], &mut spent).expect("Weak<T>");
    assert!(
        builder.declare(nominal(2, 1), &[u32::MAX], &mut spent).is_err(),
        "phantom arguments still valid"
    );
    let world = builder.seal(&mut spent).expect("original graph");
    assert_eq!(world.fact(first), Some(Fact { copy: false, clone: false }));
    assert_eq!(world.fact(phantom), Some(Fact { copy: true, clone: true }));
    assert!(!world.fact(stored).expect("Box<T>").clone);
    for id in [shared, weak] {
        assert_eq!(world.fact(id), Some(Fact { copy: false, clone: true }));
    }
}
#[test]
fn all_variant_payloads_and_zero_length_array_element_capability_are_required() {
    let mut builder = Builder::new();
    let mut spent = 0;
    let opaque = builder
        .declare(
            Kind::Opaque { owner: Frame { kind: 0, module: 0, index: 0 }, slot: 0 },
            &[],
            &mut spent,
        )
        .expect("T");
    let string = builder.declare(Kind::String, &[], &mut spent).expect("String");
    let choice = builder
        .declare(Kind::Enum { module: 0, declaration: 0, arity: 0 }, &[], &mut spent)
        .expect("Choice");
    builder
        .set_members(choice, &[None, Some(string), Some(opaque)], &mut spent)
        .expect("all variants");
    let zero =
        builder.declare(Kind::FixedArray { length: 0 }, &[opaque], &mut spent).expect("zero array");
    let world = builder.seal(&mut spent).expect("valid stored graph");
    for id in [choice, zero] {
        assert_eq!(world.fact(id), Some(Fact { copy: false, clone: false }));
    }
}
#[test]
fn failed_append_is_atomic_and_exact_full_analysis_credit_succeeds_before_first_extra() {
    let mut builder = Builder::new();
    let mut spent = super::super::budget::MAX_OWNER_UNITS;
    assert!(builder.declare(Kind::String, &[], &mut spent).is_err());
    assert_eq!(builder.len(), 0, "no header/index retention on failed prepay");
    assert_eq!(spent, super::super::budget::MAX_OWNER_UNITS);
    let mut retry = 0;
    builder.declare(Kind::String, &[], &mut retry).expect("pristine retry");
    let mut short = super::super::budget::MAX_OWNER_UNITS;
    assert!(builder.set_members(0, &[Some(0)], &mut short).is_err(), "scalar cannot gain members");
    let (builder, root, mut spent) = graph_with_indirection();
    let world = builder.seal(&mut spent).expect("full cold cost");
    let total = spent;
    for extra in [0, 1] {
        let (builder, _, before_seal) = graph_with_indirection();
        let mut used = super::super::budget::MAX_OWNER_UNITS - total + before_seal + extra;
        let result = builder.seal(&mut used);
        if extra == 0 {
            assert_eq!(result.expect("exact complete work").fact(root), world.fact(root));
            assert_eq!(used, super::super::budget::MAX_OWNER_UNITS);
        } else {
            assert!(matches!(result, Err(Failure::Diagnostics(v)) if v[0].code == "ZRYNA-I3201"));
        }
    }
}
#[test]
fn incomplete_or_mutated_member_inventory_cannot_publish() {
    let mut builder = Builder::new();
    let mut spent = 0;
    let root = builder.declare(nominal(0, 0), &[], &mut spent).expect("pending nominal");
    assert!(builder.set_members(root, &[Some(u32::MAX)], &mut spent).is_err());
    builder.set_members(root, &[Some(root)], &mut spent).expect("pristine member retry");
    assert!(builder.set_members(root, &[Some(root)], &mut spent).is_err(), "one exact definition");
    assert!(builder.seal(&mut spent).is_err(), "by-value self cycle remains invalid");
    let mut incomplete = Builder::new();
    incomplete.declare(nominal(0, 0), &[], &mut 0).expect("header");
    assert!(incomplete.seal(&mut 0).is_err());
}
#[test]
fn forward_closed_argument_identity_cycles_are_invalid_independently_of_storage() {
    for mutual in [false, true] {
        let count = if mutual { 3 } else { 2 };
        let mut builder = Builder::closed(count).expect("bounded closed inventory");
        let mut spent = 0;
        let first = builder
            .declare(nominal(0, 1), &[u32::from(mutual)], &mut spent)
            .expect("forward supplied argument");
        if mutual {
            builder.declare(nominal(1, 1), &[first], &mut spent).expect("mutual argument");
        }
        let scalar = builder.declare(Kind::I32, &[], &mut spent).expect("i32");
        builder.set_members(first, &[Some(scalar)], &mut spent).expect("finite actual storage");
        if mutual {
            builder.set_members(1, &[Some(scalar)], &mut spent).expect("other storage");
        }
        assert!(builder.seal(&mut spent).is_err(), "supplied keys cannot contain cycles");
    }
}

#[test]
fn cyclic_complete_arguments_reject_separately_from_stored_members() {
    let mut builder = Builder::closed(2).expect("closed graph slots");
    let mut spent = 0;
    let invalid = builder.declare(nominal(9, 1), &[0], &mut spent).expect("forward raw argument");
    let scalar = builder.declare(Kind::I32, &[], &mut spent).expect("stored scalar");
    builder.set_members(invalid, &[Some(scalar)], &mut spent).expect("phantom argument");
    assert!(
        builder.seal(&mut spent).is_err(),
        "an infinite supplied argument key is invalid even when phantom"
    );
}

mod container_members;
