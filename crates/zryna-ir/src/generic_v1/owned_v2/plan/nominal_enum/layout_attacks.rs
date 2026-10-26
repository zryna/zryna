//! Nominal identity/resource and physically valid whole-member layout attacks.
use super::authorities::{Fixture, key};
use crate::generic_v1::{Failure, keys};
use zryna_layout::{StorageTarget, generic_v1 as layout};

#[test]
fn independent_nominal_sealed_members_keys_and_argument_order_are_complete() {
    let f = Fixture::new();
    for layouts in [&f.linear, &f.linux] {
        for (id, expected, arguments, payloads, affine) in [
            (
                f.choice(),
                key(0x13, 0, 0, &[&[2], &[1]]),
                vec![2, 1],
                vec![None, Some(2), Some(1), Some(f.nested())],
                true,
            ),
            (f.marker(), key(0x13, 0, 1, &[&[2]]), vec![2], vec![None, Some(1)], false),
            (f.fixed(), key(0x13, 0, 2, &[&[1]]), vec![1], vec![None, Some(2)], true),
            (f.plain(), key(0x11, 0, 3, &[]), vec![], vec![None, Some(2)], true),
        ] {
            let view = layouts.types().find(|ty| ty.id().index() == id).expect("nominal view");
            assert_eq!(view.key(), expected);
            assert_eq!(
                view.arguments().map(zryna_layout::generic_v1::TypeId::index).collect::<Vec<_>>(),
                arguments
            );
            assert_eq!(
                view.variants()
                    .map(|(_, p)| p.map(zryna_layout::generic_v1::TypeId::index))
                    .collect::<Vec<_>>(),
                payloads
            );
            assert_eq!(view.drop_kind() != 0, affine);
        }
    }
    assert_ne!(key(0x13, 0, 0, &[&[2], &[1]]), key(0x13, 0, 0, &[&[1], &[2]]));
    assert_ne!(key(0x11, 0, 3, &[]), key(0x13, 0, 2, &[&[1]]));
}

#[test]
fn raw_nominal_identity_arity_ordinals_payload_references_and_resource_attacks_reject() {
    let f = Fixture::new();
    for attack in 0..7 {
        let mut graph = f.graph.clone();
        match attack {
            0 => graph.declarations[0].parameters = 1,
            1 => graph.declarations[0].members = 3,
            2 => graph.declarations[0].kind = layout::raw::NominalKind::Struct,
            3 => {
                let layout::raw::TypeKind::Enum { variants, .. } = &mut graph.types[5].kind else {
                    panic!("choice")
                };
                variants[1].ordinal = 0;
            }
            4 => {
                let layout::raw::TypeKind::Enum { variants, .. } = &mut graph.types[5].kind else {
                    panic!("choice")
                };
                variants[2].payload = Some(layout::raw::NodeId(u32::MAX));
            }
            5 => graph.modules[0].data_declarations = 5,
            _ => graph.declarations[0].members = 1025,
        }
        for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
            assert!(
                layout::verify(&graph, &f.sources, target).is_err(),
                "raw nominal attack {attack}"
            );
            let pristine = layout::verify(&f.graph, &f.sources, target)
                .expect("pristine nominal layout recovery");
            let expected = if target == StorageTarget::Linear32V1 { &f.linear } else { &f.linux };
            assert_eq!(pristine.canonical_bytes(), expected.canonical_bytes());
        }
    }
}

#[test]
fn nested_nominal_complete_key_depth_keeps_unchanged_exact_limit_and_retry() {
    let mut bytes = vec![2];
    for _ in 0..64 {
        bytes = key(0x13, 0, 1, &[&bytes]);
    }
    let accepted = bytes.clone();
    assert_eq!(
        keys::decode(&accepted, keys::Domain::Type)
            .expect("exact nominal key depth")
            .application_depth(),
        64
    );
    for _ in 0..2 {
        let too_deep = key(0x13, 0, 1, &[&accepted]);
        let Failure::Diagnostics(errors) = keys::decode(&too_deep, keys::Domain::Type)
            .expect_err("first extra nominal application")
        else {
            panic!("diagnostic")
        };
        assert_eq!(errors[0].code, "ZRYNA-I3201");
        assert_eq!(
            keys::decode(&accepted, keys::Domain::Type)
                .expect("pristine complete key retry")
                .application_depth(),
            64
        );
    }
}
