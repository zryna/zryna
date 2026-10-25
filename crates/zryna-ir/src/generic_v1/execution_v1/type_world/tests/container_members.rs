//! Retained container descriptions do not redefine validity/storage/capability relations.
use super::*;
fn containers() -> [Kind; 5] {
    [
        Kind::FixedArray { length: 0 },
        Kind::FixedArray { length: 2 },
        Kind::Vec,
        Kind::Shared,
        Kind::Weak,
    ]
}
#[test]
fn each_container_retains_exact_one_argument_and_member_including_zero_array() {
    let mut builder = Builder::new();
    let mut spent = 0;
    let opaque = builder
        .declare(
            Kind::Opaque { owner: Frame { kind: 0, module: 0, index: 0 }, slot: 0 },
            &[],
            &mut spent,
        )
        .expect("actual opaque atom");
    let mut rows = Vec::new();
    for kind in containers() {
        let id = builder
            .declare(kind, &[opaque], &mut spent)
            .expect("container header and referenced member");
        rows.push((kind, id));
    }
    let world = builder.seal(&mut spent).expect("valid source graph");
    for (kind, id) in rows {
        assert_eq!(world.arguments(id), Some([opaque].as_slice()));
        assert_eq!(world.members(id), Some([Some(opaque)].as_slice()));
        assert_eq!(
            world.fact(id),
            Some(Fact { copy: false, clone: matches!(kind, Kind::Shared | Kind::Weak) })
        );
    }
}
#[test]
fn forward_closed_container_referent_is_retained_without_requiring_source_discovery_order() {
    for kind in containers() {
        let mut builder = Builder::closed(2).expect("closed universe");
        let mut spent = 0;
        let container =
            builder.declare(kind, &[1], &mut spent).expect("forward exact source argument");
        let element = builder.declare(Kind::I32, &[], &mut spent).expect("actual i32 node");
        let world = builder.seal(&mut spent).expect("valid forward source world");
        assert_eq!(element, 1);
        assert_eq!(world.members(container), Some([Some(element)].as_slice()));
        assert_eq!(
            world.fact(container),
            Some(Fact { copy: matches!(kind, Kind::FixedArray { .. }), clone: true })
        );
    }
}
#[test]
fn retained_indirect_members_do_not_create_by_value_or_supplied_argument_cycles() {
    for kind in [Kind::Vec, Kind::Shared, Kind::Weak] {
        let mut builder = Builder::new();
        let mut spent = 0;
        let root = builder.declare(nominal(0, 0), &[], &mut spent).expect("Node source header");
        let indirect = builder.declare(kind, &[root], &mut spent).expect("indirect Node owner");
        builder.set_members(root, &[Some(indirect)], &mut spent).expect("actual indirect member");
        let world = builder.seal(&mut spent).expect("finite indirect SCC remains valid");
        assert_eq!(world.members(indirect), Some([Some(root)].as_slice()));
        assert_eq!(world.fact(root), Some(Fact { copy: false, clone: true }));
    }
    for kind in containers() {
        let mut builder = Builder::closed(2).expect("closed slots");
        let mut spent = 0;
        builder.declare(kind, &[0], &mut spent).expect("untrusted self argument header");
        builder.declare(Kind::I32, &[], &mut spent).expect("remaining node");
        builder
            .seal(&mut spent)
            .expect_err("capability terminal cannot legalize a cyclic complete argument");
    }
}
#[test]
fn container_frontiers_keep_real_children_zero_fanout_and_count_terminals_separate() {
    use crate::generic_v1::execution_v1::frontier::{CloneNodeKind, Table};
    let mut builder = Builder::new();
    let mut spent = 0;
    let string = builder.declare(Kind::String, &[], &mut spent).expect("String");
    let mut ids = Vec::new();
    for kind in containers() {
        ids.push(builder.declare(kind, &[string], &mut spent).expect("container"));
    }
    let world = builder.seal(&mut spent).expect("all source Clone facts");
    let table = Table::build(&world, &mut spent).expect("retained source clone frontiers");
    for (index, expected) in [
        CloneNodeKind::FixedArray { length: 0 },
        CloneNodeKind::FixedArray { length: 2 },
        CloneNodeKind::VecEach,
    ]
    .into_iter()
    .enumerate()
    {
        let node = table.node(ids[index]).expect("container recipe");
        assert_eq!(node.kind, expected);
        assert_eq!(node.children, [Some(string)]);
        assert_eq!(node.may_fail, index != 0, "zero-length traversal does not visit String");
    }
    for (index, expected) in [(3, CloneNodeKind::SharedCount), (4, CloneNodeKind::WeakCount)] {
        let node = table.node(ids[index]).expect("count terminal");
        assert_eq!(node.kind, expected);
        assert!(node.children.is_empty(), "count acquisition does not clone payload");
        assert!(node.may_fail);
    }
}
fn scalar_builder() -> (Builder, u32) {
    let mut builder = Builder::new();
    let scalar = builder.declare(Kind::I32, &[], &mut 0).expect("pre-existing scalar fixture");
    (builder, scalar)
}
#[test]
fn exact_retained_member_credit_is_required_before_any_container_publication() {
    // Argument probe+identity2, one existing index comparison9, copied argument1,
    // retained referenced member1, new node/index slots2, no shifted index slots.
    const DECLARATION_CREDIT: usize = 2 + 9 + 1 + 1 + 2;
    const COMPLETED_PRE_RETENTION: usize = 2 + 9;
    for kind in containers() {
        let (mut builder, scalar) = scalar_builder();
        let mut cold = 0;
        let id = builder.declare(kind, &[scalar], &mut cold).expect("cold constructor");
        assert_eq!(cold, DECLARATION_CREDIT, "one actual retained member is prepaid");
        let world = builder.seal(&mut 0).expect("separate graph phase control");
        assert_eq!(world.members(id), Some([Some(scalar)].as_slice()));
        let (mut exact_builder, scalar) = scalar_builder();
        let mut exact = super::super::super::budget::MAX_OWNER_UNITS - DECLARATION_CREDIT;
        exact_builder.declare(kind, &[scalar], &mut exact).expect("exact constructor credit");
        assert_eq!(exact, super::super::super::budget::MAX_OWNER_UNITS);
        let (mut short_builder, scalar) = scalar_builder();
        let mut short = super::super::super::budget::MAX_OWNER_UNITS - DECLARATION_CREDIT + 1;
        let before = short;
        short_builder
            .declare(kind, &[scalar], &mut short)
            .expect_err("missing member-retention credit");
        assert_eq!(short_builder.len(), 1, "no partial container header/index/member");
        assert_eq!(
            short,
            before + COMPLETED_PRE_RETENTION,
            "completed probes only; failed retention prepay is atomic"
        );
        let retried =
            short_builder.declare(kind, &[scalar], &mut 0).expect("pristine constructor retry");
        assert_eq!(retried, 1);
        let world = short_builder.seal(&mut 0).expect("pristine sealed graph");
        assert_eq!(world.members(retried), Some([Some(scalar)].as_slice()));
    }
}
