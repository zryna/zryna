//! Source identity, nominal graph shape, cycles and root-demand admission.

use super::*;

#[test]
fn foreign_equal_text_source_authorities_and_cross_universe_type_ids_reject() {
    let first = sources();
    let second = sources();
    let graph = graph(&first, fixture_kind("Box<i32>"));
    assert!(verify(&graph, &second, StorageTarget::Linear32V1).is_err());
    let a = verify(&graph, &first, StorageTarget::Linear32V1).expect("first source layout");
    let b = verify(
        &super::graph(&second, fixture_kind("Box<i32>")),
        &second,
        StorageTarget::Linear32V1,
    )
    .expect("second source layout");
    let id = a.types().last().expect("first exact ID").id();
    assert!(b.type_by_id(id).is_none());
    let other = super::graph(&first, fixture_kind("Result<i32,bool>"));
    assert!(
        verify(&other, &first, StorageTarget::Linear32V1)
            .expect("different universe")
            .type_by_id(id)
            .is_none()
    );
}

#[test]
fn hostile_graph_identity_arity_ordinals_and_payload_references_reject_then_recover() {
    let sources = sources();
    let original = graph(&sources, fixture_kind("Choice<i32,bool>"));
    for mutation in 0..6 {
        let mut graph = original.clone();
        match mutation {
            0 => graph.types[3].id = raw::NodeId(0),
            1 => graph.types[3].span = None,
            2 => graph.declarations[0].parameters = 1,
            3 => {
                if let raw::TypeKind::Enum { arguments, .. } = &mut graph.types[3].kind {
                    arguments[0] = raw::NodeId(999);
                }
            }
            4 => {
                if let raw::TypeKind::Enum { variants, .. } = &mut graph.types[3].kind {
                    variants[0].ordinal = 1;
                }
            }
            _ => {
                if let raw::TypeKind::Enum { variants, .. } = &mut graph.types[3].kind {
                    variants[0].payload = Some(raw::NodeId(999));
                }
            }
        }
        assert!(matches!(
            verify(&graph, &sources, StorageTarget::Linear32V1),
            Err(Failure::Diagnostics(_))
        ));
        assert!(verify(&original, &sources, StorageTarget::Linear32V1).is_ok());
    }
}

#[test]
fn direct_by_value_cycle_rejects_while_nominal_vec_indirection_remains_finite() {
    let sources = sources();
    let mut graph = graph(&sources, fixture_kind("Box<i32>"));
    if let raw::TypeKind::Struct { fields, .. } = &mut graph.types[3].kind {
        fields[0].ty = raw::NodeId(3);
    }
    let Err(Failure::Diagnostics(errors)) = verify(&graph, &sources, StorageTarget::Linear32V1)
    else {
        panic!("layout must reject by-value cycle");
    };
    assert!(errors.iter().any(|error| error.code() == "ZRYNA-L3002"));
    graph.types.push(raw::TypeNode {
        id: raw::NodeId(4),
        span: None,
        kind: raw::TypeKind::Base(Base::Vec { element: raw::NodeId(3) }),
    });
    if let raw::TypeKind::Struct { fields, .. } = &mut graph.types[3].kind {
        fields[0].ty = raw::NodeId(4);
    }
    assert!(verify(&graph, &sources, StorageTarget::Linear32V1).is_ok());
}

#[test]
fn duplicate_program_roots_do_not_change_identity_or_consume_closed_type_budget() {
    let sources = sources();
    let mut graph = graph(&sources, fixture_kind("Option<i32>"));
    let target = StorageTarget::Linear32V1;
    let original = verify(&graph, &sources, target).expect("original graph");
    graph.program_roots = vec![raw::NodeId(3); 65537];
    let repeated = verify(&graph, &sources, target).expect("duplicate occurrences share one root");
    assert_eq!(original.universe_identity(), repeated.universe_identity());
    assert_eq!(original.canonical_bytes(), repeated.canonical_bytes());
}
